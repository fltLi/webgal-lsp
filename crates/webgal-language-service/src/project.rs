use std::result;

use getset::Getters;
use path_tree::{Entry, Folder, Node, canonicalize};
use webgal_language_core::{
    resource::{Config, ResourceInfo, ResourceKind},
    sentence::Scene,
};

pub use error::*;
pub use ident::*;
pub use resource::*;

mod error;
mod ident;
mod resource;

/// WebGAL 项目信息
#[derive(Debug, Default, Getters)]
pub struct Project {
    #[getset(get = "pub")]
    config: Config,
    #[getset(get = "pub")]
    resource: Resource,
    #[getset(get = "pub")]
    ident: IdentTable,
}

impl Project {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            resource: Resource::new(),
            ident: IdentTable::new(),
        }
    }

    /// 插入 / 修改项目文件
    ///
    /// # Arguments
    /// * **path** - 文件相对于项目根目录的路径.
    /// * **f** - 获取文件内容 (字符串).
    ///
    /// # Behavior
    /// * 按需获取文件内容 (属于资源类型 + 该类型需要解析文件内容 + 路径正确).
    pub fn insert<F>(&mut self, path: &str, f: F) -> Result<()>
    where
        F: FnOnce() -> anyhow::Result<String>,
    {
        let path = try_canonicalize(path)?;
        let info = ResourceInfo::from_path(&path);

        if !info.is_relevant_file() {
            return Ok(());
        }

        let make_error = |detail| Error {
            path: path.clone(),
            kind: info.kind,
            detail,
        };
        let ResourceInfo { kind, path, .. } = info;

        match kind {
            ResourceKind::Config => {
                let content = f().map_err(ErrorKind::Content).map_err(make_error)?;
                self.config = Config::from_str(&content);
            }

            ResourceKind::Scene => {
                // 定位场景
                let entry = try_entry_of(path, &mut self.resource.scene).map_err(make_error)?;

                // 解析场景
                let content = f().map_err(ErrorKind::Content).map_err(make_error)?;
                let scene = Scene::from_str(content);

                // 更新符号
                self.ident.insert_scene(&scene);
                if let Entry::Occupied(o) = &entry {
                    self.ident.remove_scene(
                        o.get()
                            .as_item()
                            .expect("场景条目已在 [`try_entry_of`] 校验"),
                    );
                }

                // 插入场景
                entry.insert_entry(Node::Item(scene));
            }

            ResourceKind::Animation => {
                try_insert(path, (), &mut self.resource.animation).map_err(make_error)?;
            }

            ResourceKind::Background => {
                try_insert(path, (), &mut self.resource.background).map_err(make_error)?;
            }

            ResourceKind::Figure => {
                self.resource.insert_figure(path, f).map_err(make_error)?;
            }

            ResourceKind::Bgm => {
                try_insert(path, (), &mut self.resource.bgm).map_err(make_error)?;
            }

            ResourceKind::Vocal => {
                try_insert(path, (), &mut self.resource.vocal).map_err(make_error)?;
            }

            ResourceKind::Video => {
                try_insert(path, (), &mut self.resource.video).map_err(make_error)?;
            }

            ResourceKind::Other => {}
        }

        Ok(())
    }

    /// 移除项目文件或目录
    ///
    /// # Arguments
    /// * **path** - 文件或目录相对于项目根目录的路径.
    pub fn remove(&mut self, path: &str) -> Result<()> {
        let path = try_canonicalize(path)?;
        let info = ResourceInfo::from_path(&path);

        if !info.is_relevant_file() {
            return Ok(());
        }

        let make_error = |detail| Error {
            path: path.clone(),
            kind: info.kind,
            detail,
        };
        let ResourceInfo { kind, path, .. } = info;

        match kind {
            ResourceKind::Config => return Err(make_error(ErrorKind::RemoveConfig)),

            ResourceKind::Scene => {
                let scene = try_remove(path, &mut self.resource.scene).map_err(make_error)?;
                self.ident.remove_scene(&scene);
            }

            ResourceKind::Animation => {
                try_remove(path, &mut self.resource.animation).map_err(make_error)?;
            }

            ResourceKind::Background => {
                try_remove(path, &mut self.resource.background).map_err(make_error)?;
            }

            ResourceKind::Figure => {
                try_remove(path, &mut self.resource.figure).map_err(make_error)?;
            }

            ResourceKind::Bgm => {
                try_remove(path, &mut self.resource.bgm).map_err(make_error)?;
            }

            ResourceKind::Vocal => {
                try_remove(path, &mut self.resource.vocal).map_err(make_error)?;
            }

            ResourceKind::Video => {
                try_remove(path, &mut self.resource.video).map_err(make_error)?;
            }

            ResourceKind::Other => {}
        }

        Ok(())
    }
}

fn try_canonicalize(path: &str) -> Result<String> {
    let make_error = |error| Error {
        path: path.to_string(),
        kind: ResourceKind::Other,
        detail: ErrorKind::InvalidPath(error),
    };
    let path = canonicalize(path).ok_or_else(|| make_error(PathError::OutsideRoot))?;
    if path.is_empty() {
        return Err(make_error(PathError::IsFolder));
    }
    Ok(path)
}

fn try_entry_of<'a, 'p, T>(
    path: &'p str,
    folder: &'a mut Folder<T>,
) -> result::Result<Entry<'a, 'p, T>, ErrorKind> {
    let entry = folder
        .entry(path)
        .map_err(|_| ErrorKind::InvalidPath(PathError::CrashFile))?;
    if matches!(&entry, Entry::Occupied(o) if o.get().is_folder()) {
        return Err(ErrorKind::InvalidPath(PathError::IsFolder));
    }
    Ok(entry)
}

fn try_insert<T>(path: &str, item: T, folder: &mut Folder<T>) -> result::Result<(), ErrorKind> {
    let entry = try_entry_of(path, folder)?;
    entry.insert_entry(Node::Item(item));
    Ok(())
}

fn try_remove<T>(path: &str, folder: &mut Folder<T>) -> result::Result<T, ErrorKind> {
    let entry = folder
        .entry(path)
        .map_err(|_| ErrorKind::InvalidPath(PathError::CrashFile))?;
    match entry {
        Entry::Vacant(_) => Err(ErrorKind::InvalidPath(PathError::NotFound)),
        Entry::Occupied(o) => o
            .remove()
            .into_item()
            .ok_or(ErrorKind::InvalidPath(PathError::IsFolder)),
    }
}
