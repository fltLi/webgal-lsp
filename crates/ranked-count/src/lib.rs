use std::{fmt, iter, vec};

/// 有序计数器
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
pub struct Counter<T: Ord> {
    values: Vec<T>,
    value_order: Vec<(usize, usize)>,
    count_order: Vec<(usize, usize)>,
}

impl<T: Ord> Counter<T> {
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
    pub fn len_count(&mut self) -> usize {
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
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.iter()
    }

    /// 按照元素大小升序迭代不重复元素
    pub fn iter_by_value(&self) -> impl Iterator<Item = (&T, usize)> {
        self.value_order
            .iter()
            .map(|&(index, count)| (&self.values[index], count))
    }

    /// 按照计数多少降序迭代不重复元素
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

impl<T: fmt::Debug + Ord> fmt::Debug for Counter<T> {
    /// 按计数多少降序调试输出元素及其计数
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter_by_count()).finish()
    }
}

impl<T: Ord> Default for Counter<T> {
    fn default() -> Self {
        Self {
            values: Vec::default(),
            value_order: Vec::default(),
            count_order: Vec::default(),
        }
    }
}

impl<T: Ord> FromIterator<T> for Counter<T> {
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

impl<T: Ord> FromIterator<(T, usize)> for Counter<T> {
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

impl<T: Ord> Extend<T> for Counter<T> {
    /// 逐一插入元素
    ///
    /// # Performance
    /// 先二分查找元素, 再二分修改计数序列, 单次插入复杂度介于 `O(\log n)` ~ `O(n)`.
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().zip(iter::repeat(1)));
    }
}

impl<T: Ord> Extend<(T, usize)> for Counter<T> {
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

impl<T: Ord> IntoIterator for Counter<T> {
    type Item = T;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<T: Ord> From<Counter<T>> for Vec<T> {
    fn from(value: Counter<T>) -> Self {
        value.into_vec()
    }
}
