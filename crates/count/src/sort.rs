//! 基于有序列表实现的计数器

use std::{
    fmt,
    iter::{self, Zip},
    vec,
};

/// 基于有序列表实现的计数器
///
/// 内部维护三个数组:
/// * `values` - 元素数组, 存放元素, 使用下标访问.
/// * `value_order` - 元素大小序数组, 直接存储并提供对元素计数的二分查找访问.
/// * `count_order` - 计数多少序数组, 额外二分查修维护计数顺序.
///
/// # Performance
/// 本结构针对不重复元素有限的计数进行了优化.
/// 内存连续, 算法简单常数小, 查修复杂度介于 `O(\log n) ~ O(n)`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SortedCounter<T: Ord> {
    values: Vec<T>,
    value_order: Vec<(usize, usize)>,
    count_order: Vec<(usize, usize)>,
}

impl<T: Ord> SortedCounter<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            value_order: Vec::with_capacity(capacity),
            count_order: Vec::with_capacity(capacity),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// 获取不重复的元素数量
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// 获取所有元素的数量 (含重复数量)
    ///
    /// # Performance
    /// 遍历每个元素求和计数, 复杂度 `O(n)`.
    pub fn len_count(&self) -> usize {
        self.value_order.iter().map(|(_, count)| count).sum()
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.value_order.clear();
        self.count_order.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.values.reserve(additional);
        self.value_order.reserve(additional);
        self.count_order.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.values.shrink_to_fit();
        self.value_order.shrink_to_fit();
        self.count_order.shrink_to_fit();
    }

    /// 查找元素是否存在
    ///
    /// # Performance
    /// 二分查找元素, 复杂度 `O(\log n)`.
    pub fn contains(&self, value: &T) -> bool {
        self.binary_search_value(value).is_ok()
    }

    /// 获取元素计数
    ///
    /// # Performance
    /// 二分查找元素, 复杂度 `O(\log n)`.
    pub fn get(&self, value: &T) -> usize {
        match self.binary_search_value(value) {
            Ok(index) => self.value_order[index].1,
            Err(_) => 0,
        }
    }

    /// 插入一个元素
    ///
    /// # Returns
    /// 插入后该元素的计数.
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn insert(&mut self, value: T) -> usize {
        self.insertn(value, 1)
    }

    /// 插入一定数量的元素
    ///
    /// # Returns
    /// 插入后该元素的计数.
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn insertn(&mut self, value: T, count: usize) -> usize {
        match self.binary_search_value(&value) {
            Ok(index) => {
                let previous = &mut self.value_order[index];
                let (value_index, previous_count) = *previous;
                let count = previous_count + count;
                if count != previous_count {
                    previous.1 = count;
                    self.update_count(value_index, previous_count, count);
                }
                count
            }

            Err(_) if count == 0 => 0,

            Err(index) => {
                self.insert_value(value, index, count);
                count
            }
        }
    }

    /// 设置指定元素计数
    ///
    /// # Returns
    /// 插入前该元素的计数.
    ///
    /// # Behavior
    /// * `count` 为 0 时删除该元素.
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn insert_to(&mut self, value: T, count: usize) -> usize {
        if count == 0 {
            return self.remove_all(&value).unwrap_or(0);
        }

        match self.binary_search_value(&value) {
            Ok(index) => {
                let previous = &mut self.value_order[index];
                let (value_index, previous_count) = *previous;
                if count != previous_count {
                    previous.1 = count;
                    self.update_count(value_index, previous_count, count);
                }
                count
            }

            Err(index) => {
                self.insert_value(value, index, count);
                0
            }
        }
    }

    /// 删除一个元素
    ///
    /// # Returns
    /// 若元素存在, 返回删除后计数.
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn remove(&mut self, value: &T) -> Option<usize> {
        self.removen(value, 1)
    }

    /// 删除一定数量的元素
    ///
    /// # Returns
    /// 若元素存在, 返回删除后计数.
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn removen(&mut self, value: &T, count: usize) -> Option<usize> {
        let index = self.binary_search_value(value).ok()?;
        let (value_index, previous_count) = self.value_order[index];

        match previous_count.checked_sub(count) {
            Some(0) | None => {
                self.remove_value(index);
                Some(0)
            }

            Some(count) => {
                self.value_order[index].1 = count;
                self.update_count(value_index, previous_count, count);
                Some(count)
            }
        }
    }

    /// 删除元素
    ///
    /// # Returns
    /// 若元素存在, 返回删除前计数.
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列并删除元素, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn remove_all(&mut self, value: &T) -> Option<usize> {
        let index = self.binary_search_value(value).ok()?;
        Some(self.remove_value(index))
    }

    /// 无序迭代不重复元素
    ///
    /// # Performance
    /// 直接遍历有序索引列表, 复杂度 `O(n)`.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.iter()
    }

    /// 按照元素大小升序迭代不重复元素
    ///
    /// # Performance
    /// 直接遍历有序索引列表, 复杂度 `O(n)`.
    pub fn iter_by_value(&self) -> impl Iterator<Item = (&T, usize)> {
        self.value_order
            .iter()
            .map(|&(index, count)| (&self.values[index], count))
    }

    /// 按照计数多少降序迭代不重复元素
    ///
    /// # Performance
    /// 直接遍历有序索引列表, 复杂度 `O(n)`.
    pub fn iter_by_count(&self) -> impl Iterator<Item = (&T, usize)> {
        self.count_order
            .iter()
            .map(|&(index, count)| (&self.values[index], count))
    }

    pub fn into_vec(self) -> Vec<T> {
        self.values
    }

    fn binary_search_value(&self, value: &T) -> Result<usize, usize> {
        self.value_order
            .binary_search_by_key(&value, |&(index, _)| &self.values[index])
    }

    fn binary_search_count(&self, value: &T, count: usize) -> Result<usize, usize> {
        self.count_order.binary_search_by(|&(index, count_copy)| {
            count_copy
                .cmp(&count)
                .reverse()
                .then_with(|| self.values[index].cmp(value))
        })
    }

    fn insert_count(&mut self, value_index: usize, count: usize) {
        let value = &self.values[value_index];
        let index = self.binary_search_count(value, count).unwrap_err();
        self.count_order.insert(index, (value_index, count));
    }

    fn update_count(&mut self, value_index: usize, previous_count: usize, current_count: usize) {
        let value = &self.values[value_index];
        let index = self.binary_search_count(value, previous_count).unwrap();
        self.count_order.remove(index);
        self.insert_count(value_index, current_count);
    }

    fn insert_value(&mut self, value: T, index: usize, count: usize) {
        let value_index = self.values.len();
        self.values.push(value);
        self.value_order.insert(index, (value_index, count));
        self.insert_count(value_index, count);
    }

    fn remove_value(&mut self, index: usize) -> usize {
        let (value_index, count) = self.value_order.remove(index);
        let value = &self.values[value_index];

        let index = self.binary_search_count(value, count).unwrap();
        self.count_order.remove(index);

        // 修改受 values.swap_remove 影响的 value_index
        let swapping_value_index = self.values.len() - 1;
        if swapping_value_index != value_index {
            let value = &self.values[swapping_value_index];

            let index = self.binary_search_value(value).unwrap();
            let swapping = &mut self.value_order[index];
            swapping.0 = value_index;
            let count = swapping.1;

            let index = self.binary_search_count(value, count).unwrap();
            self.count_order[index].0 = value_index;
        }
        self.values.swap_remove(value_index);

        count
    }
}

