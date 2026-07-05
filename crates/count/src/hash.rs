//! 基于 [`HashMap`] 实现的计数器

use std::{
    collections::{HashMap, hash_map},
    fmt,
    hash::Hash,
    iter,
};

/// 基于 [`HashMap`] 实现的计数器
#[derive(Clone, Eq)]
pub struct HashCounter<T: Eq + Hash>(HashMap<T, usize>);

impl<T: Eq + Hash> HashCounter<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
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
    /// 遍历每个元素求和计数, 复杂度 `O(n)`.
    pub fn len_count(&self) -> usize {
        self.0.values().sum()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// 查找元素是否存在
    ///
    /// # Performance
    /// 哈希查找元素, 复杂度 `O(1)`.
    pub fn contains(&self, value: &T) -> bool {
        self.0.contains_key(value)
    }

    /// 获取元素计数
    ///
    /// # Performance
    /// 哈希查找元素, 复杂度 `O(1)`.
    pub fn get(&self, value: &T) -> usize {
        self.0.get(value).cloned().unwrap_or(0)
    }

    /// 插入一个元素
    ///
    /// # Returns
    /// 插入后该元素的计数.
    ///
    /// # Performance
    /// 哈希修改元素, 复杂度 `O(1)`.
    pub fn insert(&mut self, value: T) -> usize {
        self.insertn(value, 1)
    }

    /// 插入一定数量的元素
    ///
    /// # Returns
    /// 插入后该元素的计数.
    ///
    /// # Performance
    /// 哈希修改元素, 复杂度 `O(1)`.
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
    /// 哈希修改元素, 复杂度 `O(1)`.
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
    /// 哈希修改元素, 复杂度 `O(1)`.
    pub fn remove(&mut self, value: &T) -> Option<usize> {
        self.removen(value, 1)
    }

    /// 删除一定数量的元素
    ///
    /// # Returns
    /// 若元素存在, 返回删除后计数.
    ///
    /// # Performance
    /// 哈希修改元素, 复杂度 `O(1)`.
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
    /// 哈希删除元素, 复杂度 `O(1)`.
    pub fn remove_all(&mut self, value: &T) -> Option<usize> {
        self.0.remove(value)
    }

    /// 无序迭代不重复元素
    ///
    /// # Performance
    /// 遍历 [`HashMap`], 复杂度 `O(n)`.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.keys()
    }

    /// 伴随计数无序迭代不重复元素
    ///
    /// # Performance
    /// 遍历 [`HashMap`], 复杂度 `O(n)`.
    pub fn iter_with_count(&self) -> impl Iterator<Item = (&T, usize)> {
        self.0.iter().map(|(value, count)| (value, *count))
    }
}

impl<T: fmt::Debug + Eq + Hash> fmt::Debug for HashCounter<T> {
    /// 无序调试输出元素及其计数
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter_with_count()).finish()
    }
}

impl<T: Eq + Hash> Default for HashCounter<T> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

impl<T: Eq + Hash> PartialEq for HashCounter<T> {
    /// 遍历比较计数器是否相等
    ///
    /// # Performance
    /// 遍历 [`HashMap`], 复杂度 `O(n)`.
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .all(|(value, count)| other.0.get(value) == Some(count))
    }
}

impl<T: Eq + Hash> FromIterator<T> for HashCounter<T> {
    /// 依次插入元素构造计数器
    ///
    /// # Performance
    /// 哈希插入元素, 总复杂度 `O(n)`.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut counter = Self::new();
        counter.extend(iter);
        counter
    }
}

impl<T: Eq + Hash> FromIterator<(T, usize)> for HashCounter<T> {
    /// 依次插入元素构造计数器
    ///
    /// # Performance
    /// 哈希插入元素, 总复杂度 `O(n)`.
    fn from_iter<I: IntoIterator<Item = (T, usize)>>(iter: I) -> Self {
        let mut counter = Self::new();
        counter.extend(iter);
        counter
    }
}

impl<T: Eq + Hash> Extend<T> for HashCounter<T> {
    /// 逐一插入元素
    ///
    /// # Performance
    /// 哈希插入元素, 总复杂度 `O(n)`.
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().zip(iter::repeat(1)));
    }
}

impl<T: Eq + Hash> Extend<(T, usize)> for HashCounter<T> {
    /// 逐一插入一定数量的元素
    ///
    /// # Performance
    /// 哈希插入元素, 总复杂度 `O(n)`.
    fn extend<I: IntoIterator<Item = (T, usize)>>(&mut self, iter: I) {
        for (value, count) in iter {
            self.insertn(value, count);
        }
    }
}

impl<T: Eq + Hash> IntoIterator for HashCounter<T> {
    type Item = (T, usize);
    type IntoIter = hash_map::IntoIter<T, usize>;

    /// 无序取出存储的元素及其计数
    ///
    /// # Performance
    /// 遍历 [`HashMap`], 复杂度 `O(n)`.
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
        let c: HashCounter<i32> = HashCounter::new();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
        assert_eq!(c.len_count(), 0);
    }

    #[test]
    fn insert_and_modify() {
        let mut c = HashCounter::new();
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
        let mut c: HashCounter<&str> = HashCounter::new();
        assert_eq!(c.insertn("x", 0), 0);
        assert!(c.is_empty());
        assert_eq!(c.insert_to("y", 0), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn remove_all_and_clear() {
        let mut c = HashCounter::new();
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
        let mut c = HashCounter::new();
        c.insertn("a", 3);
        c.insertn("b", 1);
        let values: Vec<_> = c.iter().collect();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&&"a"));
        assert!(values.contains(&&"b"));
    }

    #[test]
    fn iter_with_count() {
        let mut c = HashCounter::new();
        c.insertn("a", 3);
        c.insertn("b", 1);
        let mut pairs: Vec<_> = c.iter_with_count().collect();
        pairs.sort_by_key(|(k, _)| *k);
        assert_eq!(pairs, vec![(&"a", 3), (&"b", 1)]);
    }

    #[test]
    fn into_iter_works() {
        let mut c = HashCounter::new();
        c.insertn("a", 3);
        c.insertn("b", 1);
        let mut pairs: Vec<_> = c.into_iter().collect();
        pairs.sort_by_key(|(k, _)| *k);
        assert_eq!(pairs, vec![("a", 3), ("b", 1)]);
    }

    #[test]
    fn extend_and_from_iter() {
        let items = vec!["x", "y", "x", "z", "y", "x"];
        let c: HashCounter<&str> = items.into_iter().collect();
        assert_eq!(c.get(&"x"), 3);
        assert_eq!(c.get(&"y"), 2);
        assert_eq!(c.get(&"z"), 1);

        let mut c2 = HashCounter::new();
        c2.extend(vec![("a", 2), ("b", 1)]);
        assert_eq!(c2.get(&"a"), 2);
        assert_eq!(c2.get(&"b"), 1);
    }

    #[test]
    fn clone_and_eq() {
        let mut a = HashCounter::new();
        a.insertn("x", 3);
        let b = a.clone();
        assert_eq!(a, b);
        a.insert("y");
        assert_ne!(a, b);
    }
}
