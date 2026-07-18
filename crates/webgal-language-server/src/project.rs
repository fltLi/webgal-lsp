use std::mem;

use anyhow::anyhow;
use path_tree::join;
use tokio::{runtime::Handle, task::spawn_blocking};
use webgal_language_core::resource::{Config, RESOURCE_ROOTS, ResourceKind};

pub use fs::*;
pub use webgal_language_service::project::*; // 重新导出方便使用
pub use workspace::*;

mod fs;
mod workspace;

/// 扫描目录构建项目
pub async fn load_project<F: FileSystem + Send + Sync + 'static>(
    root: &str,
    fs: &F,
    errors: &mut Vec<anyhow::Error>,
) -> Project {
    let fs: &'static F = unsafe { mem::transmute(fs) };
    let root: &'static str = unsafe { mem::transmute(root) };
    let absolute_path = move |path: &str| join(root, path);

    // 读取配置
    let config_path = absolute_path("config.txt");
    let config = match fs.read_to_string(&config_path).await {
        Ok(content) => Config::from_str(&content),
        Err(error) => {
            errors.push(
                Error {
                    path: config_path,
                    kind: ResourceKind::Config,
                    detail: ErrorKind::Content(anyhow!("无法读取项目配置: {error}")),
                }
                .into(),
            );
            Default::default()
        }
    };

    let (project, mut collected_errors) = spawn_blocking(move || {
        let mut errors = Vec::new();
        let mut project = Project::new(config);

        // 扫描资源目录
        let mut stack: Vec<_> = RESOURCE_ROOTS.iter().map(|root| root.to_string()).collect();
        while let Some(root) = stack.pop() {
            let full_root = absolute_path(&root);
            let children = match Handle::current().block_on(fs.read_dir(&full_root)) {
                Ok(v) => v,
                Err(error) => {
                    errors.push(anyhow!("遍历目录 `{full_root}` 出错: {error}"));
                    continue;
                }
            };

            for DirEntry { name, is_directory } in children {
                let path = join(&root, &name);
                if is_directory {
                    stack.push(path);
                    continue;
                }

                // 加载资源
                if let Err(error) = project.insert(&path, || {
                    let full_path = absolute_path(&path);
                    Handle::current().block_on(fs.read_to_string(&full_path))
                }) {
                    errors.push(error.into());
                }
            }
        }

        (project, errors)
    })
    .await
    .unwrap();

    errors.append(&mut collected_errors);
    project
}
