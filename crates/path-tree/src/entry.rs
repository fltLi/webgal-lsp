use std::mem;

use Entry::*;

use crate::{Folder, Node, split_path_once};

#[derive(Debug)]
pub enum Entry<'a, 'p, T> {
    Occupied(OccupiedEntry<'a, T>),
    Vacant(VacantEntry<'a, 'p, T>),
}

#[derive(Debug)]
pub struct OccupiedEntry<'a, T> {
    pub(crate) folder: &'a mut Folder<T>,
    pub(crate) index: usize,
}

#[derive(Debug)]
pub struct VacantEntry<'a, 'p, T> {
    pub(crate) folder: &'a mut Folder<T>,
    pub(crate) path: &'p str,
    pub(crate) index: usize,
}

impl<'a, T> Entry<'a, '_, T> {
    pub fn or_insert(self, default: Node<T>) -> &'a mut Node<T> {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default),
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut Node<T>
    where
        F: FnOnce() -> Node<T>,
    {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default()),
        }
    }

    pub fn and_modify<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Node<T>),
    {
        if let Occupied(entry) = &mut self {
            f(entry.get_mut());
        }
        self
    }

    pub fn insert_entry(self, node: Node<T>) -> OccupiedEntry<'a, T> {
        match self {
            Occupied(mut entry) => {
                entry.insert(node);
                entry
            }
            Vacant(entry) => entry.insert_entry(node),
        }
    }
}

impl<'a, T> OccupiedEntry<'a, T> {
    pub fn get(&self) -> &Node<T> {
        &self.folder.children[self.index].1
    }

    pub fn get_mut(&mut self) -> &mut Node<T> {
        &mut self.folder.children[self.index].1
    }

    pub fn into_mut(self) -> &'a mut Node<T> {
        &mut self.folder.children[self.index].1
    }

    pub fn insert(&mut self, node: Node<T>) -> Node<T> {
        mem::replace(self.get_mut(), node)
    }

    pub fn remove(self) -> Node<T> {
        self.folder.children.remove(self.index).1
    }
}

impl<'a, 'p, T> VacantEntry<'a, 'p, T> {
    pub fn insert(self, node: Node<T>) -> &'a mut Node<T> {
        let Self {
            folder,
            path,
            index,
        } = self;
        let (name, node) = match split_path_once(path) {
            (name, Some(path)) => (name, Node::with_ancestors(path, node)),
            (_, None) => (path, node),
        };
        &mut folder
            .children
            .insert_mut(index, (name.to_string(), node))
            .1
    }

    pub fn insert_entry(self, node: Node<T>) -> OccupiedEntry<'a, T> {
        let Self {
            folder,
            path,
            index,
        } = self;
        let (name, node) = match split_path_once(path) {
            (name, Some(path)) => (name, Node::with_ancestors(path, node)),
            (_, None) => (path, node),
        };
        folder.children.insert(index, (name.to_string(), node));
        OccupiedEntry { folder, index }
    }
}
