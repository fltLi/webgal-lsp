use std::{iter, slice, vec};

use crate::{Folder, Node};

#[derive(Debug, Clone)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a, T> {
    #[allow(clippy::type_complexity)]
    inner: Vec<(&'a str, slice::Iter<'a, (String, Node<T>)>)>,
}

#[derive(Debug)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct IterMut<'a, T> {
    #[allow(clippy::type_complexity)]
    inner: Vec<(&'a str, slice::IterMut<'a, (String, Node<T>)>)>,
    pending: Option<(&'a str, *mut Node<T>)>,
}

impl<'a, T> Iter<'a, T> {
    /// 创建目录的递归迭代器 (不含根节点)
    pub fn new(folder: &'a Folder<T>) -> Self {
        Self {
            inner: vec![("", folder.children.iter())],
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (String, &'a Node<T>);

    /// 递归遍历下一个节点, 返回其到根的路径和节点借用
    fn next(&mut self) -> Option<Self::Item> {
        let (_, current) = self.inner.last_mut()?;

        match current.next() {
            Some((name, node)) => {
                let path = itertools::intersperse(
                    self.inner
                        .iter()
                        .skip(1)
                        .map(|(name, _)| *name)
                        .chain(iter::once(name.as_str())),
                    "/",
                )
                .collect();
                if let Node::Folder(folder) = node {
                    self.inner.push((name, folder.children.iter()));
                }
                Some((path, node))
            }

            None => {
                self.inner.pop();
                self.next()
            }
        }
    }
}

impl<'a, T> IterMut<'a, T> {
    /// 创建目录的递归迭代器 (不含根节点)
    pub fn new(folder: &'a mut Folder<T>) -> Self {
        Self {
            inner: vec![("", folder.children.iter_mut())],
            pending: None,
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (String, &'a mut Node<T>);

    /// 递归遍历下一个节点, 返回其到根的路径和节点借用
    fn next(&mut self) -> Option<Self::Item> {
        // 检查调用者使用后上一个节点是否为目录
        if let Some((name, node)) = self.pending.take()
            && let Node::Folder(folder) = unsafe { &mut *node }
        {
            self.inner.push((name, folder.children.iter_mut()));
        }

        let (_, current) = self.inner.last_mut()?;

        match current.next() {
            Some((name, node)) => {
                let path = itertools::intersperse(
                    self.inner
                        .iter()
                        .skip(1)
                        .map(|(name, _)| *name)
                        .chain(iter::once(name.as_str())),
                    "/",
                )
                .collect();
                self.pending = Some((name, node as *mut _));
                Some((path, node))
            }

            None => {
                self.inner.pop();
                self.next()
            }
        }
    }
}
