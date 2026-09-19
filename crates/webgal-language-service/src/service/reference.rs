use lsp_types::*;
use path_tree::join;

use crate::{
    project::Project,
    service::{position_in_range, variable_location_to_range},
};

// TODO: 可能的优化, 先尝试获取变量名并在变量表查找, 失败时再回退到遍历变量表
// TODO: 变量表遍历定位使用二分查找进行优化
// TODO: config.txt 中的变量引用也要查找

pub fn reference_capability() -> OneOf<bool, ReferencesOptions> {
    OneOf::Left(true)
}

/// 查找引用
///
/// # Returns
/// 变量引用列表，需要调用者手动拼接 URI.
pub fn reference(
    scene_path: &str,
    position: Position,
    project: &Project,
) -> Option<Vec<(String, Range)>> {
    let variable = project.variable().values().find(|variable| {
        variable
            .references
            .iter()
            .chain(variable.definitions.iter())
            .any(|location| {
                location.scene == scene_path
                    && position_in_range(position, variable_location_to_range(location))
            })
    })?;

    Some(
        variable
            .references
            .iter()
            .map(|location| (location.scene.clone(), variable_location_to_range(location)))
            .collect(),
    )
}

/// 将场景相对路径的引用转换为带完整 URI 的 LSP 位置。
pub fn references_to_locations(
    project_root: &str,
    references: Vec<(String, Range)>,
) -> Vec<Location> {
    let root_uri = join(project_root, "scene");
    references
        .into_iter()
        .map(|(scene, range)| Location {
            uri: join(&root_uri, &scene).parse().unwrap(),
            range,
        })
        .collect()
}
