use std::{fmt, iter::Peekable, mem, vec};

use crate::{
    Entry, Iter, IterMut, Node, OccupiedEntry, VacantEntry, ancestors_of, split_path_once,
};

/// 目录节点
///
/// 内部维护有序数组, 通过二分查找实现子节点查修操作.
///
/// # Performance
/// 本结构针对节点数量有限, 深度适中的路径树进行了优化.
/// 可保证:
/// * 子节点查询, 遍历 (含递归子树遍历) 具备极低常数和良好缓存局部性, 内存占用紧凑.
/// * 单层节点数量较少时, 二分查找开销极小, 不存在哈希表的内存浪费.
///
/// 插入与删除操作会引发元素移动, 因此更适合一次性构建且修改极少的使用模式.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Folder<T> {
    pub(crate) children: Vec<(String, Node<T>)>,
}

impl<T> Folder<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            children: Vec::with_capacity(capacity),
        }
    }

    /// 依据路径构造节点树
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 若否将出现 `.` 等非法节点名.
    /// * 节点路径不应重复, 重复则将去重且只保留第一个.
    ///
    /// # Performance
    /// 先按路径为节点排序, 接着维护到根的路径依次插入, 最坏 `O(n\log n + n)`.
    pub fn from_vec<P: AsRef<str>>(mut vec: Vec<(P, T)>) -> Self {
        // 为所有元素排序去重 - `O(n)` ~ `O(n\log n)`
        sort_children(&mut vec);
        vec.dedup_by(|(a, _), (b, _)| a.as_ref() == b.as_ref());

        let mut root = Self::new();
        let mut current_route = vec![("", &mut root as *mut _)];

        for (path, value) in vec {
            let mut path_split = ancestors_of(path.as_ref()).peekable();

            // 返回新旧路径 LCA - `O(n)`
            let diff_index = find_diff_index(
                current_route.iter().skip(1).map(|(name, _)| *name),
                &mut path_split,
            );
            current_route.truncate(diff_index + 1);

            // 创建新节点 - `O(n)`
            while let Some(name) = path_split.next() {
                let name = name.to_string();
                let parent: &mut Folder<T> = unsafe { &mut *current_route.last().unwrap().1 };

                if path_split.peek().is_some() {
                    // 插入目录
                    let name_view = unsafe { mem::transmute::<&str, &'static str>(name.as_str()) };
                    let (_, current) = parent.children.push_mut((name, Node::Folder(Self::new())));
                    let current_ptr =
                        unsafe { current.as_folder_mut().unwrap_unchecked() } as *mut _;
                    current_route.push((name_view, current_ptr));
                } else {
                    // 插入节点
                    parent.children.push((name, Node::Item(value)));
                    break;
                }
            }
        }

        root
    }

    // TODO: 实现 `append` 方法
    // /// 将一个目录全部节点移动到此目录
    // ///
    // /// # Performance
    // /// 递归走指针合并, 复杂度 `O(n)`.
    // pub fn append(&mut self, other: &mut Self) {
    //     unimplemented!()
    // }

    /// 当前目录是否为空
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// 获取当前目录节点数量
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// 递归统计当前目录所有子节点的数量
    pub fn len_recursively(&self) -> usize {
        self.len()
            + self
                .children
                .iter()
                .map(|(_, node)| match node {
                    Node::Item(_) => 0,
                    Node::Folder(folder) => folder.len_recursively(),
                })
                .sum::<usize>()
    }

    pub fn clear(&mut self) {
        self.children.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.children.reserve(additional);
    }

    pub fn reserve_recursively(&mut self, additional: usize) {
        self.children.reserve(additional);
        for (_, node) in &mut self.children {
            if let Node::Folder(folder) = node {
                folder.reserve_recursively(additional);
            }
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.children.shrink_to_fit();
    }

    pub fn shrink_to_fit_recursively(&mut self) {
        self.children.shrink_to_fit();
        for (_, node) in &mut self.children {
            if let Node::Folder(folder) = node {
                folder.shrink_to_fit_recursively();
            }
        }
    }

    /// 按路径查找节点是否存在
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 即不处理相对路径 `.` 跳转和不规范分隔符等情况.
    ///
    /// # Performance
    /// 将路径分段后逐层二分查找, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn contains(&self, path: &str) -> bool {
        self.get(path).is_some()
    }

    /// 按路径查找节点
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 即不处理相对路径 `.` 跳转和不规范分隔符等情况.
    ///
    /// # Performance
    /// 将路径分段后逐层二分查找, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn get(&self, path: &str) -> Option<&Node<T>> {
        let (name, path) = split_path_once(path);
        let index = self.binary_search_child(name).ok()?;
        let child = unsafe { &self.children.get_unchecked(index).1 };
        match path {
            Some(path) => child.as_folder().and_then(|folder| folder.get(path)),
            None => Some(child),
        }
    }

    /// 按路径查找节点
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 即不处理相对路径 `.` 跳转和不规范分隔符等情况.
    ///
    /// # Performance
    /// 将路径分段后逐层二分查找, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut Node<T>> {
        let (name, path) = split_path_once(path);
        let index = self.binary_search_child(name).ok()?;
        let child = unsafe { &mut self.children.get_unchecked_mut(index).1 };
        match path {
            Some(path) => child
                .as_folder_mut()
                .and_then(|folder| folder.get_mut(path)),
            None => Some(child),
        }
    }

    /// 插入新节点, 若节点存在则同时返回旧节点
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 若否将出现 `.`, `\` 等非法节点名.
    ///
    /// # Performance
    /// 将路径分段后逐层二分查找, 替换节点或插入子链, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn insert(&mut self, path: &str, node: Node<T>) -> Option<Node<T>> {
        let (name, path) = split_path_once(path);

        match self.binary_search_child(name) {
            Ok(index) => {
                let child = unsafe { &mut self.children.get_unchecked_mut(index).1 };
                match path {
                    Some(path) => child
                        .as_folder_mut()
                        .and_then(|folder| folder.insert(path, node)),
                    None => Some(mem::replace(child, node)),
                }
            }

            Err(index) => {
                // 创建并插入剩余节点
                let node = match path {
                    Some(path) => Node::with_ancestors(path, node),
                    None => node,
                };
                self.children.insert(index, (name.to_string(), node));
                None
            }
        }
    }

    /// 查找并弹出节点
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 若否将出现 `.`, `\` 等非法节点名.
    /// * 删除节点时, 若该层级目录变为空, 则一并移除目录节点. // TODO
    ///
    /// # Performance
    /// 将路径分段后逐层二分查找, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn remove(&mut self, path: &str) -> Option<Node<T>> {
        let (name, path) = split_path_once(path);
        let index = self.binary_search_child(name).ok()?;
        match path {
            Some(path) => unsafe {
                self.children
                    .get_unchecked_mut(index)
                    .1
                    .as_folder_mut()
                    .and_then(|folder| folder.remove(path).map(|node| (node, folder.is_empty())))
                    .map(|(node, empty)| {
                        if empty {
                            self.children.remove(index);
                        }
                        node
                    })
            },
            None => Some(self.children.remove(index).1),
        }
    }

    /// 获取指定路径节点访问器
    ///
    /// # Errors
    /// 路径上已存在资源节点时, 返回该节点路径及该其引用.
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 若否将出现 `.`, `\` 等非法节点名.
    ///
    /// # Performance
    /// 将路径分段后逐层二分查找, 不执行修改, 复杂度介于 `O(\log n)` ~ `O(n)`.
    pub fn entry<'a, 'p>(
        &'a mut self,
        path: &'p str,
    ) -> Result<Entry<'a, 'p, T>, (&'p str, &'a mut Node<T>)> {
        self.entry_inner(path)
            .map_err(|(last, node)| (&path[..path.len() - last.len()], node))
    }

    fn entry_inner<'a, 'p>(
        &'a mut self,
        path: &'p str,
    ) -> Result<Entry<'a, 'p, T>, (&'p str, &'a mut Node<T>)> {
        let full = path;
        let (name, path) = split_path_once(path);

        match self.binary_search_child(name) {
            Ok(index) => match path {
                Some(path) => match unsafe { &mut self.children.get_unchecked_mut(index).1 } {
                    Node::Folder(folder) => folder.entry(path),
                    node => Err((path, node)),
                },
                None => Ok(Entry::Occupied(OccupiedEntry {
                    folder: self,
                    index,
                })),
            },

            Err(index) => Ok(Entry::Vacant(VacantEntry {
                folder: self,
                path: full,
                index,
            })),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Node<T>)> {
        self.children
            .iter()
            .map(|(name, node)| (name.as_str(), node))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut Node<T>)> {
        self.children
            .iter_mut()
            .map(|(name, node)| (name.as_str(), node))
    }

    /// 获取目录的递归迭代器 (不含根节点)
    pub fn iter_recursively(&self) -> Iter<'_, T> {
        Iter::new(self)
    }

    /// 获取目录的递归迭代器 (不含根节点)
    pub fn iter_mut_recursively(&mut self) -> IterMut<'_, T> {
        IterMut::new(self)
    }

    pub fn into_vec(self) -> Vec<(String, Node<T>)> {
        self.children
    }

    fn binary_search_child(&self, name: &str) -> Result<usize, usize> {
        self.children
            .binary_search_by_key(&name, |(probe, _)| probe)
    }
}

impl<T: fmt::Debug> fmt::Debug for Folder<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<T> Default for Folder<T> {
    fn default() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl<P: AsRef<str>, T> FromIterator<(P, T)> for Folder<T> {
    /// 依据路径构造节点树
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 若否将出现 `.`, `\` 等非法节点名.
    /// * 节点路径不应重复, 重复则将去重且只保留第一个.
    ///
    /// # Performance
    /// 先按收集路径并为节点排序, 接着维护到根的路径依次插入, 最坏 `O(n\log n + n)`.
    fn from_iter<I: IntoIterator<Item = (P, T)>>(iter: I) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

impl<P: AsRef<str>, T> Extend<(P, T)> for Folder<T> {
    /// 逐一插入新节点
    ///
    /// # Behavior
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 若否将出现 `.`, `\` 等非法节点名.
    ///
    /// # Performance
    /// 将路径分段后逐层二分查找, 替换节点或插入子链, 单次插入复杂度介于 `O(\log n)` ~ `O(n)`.
    fn extend<I: IntoIterator<Item = (P, T)>>(&mut self, iter: I) {
        for (path, value) in iter {
            self.insert(path.as_ref(), Node::Item(value));
        }
    }
}

impl<T> IntoIterator for Folder<T> {
    type Item = (String, Node<T>);
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.children.into_iter()
    }
}

impl<P: AsRef<str>, T> From<Vec<(P, T)>> for Folder<T> {
    fn from(value: Vec<(P, T)>) -> Self {
        Folder::from_vec(value)
    }
}

impl<T> From<Folder<T>> for Vec<(String, Node<T>)> {
    fn from(value: Folder<T>) -> Self {
        value.into_vec()
    }
}

fn sort_children<P: AsRef<str>, T>(children: &mut [(P, T)]) {
    children.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
}

fn find_diff_index<T, I1, I2>(iter1: I1, iter2: &mut Peekable<I2>) -> usize
where
    T: PartialEq,
    I1: Iterator<Item = T>,
    I2: Iterator<Item = T>,
{
    let mut i = 0;
    for v in iter1 {
        if let Some(it) = iter2.peek()
            && v == *it
        {
            iter2.next();
            i += 1;
        } else {
            return i;
        }
    }
    i
}
