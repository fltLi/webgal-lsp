//! 全局变量信息

use std::{collections::HashMap, mem, ops::Range};

use derive_more::{Deref, DerefMut, From, Into, IntoIterator, TryInto};
use expression::{Expression, TypeContext, ValueKind};
use webgal_language_core::{
    element::{ChoiceSplit, variables_of},
    sentence::{ReturnSentence, Scene, Sentence, SentenceExt, SentenceInfo},
    util::span_of,
};

// TODO: 区分变量不同作用域和类型覆盖

/// 变量信息表
///
/// 场景变量信息更改时触发项目级变量信息重建.
#[derive(Debug, Clone, Default, From, Into, IntoIterator, Deref, DerefMut)]
pub struct VariableTable(HashMap<String, VariableInfo>);

/// 变量信息
#[derive(Debug, Clone, Default, Hash)]
pub struct VariableInfo {
    pub kind: VariableKind,
    pub definitions: Vec<VariableLocation>,
    pub references: Vec<VariableLocation>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, From, TryInto)]
pub enum VariableKind {
    Known(ValueKind),
    #[default]
    Unknown,
    Conflict,
    // Undefined,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableLocation {
    pub scene: String,
    pub line: usize,
    pub span: Range<usize>,
}

impl VariableKind {
    pub fn as_kind(&self) -> Option<ValueKind> {
        match *self {
            Self::Known(kind) => Some(kind),
            _ => None,
        }
    }

    fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Known(a), Self::Known(b)) if a == b => *self,
            (Self::Known(_), Self::Known(_)) => Self::Conflict,
            (Self::Conflict, _) | (_, Self::Conflict) => Self::Conflict,
            (_, Self::Unknown) => *self,
            (Self::Unknown, _) => *other,
        }
    }
}

impl From<Option<ValueKind>> for VariableKind {
    fn from(value: Option<ValueKind>) -> Self {
        if let Some(kind) = value {
            Self::Known(kind)
        } else {
            Self::Unknown
        }
    }
}

