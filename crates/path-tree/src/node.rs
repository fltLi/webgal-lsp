use std::fmt;

use derive_more::From;

use crate::Folder;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub enum Node<T> {
    Item(T),
    Folder(Folder<T>),
}

impl<T> Node<T> {
    /// 依据路径创建到节点的路径链, 并返回根
    ///
    /// # Behavior
    /// * 当路径为空原样返回节点.
    /// * 此函数假设路径规范 ([`crate::canonicalize`]), 若否将出现 `.`, `\` 等非法节点名.
    pub fn with_ancestors(path: &str, mut node: Self) -> Self {
        for name in path.rsplit('/') {
            node = Folder {
                children: vec![(name.to_string(), node)],
            }
            .into();
        }
        node
    }

    pub fn is_item(&self) -> bool {
        matches!(self, Self::Item(_))
    }

    pub fn as_item(&self) -> Option<&T> {
        match self {
            Self::Item(item) => Some(item),
            _ => None,
        }
    }

    pub fn as_item_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Item(item) => Some(item),
            _ => None,
        }
    }

    pub fn into_item(self) -> Option<T> {
        match self {
            Self::Item(item) => Some(item),
            _ => None,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, Self::Folder(_))
    }

    pub fn as_folder(&self) -> Option<&Folder<T>> {
        match self {
            Self::Folder(folder) => Some(folder),
            _ => None,
        }
    }

    pub fn as_folder_mut(&mut self) -> Option<&mut Folder<T>> {
        match self {
            Self::Folder(folder) => Some(folder),
            _ => None,
        }
    }

    pub fn into_folder(self) -> Option<Folder<T>> {
        match self {
            Self::Folder(folder) => Some(folder),
            _ => None,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Item(item) => item.fmt(f),
            Self::Folder(folder) => folder.fmt(f),
        }
    }
}