impl<T: fmt::Debug + Ord> fmt::Debug for SortedCounter<T> {
    /// 按计数多少降序调试输出元素及其计数
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter_by_count()).finish()
    }
}

impl<T: Ord> Default for SortedCounter<T> {
    fn default() -> Self {
        Self {
            values: Vec::default(),
            value_order: Vec::default(),
            count_order: Vec::default(),
        }
    }
}

impl<T: Ord> FromIterator<T> for SortedCounter<T> {
    /// 依次插入元素构造计数器
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 单次插入复杂度介于 `O(\log n)` ~ `O(n)`.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut counter = Self::new();
        counter.extend(iter);
        counter
    }
}

impl<T: Ord> FromIterator<(T, usize)> for SortedCounter<T> {
    /// 依次插入一定数量的元素构造计数器
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 单次插入复杂度介于 `O(\log n)` ~ `O(n)`.
    fn from_iter<I: IntoIterator<Item = (T, usize)>>(iter: I) -> Self {
        let mut counter = Self::new();
        counter.extend(iter);
        counter
    }
}

impl<T: Ord> Extend<T> for SortedCounter<T> {
    /// 逐一插入元素
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 单次插入复杂度介于 `O(\log n)` ~ `O(n)`.
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().zip(iter::repeat(1)));
    }
}