// -------- build --------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DestinationKind<'a> {
    Variable(&'a str),
    SceneReturn(&'a str),
}

impl VariableTable {
    pub fn build<'a, I: IntoIterator<Item = (String, &'a Scene)>>(iter: I) -> Self {
        // 1. 收集变量信息
        let scenes: Vec<_> = iter
            .into_iter()
            .map(|(path, scene)| (path, SceneVariables::collect(scene)))
            .collect();

        // 2. 初始化变量信息
        let mut variables = Self::default();

        // 添加定义
        for (path, scene_variables) in &scenes {
            for definition in &scene_variables.definitions {
                if matches!(definition.kind, VariableDefinitionKind::ReturnDefinition(_)) {
                    continue;
                }
                variables
                    .entry(definition.name.to_string())
                    .or_default()
                    .definitions
                    .push(VariableLocation {
                        scene: path.to_string(),
                        line: definition.line,
                        span: definition.span.clone(),
                    });
            }
        }

        // 添加引用
        for (path, scene_variables) in &scenes {
            for reference in &scene_variables.references {
                if let Some(variable) = variables.get_mut(reference.name) {
                    variable.references.push(VariableLocation {
                        scene: path.to_string(),
                        line: reference.line,
                        span: reference.span.clone(),
                    });
                }
            }
        }

        // 3. 初始化依赖图
        let mut anonymous_destinations = Vec::new();
        let mut variable_destinations: HashMap<_, Vec<_>> = HashMap::new();
        let mut scene_return_destinations: HashMap<_, Vec<_>> = HashMap::new();

        for (path, scene_variables) in &scenes {
            for definition in &scene_variables.definitions {
                match definition.kind {
                    VariableDefinitionKind::Expression(expression) => {
                        let variables = expression.variables();
                        if variables.is_empty() {
                            anonymous_destinations
                                .push((DestinationKind::Variable(definition.name), expression));
                            continue;
                        }

                        for variable in variables {
                            variable_destinations
                                .entry(variable)
                                .or_default()
                                .push((DestinationKind::Variable(definition.name), expression));
                        }
                    }

                    VariableDefinitionKind::ReturnDefinition(expression) => {
                        let variables = expression.variables();
                        if variables.is_empty() {
                            anonymous_destinations
                                .push((DestinationKind::SceneReturn(path), expression));
                            continue;
                        }

                        for variable in variables {
                            variable_destinations
                                .entry(variable)
                                .or_default()
                                .push((DestinationKind::SceneReturn(path), expression));
                        }
                    }

                    VariableDefinitionKind::ReturnDestination(target) => {
                        scene_return_destinations
                            .entry(target)
                            .or_default()
                            .push(definition.name);
                    }
                }
            }
        }

        // 4. 传播类型推断
        let mut queue = Vec::with_capacity(
            anonymous_destinations.len()
                + variable_destinations.len()
                + scene_return_destinations.len(),
        );
        let mut scene_kinds = HashMap::new();

        // 推入初始可推断变量 (无依赖的常量表达式)
        for (destination, expression) in anonymous_destinations {
            if let Some(kind) = expression.infer_type(&variables) {
                queue.push((destination, VariableKind::Known(kind)));
            }
        }

        // 执行求解: 工作列表式类型传播, 直至收敛
        while let Some((destination, kind)) = queue.pop() {
            match destination {
                // 变量类型更新
                DestinationKind::Variable(name) => {
                    let old = variables.get(name).map(|v| v.kind).unwrap_or_default();
                    let new_kind = old.merge(&kind);
                    if new_kind == old {
                        continue;
                    }
                    variables.get_mut(name).unwrap().kind = new_kind;

                    // 取出依赖列表, 重算依赖者的类型并传播
                    let dependents = mem::take(variable_destinations.entry(name).or_default());
                    let mut keep = Vec::with_capacity(dependents.len());
                    for (destination, expression) in dependents {
                        // 目标已冲突则剪枝
                        let destination_kind = match destination {
                            DestinationKind::Variable(name) => {
                                variables.get(name).map(|v| v.kind).unwrap_or_default()
                            }
                            DestinationKind::SceneReturn(path) => {
                                scene_kinds.get(path).copied().unwrap_or_default()
                            }
                        };
                        if destination_kind == VariableKind::Conflict {
                            continue;
                        }

                        // 源变量冲突则扩散冲突, 否则依据当前已推断类型重算
                        let inferred = match new_kind {
                            VariableKind::Conflict => VariableKind::Conflict,
                            _ => expression.infer_type(&variables).into(),
                        };
                        if inferred != VariableKind::Unknown {
                            queue.push((destination, inferred));
                        }
                        keep.push((destination, expression));
                    }
                    variable_destinations.insert(name, keep);
                }

                // 场景返回值类型更新, 并传播给 writeReturnTo 目标变量
                DestinationKind::SceneReturn(path) => {
                    let old = scene_kinds.get(path).copied().unwrap_or_default();
                    let new_kind = old.merge(&kind);
                    if new_kind == old {
                        continue;
                    }
                    scene_kinds.insert(path, new_kind);

                    if let Some(names) = scene_return_destinations.get(path) {
                        queue.extend(
                            names
                                .iter()
                                .map(|name| (DestinationKind::Variable(name), new_kind)),
                        );
                    }
                }
            }
        }

        variables
    }
}

impl TypeContext for VariableTable {
    fn type_of_variable(&self, name: &str) -> Option<ValueKind> {
        self.get(name)?.kind.as_kind()
    }

    fn type_of_function(&self, name: &str) -> Option<ValueKind> {
        match name {
            "range" => Some(ValueKind::Number),
            _ => None,
        }
    }
}

// -------- scene --------

/// 比较场景变化前后变量信息是否改变
pub fn is_scene_variables_changed(prev: &Scene, next: &Scene) -> bool {
    SceneVariables::collect(prev) != SceneVariables::collect(next)
}

/// 场景变量信息
#[derive(Debug, Clone, Default, PartialEq)]
struct SceneVariables<'a> {
    definitions: Vec<VariableDefinition<'a>>,
    references: Vec<VariableReference<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
