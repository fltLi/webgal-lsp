//! 工作区多项目管理与路由

use std::sync::Arc;

use anyhow::{Result, anyhow};
use derive_more::{From, Into};
use path_tree::{Entry, Folder, Node, join};
use tokio::sync::RwLock;

use crate::project::Project;

/// WebGAL 项目工作区管理
#[derive(Debug, Default)]
pub struct Workspace {
    projects: Folder<Arc<RwLock<Project>>>,
}

impl Workspace {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取资源所在项目
    ///
    /// # Arguments
    /// * **path** - 资源规范 URI.
    pub fn get(&self, path: &str) -> Option<GetProjectResult> {
        let (root, path) = split_root(path);

        let mut folder = self.projects.get(root)?.as_folder()?;
        let mut start = 0;

        while let Some(end) = path[start..].find('/') {
            let name = &path[start..end];
            start = end + 1;

            match folder.get(name) {
                Some(Node::Item(project)) => {
                    return Some(GetProjectResult {
                        project_path: path[..end].to_string(),
                        resource_path: path[start..].to_string(),
                        project: project.clone(),
                    });
                }

                Some(Node::Folder(next)) => {
                    folder = next;
                }

                _ => break,
            }
        }

        None
    }

    /// 插入项目
    ///
    /// # Arguments
    /// * **path** - 项目根目录规范 URI.
    pub fn insert(&mut self, path: &str, project: Project) -> Result<Arc<RwLock<Project>>> {
        let path = canonicalize(path);
        let entry = self
            .projects
            .entry(&path)
            .map_err(|_| anyhow!("项目 `{path}` 不能嵌套于其他项目下"))?;
        let project = Arc::new(RwLock::new(project));
        entry.insert_entry(Node::Item(project.clone()));
        Ok(project)
    }

    /// 删除项目
    ///
    /// # Arguments
    /// * **path** - 项目根目录规范 URI.
    pub fn remove(&mut self, path: &str) -> Option<Arc<RwLock<Project>>> {
        let path = canonicalize(path);
        let entry = self.projects.entry(&path).ok()?;
        if let Entry::Occupied(o) = entry
            && o.get().is_item()
        {
            Some(o.remove().into_item().unwrap())
        } else {
            None
        }
    }

    /// 移除根目录下的全部项目
    ///
    /// # Arguments
    /// * **root** - 目录规范 URI.
    pub fn remove_all_under(&mut self, root: &str) -> Option<Node<Arc<RwLock<Project>>>> {
        let root = canonicalize(root);
        self.projects.remove(&root)
    }
}

#[derive(Debug, Clone, Default, From, Into)]
pub struct GetProjectResult {
    /// 项目根目录的 URI
    pub project_path: String,
    /// 资源相对于项目根目录的路径
    pub resource_path: String,
    pub project: Arc<RwLock<Project>>,
}

fn split_root(path: &str) -> (&str, &str) {
    path.split_once("://").unwrap_or(("", path))
}

/// 整理规范 URI (形如 `root://...`) 为 `root/...`
fn canonicalize(path: &str) -> String {
    let (root, path) = split_root(path);
    join(root, path)
}
