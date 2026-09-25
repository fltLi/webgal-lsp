use derive_more::{Deref, Into, IntoIterator};
use lsp_types::*;
use path_tree::join;

use crate::{
    project::Project,
    service::{position_in_range, variable_location_to_range},
};

// TODO: 可能的优化, 先尝试获取变量名并在变量表查找, 失败时再回退到遍历变量表
// TODO: 变量表遍历定位使用二分查找进行优化
// TODO: config.txt 中的变量引用也要查找

pub fn definition_capability() -> OneOf<bool, DefinitionOptions> {
    OneOf::Left(true)
}

pub fn references_capability() -> OneOf<bool, ReferencesOptions> {
    OneOf::Left(true)
}

/// 转到定义
pub fn definition(
    scene_path: &str,
    position: Position,
    project: &Project,
) -> Option<ReferenceList> {
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

    Some(ReferenceList(
        variable
            .definitions
            .iter()
            .map(|definition| &definition.location)
            .map(|location| (location.scene.clone(), variable_location_to_range(location)))
            .collect(),
    ))
}

/// 查找引用
pub fn references(
    scene_path: &str,
    position: Position,
    project: &Project,
) -> Option<ReferenceList> {
    let variable = project.variable().values().find(|variable| {
        variable.iter_references().any(|location| {
            location.scene == scene_path
                && position_in_range(position, variable_location_to_range(location))
        })
    })?;

    Some(ReferenceList(
        variable
            .iter_references()
            .map(|location| (location.scene.clone(), variable_location_to_range(location)))
            .collect(),
    ))
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Into, IntoIterator, Deref)]
pub struct ReferenceList(Vec<(String, Range)>);

impl ReferenceList {
    pub fn to_locations(self, project_root: &str) -> Vec<Location> {
        let scene_root = join(project_root, "scene");
        self.iter()
            .map(|(scene, span)| Location {
                uri: join(&scene_root, scene).parse().unwrap(),
                range: *span,
            })
            .collect()
    }
}