struct VariableDefinition<'a> {
    name: &'a str,
    kind: VariableDefinitionKind<'a>,
    line: usize,
    span: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VariableDefinitionKind<'a> {
    Expression(&'a Expression),
    ReturnDestination(&'a str),
    ReturnDefinition(&'a Expression),
}

#[derive(Debug, Clone, PartialEq)]
struct VariableReference<'a> {
    name: &'a str,
    line: usize,
    span: Range<usize>,
}

impl<'a> SceneVariables<'a> {
    /// 收集场景的变量信息
    fn collect(scene: &'a Scene) -> Self {
        let mut variables = Self::default();
        for (i, sentence) in scene.sentences().iter().enumerate() {
            variables.collect_sentence(sentence, i);
        }
        variables
    }

    fn collect_sentence(&mut self, sentence: &'a SentenceInfo<'a>, line: usize) {
        if let Some(expression) = sentence.condition()
            && let Some((_, Some(when))) = sentence.primary.get_argument("when")
        {
            let span = sentence.primary.get_span(when);
            self.collect_expression_reference(expression, span, line);
        }

        match &sentence.sentence {
            // 常规演出
            Sentence::Say(_) => {
                self.collect_reference([sentence.primary.command], sentence.content, line);
                if let Some(content) = sentence.primary.content {
                    self.collect_reference(content.split('|'), sentence.content, line);
                }
                if let Some((_, Some(speaker))) = sentence.primary.get_argument("speaker") {
                    self.collect_reference([speaker], sentence.content, line);
                }
            }

            // 特殊演出
            Sentence::Intro(_) if let Some(content) = sentence.primary.content => {
                self.collect_reference(content.split('|'), sentence.content, line);
            }

            // 场景与分支
            Sentence::CallScene(call_scene) => {
                if call_scene.write_return_to.is_some()
                    && let Some((_, Some(name))) = sentence.primary.get_argument("writeReturnTo")
                {
                    let span = sentence.primary.get_span(name);
                    self.definitions.push(VariableDefinition {
                        name,
                        kind: VariableDefinitionKind::ReturnDestination(&call_scene.scene),
                        line,
                        span,
                    });
                }

                for (name, expression) in &call_scene.variables {
                    let (name, value) =
                        sentence.primary.arguments[sentence.primary.get_argument(name).unwrap().0];

                    let name_span = sentence.primary.get_span(name);
                    self.definitions.push(VariableDefinition {
                        name,
                        kind: VariableDefinitionKind::Expression(expression),
                        line,
                        span: name_span,
                    });

                    let expression_span = sentence.primary.get_span(value.unwrap());
                    self.collect_expression_reference(expression, expression_span, line);
                }
            }
            Sentence::Choose(choose) if let Some(content) = sentence.primary.content => {
                for (choice, choice_view) in choose.choices.iter().zip(ChoiceSplit::new(content)) {
                    if let Some(expression) = &choice.show {
                        let span = sentence.primary.get_span(choice_view.show.unwrap());
                        self.collect_expression_reference(expression, span, line);
                    }
                    if let Some(expression) = &choice.enable {
                        let span = sentence.primary.get_span(choice_view.enable.unwrap());
                        self.collect_expression_reference(expression, span, line);
                    }
                }
            }
            Sentence::Return(ReturnSentence { value, .. })
                if let Some(content) = sentence.primary.content =>
            {
                let span = sentence.primary.get_span(content);
                self.definitions.push(VariableDefinition {
                    name: "",
                    kind: VariableDefinitionKind::ReturnDefinition(value),
                    line,
                    span: span.clone(),
                });
                self.collect_expression_reference(value, span, line);
            }

            // 游戏控制
            Sentence::SetVariable(set_variable)
                if let Some(content) = sentence.primary.content
                    && let Some((name, expression)) = content.split_once('=') =>
            {
                let name_span = sentence.primary.get_span(name);
                self.definitions.push(VariableDefinition {
                    name,
                    kind: VariableDefinitionKind::Expression(&set_variable.expression.1),
                    line,
                    span: name_span,
                });

                let expression_span = sentence.primary.get_span(expression);
                self.collect_expression_reference(
                    &set_variable.expression.1,
                    expression_span,
                    line,
                );
            }

            _ => {}
        }
    }

    fn collect_reference<I: IntoIterator<Item = &'a str>>(
        &mut self,
        iter: I,
        content: &str,
        line: usize,
    ) {
        self.references
            .extend(
                iter.into_iter()
                    .flat_map(variables_of)
                    .map(|name| VariableReference {
                        name,
                        line,
                        span: span_of(content, name),
                    }),
            );
    }

    fn collect_expression_reference(
        &mut self,
        expression: &'a Expression,
        span: Range<usize>,
        line: usize,
    ) {
        self.references.extend(
            expression
                .variables()
                .into_iter()
                .map(|name| VariableReference {
                    name,
                    line,
                    span: span.clone(), // TODO: 精细化变量勾画
                }),
        );
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    /// 构建变量表: 输入 (场景路径, 场景内容) 列表
    fn build(pairs: &[(&str, &str)]) -> VariableTable {
        let owned: Vec<Scene> = pairs
            .iter()
            .map(|&(_, source)| Scene::from_str(source))
            .collect();
        let input: Vec<(String, &Scene)> = pairs
            .iter()
            .zip(owned.iter())
            .map(|(&(path, _), scene)| (path.to_string(), scene))
            .collect();
        VariableTable::build(input)
    }

    fn kind<'a>(table: &'a VariableTable, name: &str) -> &'a VariableKind {
        &table.get(name).expect("变量应存在于表中").kind
    }

    // -------- 类型推断 --------

    #[test]
    fn infers_constant_type() {
        let table = build(&[("1.txt", "setVar:hp=100;")]);
        assert_eq!(*kind(&table, "hp"), VariableKind::Known(ValueKind::Number));
    }

    #[test]
    fn infers_string_literal() {
        let table = build(&[("1.txt", "setVar:name=\"小明\";")]);
        assert_eq!(
            *kind(&table, "name"),
            VariableKind::Known(ValueKind::String)
        );
    }

    #[test]
    fn propagates_through_dependency() {
        let table = build(&[("1.txt", "setVar:a=5;\nsetVar:b=a+1;")]);
        assert_eq!(*kind(&table, "a"), VariableKind::Known(ValueKind::Number));
        assert_eq!(*kind(&table, "b"), VariableKind::Known(ValueKind::Number));
    }

    #[test]
    fn propagates_along_chain() {
        let table = build(&[("1.txt", "setVar:a=5;\nsetVar:b=a;\nsetVar:c=b+1;")]);
        assert_eq!(*kind(&table, "c"), VariableKind::Known(ValueKind::Number));
    }

    #[test]
    fn stays_unknown_when_no_source() {
        // b 依赖 a, 但 a 从未被定义 -> 无从推断
        let table = build(&[("1.txt", "setVar:b=a+1;")]);
        assert_eq!(*kind(&table, "b"), VariableKind::Unknown);
    }

    // -------- 冲突 --------

    #[test]
    fn detects_conflicting_writes() {
        let table = build(&[("1.txt", "setVar:x=1;\nsetVar:x=true;")]);
        assert_eq!(*kind(&table, "x"), VariableKind::Conflict);
    }

    #[test]
    fn propagates_conflict() {
        let table = build(&[("1.txt", "setVar:x=1;\nsetVar:x=true;\nsetVar:y=x+1;")]);
        assert_eq!(*kind(&table, "x"), VariableKind::Conflict);
        assert_eq!(*kind(&table, "y"), VariableKind::Conflict);
    }

    // -------- 场景返回值 --------

    #[test]
    fn propagates_scene_return_constant() {
        let table = build(&[
            ("a.txt", "return:100;"),
            ("1.txt", "callScene:a.txt -writeReturnTo=result;"),
        ]);
        assert_eq!(
            *kind(&table, "result"),
            VariableKind::Known(ValueKind::Number)
        );
    }

    #[test]
    fn propagates_scene_return_via_variable() {
        let table = build(&[
            ("a.txt", "setVar:base=10;\nreturn:base;"),
            ("1.txt", "callScene:a.txt -writeReturnTo=result;"),
        ]);
        assert_eq!(
            *kind(&table, "base"),
            VariableKind::Known(ValueKind::Number)
        );
        assert_eq!(
            *kind(&table, "result"),
            VariableKind::Known(ValueKind::Number)
        );
    }
}
