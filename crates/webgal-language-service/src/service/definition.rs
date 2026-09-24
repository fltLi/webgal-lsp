use lsp_types::*;
use path_tree::join;

use crate::{
    project::Project,
    service::{position_in_range, variable_location_to_range},
};

pub fn definition_capability() -> OneOf<bool, DefinitionOptions> {
    OneOf::Left(true)
}

/// 转到定义
///
/// # Returns
/// 变量定义列表，需要调用者手动拼接 URI.
pub fn definition(
    scene_path: &str,
    position: Position,
    project: &Project,
) -> Option<Vec<(String, Range)>> {
    let variable = project.variable().values().find(|variable| {
        variable
            .references
            .iter()
            .chain(
                variable
                    .definitions
                    .iter()
                    .map(|definition| &definition.location),
            )
            .any(|location| {
                location.scene == scene_path
                    && position_in_range(position, variable_location_to_range(location))
            })
    })?;

    Some(
        variable
            .definitions
            .iter()
            .map(|definition| &definition.location)
            .map(|location| (location.scene.clone(), variable_location_to_range(location)))
            .collect(),
    )
}

/// 将场景相对路径的引用转换为带完整 URI 的 LSP 位置
pub fn definitions_to_locations(
    project_root: &str,
    definitions: Vec<(String, Range)>,
) -> Vec<Location> {
    let root_uri = join(project_root, "scene");
    definitions
        .into_iter()
        .map(|(scene, range)| Location {
            uri: join(&root_uri, &scene).parse().unwrap(),
            range,
        })
        .collect()
}
