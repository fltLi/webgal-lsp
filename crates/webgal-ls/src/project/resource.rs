//! 项目场景和资源信息

use std::result;

use path_tree::{Folder, Node};
use tokio::task;
use webgal_model::{
    resource::{FigureInfo, FigureKind, Live2dModel, WmdlModel},
    sentence::Scene,
};

use crate::project::{ErrorKind, FigureError, try_entry_of};

/// 项目资源
#[derive(Debug, Default)]
pub struct Resource {
    // 场景
    pub scene: Folder<Scene>,
    // 动画
    pub animation: Folder<()>,
    // 立绘和图像
    pub background: Folder<()>,
    pub figure: Folder<FigureInfo>,
    // 音视频
    pub bgm: Folder<()>,
    pub vocal: Folder<()>,
    pub video: Folder<()>,
}

impl Resource {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取立绘文件
    ///
    /// # Behavior
    /// * 对于 WMDL 模型, 执行一次子模型重定向.
    pub fn get_figure(&self, path: &str) -> Option<&FigureInfo> {
        match self.figure.get(path)?.as_item()? {
            FigureInfo::Wmdl { import } => self.figure.get(import).and_then(Node::as_item),
            info => Some(info),
        }
    }

    /// 插入 / 修改立绘文件
    pub async fn insert_figure<F, Fut>(&mut self, path: &str, f: F) -> result::Result<(), ErrorKind>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<String>>,
    {
        // 定位模型
        let (kind, path) = FigureKind::from_path(path);
        let entry = try_entry_of(path, &mut self.figure)?;

        // 解析模型类型和信息
        let info = match kind {
            FigureKind::Live2d => {
                let content = f().await.map_err(ErrorKind::Content)?;
                task::spawn_blocking(
                    move || match serde_json::from_str::<Live2dModel>(&content) {
                        Ok(model) => model.to_info(),
                        Err(_) => FigureInfo::default(), // 回退到图片
                    },
                )
                .await
                .unwrap()
            }

            FigureKind::Wmdl => {
                let content = f().await.map_err(ErrorKind::Content)?;
                task::spawn_blocking(move || {
                    let model: WmdlModel =
                        serde_json::from_str(&content).map_err(FigureError::WmdlParse)?;
                    Ok::<_, ErrorKind>(model.to_info())
                })
                .await
                .unwrap()?
            }

            _ => kind.try_to_info().unwrap_or_default(),
        };

        // 加入模型
        entry.insert_entry(Node::Item(info));
        Ok(())
    }

    /// 检查动画是否存在
    ///
    /// 在动画目录 (非递归) 查找 `{animation}.json` 是否存在.
    pub fn contains_animation(&self, animation: &str) -> bool {
        self.animation.contains(&format!("{animation}.json"))
    }
}
