//! 基于 [`BTreeMap`] 实现的计数器

use std::{
    collections::{BTreeMap, btree_map},
    fmt, iter,
};

/// 基于 [`BTreeMap`] 实现的计数器
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BTreeCounter<T: Ord>(BTreeMap<T, usize>);

impl<T: Ord> BTreeCounter<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 获取不重复的元素数量
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 获取所有元素的数量 (含重复数量)
    ///
    /// # Performance
    /// 遍历每个元素求和计数, 复杂度 `O(n\log n)`.
    pub fn len_count(&self) -> usize {
        self.0.values().sum()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// 查找元素是否存在
    ///
    /// # Performance
    /// 红黑树查找元素, 复杂度 `O(\log n)`.
    pub fn contains(&self, value: &T) -> bool {
        self.0.contains_key(value)
    }

    /// 获取元素计数
    ///
    /// # Performance
    /// 红黑树查找元素, 复杂度 `O(\log n)`.
    pub fn get(&self, value: &T) -> usize {
        self.0.get(value).cloned().unwrap_or(0)
    }

    /// 插入一个元素
    ///
    /// # Returns
    /// 插入后该元素的计数.
    ///
    /// # Performance
    /// 红黑树修改元素, 复杂度 `O(\log n)`.
    pub fn insert(&mut self, value: T) -> usize {
        self.insertn(value, 1)
    }

    /// 插入一定数量的元素
    ///
    /// # Returns
    /// 插入后该元素的计数.
    ///
    /// # Performance
    /// 红黑树修改元素, 复杂度 `O(\log n)`.
    pub fn insertn(&mut self, value: T, count: usize) -> usize {
        if count == 0 {
            return 0;
        }

        *self
            .0
            .entry(value)
            .and_modify(|v| *v += count)
            .or_insert(count)
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
    /// 红黑树修改元素, 复杂度 `O(\log n)`.
    pub fn insert_to(&mut self, value: T, count: usize) -> usize {
        if count == 0 {
            return self.remove_all(&value).unwrap_or(0);
        }

        self.0.insert(value, count).unwrap_or(0)
    }

    /// 删除一个元素
    ///
    /// # Returns
    /// 若元素存在, 返回删除后计数.
    ///
    /// # Performance
    /// 红黑树修改元素, 复杂度 `O(\log n)`.
    pub fn remove(&mut self, value: &T) -> Option<usize> {
        self.removen(value, 1)
    }

    /// 删除一定数量的元素
    ///
    /// # Returns
    /// 若元素存在, 返回删除后计数.
    ///
    /// # Performance
    /// 红黑树修改元素, 复杂度 `O(\log n)`.
    pub fn removen(&mut self, value: &T, count: usize) -> Option<usize> {
        let previous_count = self.0.get_mut(value)?;

        match previous_count.checked_sub(count) {
            Some(0) | None => {
                self.0.remove(value);
                Some(0)
            }

            Some(count) => {
                *previous_count -= count;
                Some(*previous_count)
            }
        }
    }

    /// 删除元素
    ///
    /// # Returns
    /// 若元素存在, 返回删除前计数.
    ///
    /// # Performance
    /// 红黑树删除元素, 复杂度 `O(\log n)`.
    pub fn remove_all(&mut self, value: &T) -> Option<usize> {
        self.0.remove(value)
    }

    /// 按照元素大小升序迭代不重复元素
    ///
    /// # Performance
    /// 遍历 [`BTreeMap`], 复杂度 `O(n\log n)`.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.keys()
    }

    /// 按照元素大小伴随计数升序迭代不重复元素
    ///
    /// # Performance
    /// 遍历 [`BTreeMap`], 复杂度 `O(n\log n)`.
    pub fn iter_with_count(&self) -> impl Iterator<Item = (&T, usize)> {
        self.0.iter().map(|(value, count)| (value, *count))
    }
}

impl<T: fmt::Debug + Ord> fmt::Debug for BTreeCounter<T> {
    /// 按元素大小升序调试输出元素及其计数
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter_with_count()).finish()
    }
}

impl<T: Ord> Default for BTreeCounter<T> {
    fn default() -> Self {
        Self(BTreeMap::default())
    }
}

impl<T: Ord> FromIterator<T> for BTreeCounter<T> {
    /// 依次插入元素构造计数器
    ///
    /// # Performance
    /// 红黑树插入元素, 总复杂度 `O(n\log n)`.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut counter = Self::new();
        counter.extend(iter);
        counter
    }
}

