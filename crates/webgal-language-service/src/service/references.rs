use lsp_types::*;

use crate::{
    project::Project,
    service::references::{collect::*, recognize::*},
};

pub use rename::*;
pub use resource::*;

mod collect;
mod recognize;
mod rename;
mod resource;

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
    collect_ident_definition(recognize_ident(scene_path, position, project)?, project)
}

/// 查找引用
pub fn references(
    scene_path: &str,
    position: Position,
    project: &Project,
) -> Option<ReferenceList> {
    collect_ident_references(recognize_ident(scene_path, position, project)?, project)
}