impl<T: Ord> Extend<(T, usize)> for SortedCounter<T> {
    /// 逐一插入一定数量的元素
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 单次插入复杂度介于 `O(\log n)` ~ `O(n)`.
    fn extend<I: IntoIterator<Item = (T, usize)>>(&mut self, iter: I) {
        for (value, count) in iter {
            self.insertn(value, count);
        }
    }
}

impl<T: Ord> IntoIterator for SortedCounter<T> {
    type Item = (T, usize);
    type IntoIter = Zip<vec::IntoIter<T>, vec::IntoIter<usize>>;

    /// 无序取出存储的元素及其计数
    ///
    /// # Performance
    /// 先整理计数再遍历, 复杂度 `O(n)`.
    fn into_iter(self) -> Self::IntoIter {
        let mut counts = vec![0; self.len()];
        for (index, count) in self.count_order {
            counts[index] = count;
        }

        self.values.into_iter().zip(counts)
    }
}

impl<T: Ord> From<SortedCounter<T>> for Vec<T> {
    fn from(value: SortedCounter<T>) -> Self {
        value.into_vec()
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use std::collections::BTreeMap;

    use rand::{RngExt, SeedableRng, rngs::StdRng};

    use super::*;

    // -------- basic --------

    #[test]
    fn new_and_empty() {
        let c: SortedCounter<i32> = SortedCounter::new();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
        let c2: SortedCounter<i32> = SortedCounter::with_capacity(10);
        assert!(c2.is_empty());
    }

    #[test]
    fn insert_new_element() {
        let mut c = SortedCounter::new();
        assert_eq!(c.insert("a"), 1);
        assert_eq!(c.get(&"a"), 1);
        assert_eq!(c.len(), 1);
        assert_eq!(c.len_count(), 1);
    }

    #[test]
    fn insertn_new_element() {
        let mut c = SortedCounter::new();
        assert_eq!(c.insertn("b", 5), 5);
        assert_eq!(c.get(&"b"), 5);
    }

    #[test]
    fn insert_to_new_element() {
        let mut c = SortedCounter::new();
        assert_eq!(c.insert_to("c", 3), 0);
        assert_eq!(c.get(&"c"), 3);
    }

    #[test]
    fn modify_existing_by_insert() {
        let mut c = SortedCounter::new();
        c.insert("a");
        assert_eq!(c.insert("a"), 2);
        assert_eq!(c.get(&"a"), 2);
    }

    #[test]
    fn modify_existing_by_insertn() {
        let mut c = SortedCounter::new();
        c.insertn("a", 2);
        assert_eq!(c.insertn("a", 3), 5);
        assert_eq!(c.get(&"a"), 5);
    }

    #[test]
    fn modify_existing_by_insert_to() {
        let mut c = SortedCounter::new();
        c.insert_to("a", 5);
        assert_eq!(c.insert_to("a", 2), 2);
        assert_eq!(c.get(&"a"), 2);
    }

    #[test]
    fn modify_existing_by_remove() {
        let mut c = SortedCounter::new();
        c.insertn("a", 5);
        assert_eq!(c.remove(&"a"), Some(4));
        assert_eq!(c.get(&"a"), 4);
    }

    #[test]
    fn modify_existing_by_removen() {
        let mut c = SortedCounter::new();
        c.insertn("a", 5);
        assert_eq!(c.removen(&"a", 2), Some(3));
        assert_eq!(c.get(&"a"), 3);
    }

    #[test]
    fn remove_until_zero_deletes_element() {
        let mut c = SortedCounter::new();
        c.insert("a");
        assert_eq!(c.remove(&"a"), Some(0));
        assert!(!c.contains(&"a"));
        assert_eq!(c.len(), 0);

        // removen from 1 to 0
        let mut c2 = SortedCounter::new();
        c2.insertn("b", 1);
        assert_eq!(c2.removen(&"b", 1), Some(0));
        assert!(!c2.contains(&"b"));
    }

    #[test]
    fn remove_all_deletes() {
        let mut c = SortedCounter::new();
        c.insertn("a", 3);
        assert_eq!(c.remove_all(&"a"), Some(3));
        assert!(!c.contains(&"a"));
        assert_eq!(c.remove_all(&"missing"), None);
    }

    #[test]
    fn insert_to_zero_deletes() {
        let mut c = SortedCounter::new();
        c.insert_to("a", 5);
        assert_eq!(c.insert_to("a", 0), 5);
        assert!(!c.contains(&"a"));
    }

    #[test]
    fn insertn_zero_does_nothing() {
        let mut c: SortedCounter<&str> = SortedCounter::new();
        assert_eq!(c.insertn("x", 0), 0);
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn clear_reserve_shrink() {
        let mut c = SortedCounter::with_capacity(32);
        c.reserve(100);
        for i in 0..50 {
            c.insert(i);
        }
        assert_eq!(c.len(), 50);
        c.clear();
        assert!(c.is_empty());
        c.shrink_to_fit();
    }

    #[test]
    fn into_vec_and_from_vec() {
        let mut c = SortedCounter::new();
        c.insert("one");
        c.insert("two");
        let v: Vec<&str> = Vec::from(c.clone());
        assert_eq!(v.len(), 2);
        let c2: SortedCounter<&str> = v.into_iter().collect();
        assert_eq!(c2.get(&"one"), 1);
        assert_eq!(c2.get(&"two"), 1);
    }

    #[test]
    fn clone_and_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        fn hash<T: Hash>(t: &T) -> u64 {
            let mut s = DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        }
        let mut a = SortedCounter::new();
        a.insertn("x", 3);
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(hash(&a), hash(&b));
        a.insert("y");
        assert_ne!(a, b);
    }

    #[test]
    fn debug_output() {
        let mut c = SortedCounter::new();
        c.insertn("alpha", 2);
        let s = format!("{:?}", c);
        assert!(s.contains("alpha"));
    }

    // -------- iter --------

    #[test]
    fn iter_by_value_and_count_ordering() {
        let mut c = SortedCounter::new();
        c.insert_to("a", 5);
        c.insert_to("b", 2);
        c.insert_to("c", 2);
        assert_eq!(
            c.iter_by_count().map(|(v, _)| *v).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            c.iter_by_value().map(|(v, _)| *v).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn iter_consistency() {
        let mut c = SortedCounter::new();
        for i in 0..10 {
            c.insertn(i, (i % 3) + 1);
        }
        let first: Vec<_> = c.iter_by_value().collect();
        let second: Vec<_> = c.iter_by_value().collect();
        assert_eq!(first, second);
    }

    #[test]
    fn extend_and_from_iter() {
        let items = vec!["x", "y", "x", "z", "y", "x"];
        let c: SortedCounter<&str> = items.into_iter().collect();
        assert_eq!(c.get(&"x"), 3);
        assert_eq!(c.get(&"y"), 2);
        assert_eq!(c.get(&"z"), 1);

        let mut c2: SortedCounter<&str> = SortedCounter::new();
        c2.extend(vec![("a", 2), ("b", 1)]);
        assert_eq!(c2.get(&"a"), 2);
        assert_eq!(c2.get(&"b"), 1);
    }

    #[test]
    fn into_iter_works() {
        let mut c = SortedCounter::new();
        c.insertn("a", 3);
        c.insertn("b", 1);
        let mut pairs: Vec<_> = c.into_iter().collect();
        pairs.sort_by_key(|&(k, _)| k);
        assert_eq!(pairs, vec![("a", 3), ("b", 1)]);
    }

    // -------- random --------

    fn counter_to_map(c: &SortedCounter<u32>) -> BTreeMap<u32, usize> {
        c.iter_by_value().map(|(v, count)| (*v, count)).collect()
    }

    #[test]
    fn randomized_operations_match_btreemap() {
        let mut rng = StdRng::seed_from_u64(0x1234_5678_9ABC_DEF0);
        let key_space = 200u32;
        let ops = 2_000usize;

        let mut c = SortedCounter::new();
        let mut model: BTreeMap<u32, usize> = BTreeMap::new();

        for step in 0..ops {
            let op: u8 = rng.random_range(0..6);
            let key = rng.random_range(0..key_space);
            match op {
                0 => {
                    // insert single
                    c.insert(key);
                    *model.entry(key).or_insert(0) += 1;
                }
                1 => {
                    // insertn 1..5
                    let n = rng.random_range(1..6) as usize;
                    c.insertn(key, n);
                    *model.entry(key).or_insert(0) += n;
                }
                2 => {
                    // insert_to set count 0..5
                    let n = rng.random_range(0..6) as usize;
                    if n == 0 {
                        c.insert_to(key, 0);
                        model.remove(&key);
                    } else {
                        c.insert_to(key, n);
                        model.insert(key, n);
                    }
                }
                3 => {
                    // removen 1..5
                    let n = rng.random_range(1..6) as usize;
                    let prev = model.get(&key).cloned();
                    if let Some(prev_count) = prev {
                        if prev_count > n {
                            model.insert(key, prev_count - n);
                        } else {
                            model.remove(&key);
                        }
                    }
                    let _ = c.removen(&key, n);
                }
                4 => {
                    // remove_all
                    model.remove(&key);
                    let _ = c.remove_all(&key);
                }
                _ => {
                    // get / contains - no structural change
                    let _ = c.get(&key);
                    let _ = c.contains(&key);
                }
            }

            // validate model vs counter
            let from_counter = counter_to_map(&c);
            if from_counter.len() != model.len() {
                let extra_in_counter: Vec<u32> = from_counter
                    .keys()
                    .filter(|k| !model.contains_key(k))
                    .cloned()
                    .collect();
                let extra_in_model: Vec<u32> = model
                    .keys()
                    .filter(|k| !from_counter.contains_key(k))
                    .cloned()
                    .collect();
                eprintln!(
                    "Mismatch at step {}: extra_in_counter={:?}, extra_in_model={:?}",
                    step, extra_in_counter, extra_in_model
                );
                for k in extra_in_counter.iter() {
                    eprintln!("counter[{:?}] = {:?}", k, from_counter.get(k));
                }
                for k in extra_in_model.iter() {
                    eprintln!("model[{:?}] = {:?}", k, model.get(k));
                }
            }
            assert_eq!(from_counter.len(), model.len());
            for (k, v) in model.iter() {
                assert_eq!(from_counter.get(k).cloned().unwrap_or(0), *v);
            }
        }
    }
}