impl<T: Ord> FromIterator<(T, usize)> for BTreeCounter<T> {
    /// 依次插入元素构造计数器
    ///
    /// # Performance
    /// 红黑树插入元素, 总复杂度 `O(n\log n)`.
    fn from_iter<I: IntoIterator<Item = (T, usize)>>(iter: I) -> Self {
        let mut counter = Self::new();
        counter.extend(iter);
        counter
    }
}

impl<T: Ord> Extend<T> for BTreeCounter<T> {
    /// 逐一插入元素
    ///
    /// # Performance
    /// 红黑树插入元素, 总复杂度 `O(n\log n)`.
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().zip(iter::repeat(1)));
    }
}

impl<T: Ord> Extend<(T, usize)> for BTreeCounter<T> {
    /// 逐一插入一定数量的元素
    ///
    /// # Performance
    /// 红黑树插入元素, 总复杂度 `O(n\log n)`.
    fn extend<I: IntoIterator<Item = (T, usize)>>(&mut self, iter: I) {
        for (value, count) in iter {
            self.insertn(value, count);
        }
    }
}

impl<T: Ord> IntoIterator for BTreeCounter<T> {
    type Item = (T, usize);
    type IntoIter = btree_map::IntoIter<T, usize>;

    /// 按照元素大小升序取出存储的元素及其计数
    ///
    /// # Performance
    /// 遍历 [`BTreeMap`], 复杂度 `O(n\log n)`.
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    // -------- basic --------

    #[test]
    fn new_and_empty() {
        let c: BTreeCounter<i32> = BTreeCounter::new();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
        assert_eq!(c.len_count(), 0);
    }

    #[test]
    fn insert_and_modify() {
        let mut c = BTreeCounter::new();
        assert_eq!(c.insert("a"), 1);
        assert_eq!(c.get(&"a"), 1);
        assert_eq!(c.insertn("a", 3), 4);
        assert_eq!(c.get(&"a"), 4);
        assert_eq!(c.insert_to("a", 2), 4);
        assert_eq!(c.get(&"a"), 2);
        assert_eq!(c.remove(&"a"), Some(1));
        assert_eq!(c.get(&"a"), 1);
        assert_eq!(c.removen(&"a", 1), Some(0));
        assert!(!c.contains(&"a"));
        assert!(c.is_empty());
    }

    #[test]
    fn zero_insert_does_nothing() {
        let mut c: BTreeCounter<&str> = BTreeCounter::new();
        assert_eq!(c.insertn("x", 0), 0);
        assert!(c.is_empty());
        assert_eq!(c.insert_to("y", 0), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn remove_all_and_clear() {
        let mut c = BTreeCounter::new();
        c.insertn("a", 5);
        assert_eq!(c.remove_all(&"a"), Some(5));
        assert!(!c.contains(&"a"));
        c.insertn("b", 3);
        c.clear();
        assert!(c.is_empty());
    }

    // -------- iter --------

    #[test]
    fn iter_works() {
        let mut c = BTreeCounter::new();
        c.insertn("a", 3);
        c.insertn("b", 1);
        let values: Vec<_> = c.iter().collect();
        assert_eq!(values, vec![&"a", &"b"]);
    }

    #[test]
    fn iter_with_count() {
        let mut c = BTreeCounter::new();
        c.insertn("a", 3);
        c.insertn("b", 1);
        let pairs: Vec<_> = c.iter_with_count().collect();
        assert_eq!(pairs, vec![(&"a", 3), (&"b", 1)]);
    }

    #[test]
    fn into_iter_works() {
        let mut c = BTreeCounter::new();
        c.insertn("a", 3);
        c.insertn("b", 1);
        let pairs: Vec<_> = c.into_iter().collect();
        assert_eq!(pairs, vec![("a", 3), ("b", 1)]);
    }

    #[test]
    fn extend_and_from_iter() {
        let items = vec!["x", "y", "x", "z", "y", "x"];
        let c: BTreeCounter<&str> = items.into_iter().collect();
        assert_eq!(c.get(&"x"), 3);
        assert_eq!(c.get(&"y"), 2);
        assert_eq!(c.get(&"z"), 1);

        let mut c2 = BTreeCounter::new();
        c2.extend(vec![("a", 2), ("b", 1)]);
        assert_eq!(c2.get(&"a"), 2);
        assert_eq!(c2.get(&"b"), 1);
    }

    #[test]
    fn clone_and_eq() {
        let mut a = BTreeCounter::new();
        a.insertn("x", 3);
        let b = a.clone();
        assert_eq!(a, b);
        a.insert("y");
        assert_ne!(a, b);
    }
}
