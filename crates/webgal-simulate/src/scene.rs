//! 场景访问

use std::{
    collections::{BTreeSet, HashMap},
    fmt,
};

use derive_more::{Deref, DerefMut, From, Into};
use getset::{CopyGetters, Getters, MutGetters};
use rayon::prelude::*;
use webgal_language_core::{
    element::{AnimationList, Forward},
    resource::Config,
    sentence::{Sentence, SentenceExt},
};

use crate::{
    Diagnostic, DiagnosticKind, DiagnosticList, MAX_CHECKPOINT_VISITS, START_SCENE,
    expression::evaluate_constantly, state::ExecutionHash,
};

// -------- project --------

/// WebGAL 项目简单只读视图
pub trait ProjectView<'a>: Send + Sync {
    type Scene: Iterator<Item = &'a Sentence> + Send;

    /// 读取配置
    fn get_config(&self) -> &'a Config;

    /// 读取场景
    fn get_scene(&self, path: &str) -> Option<Self::Scene>;

    /// 遍历场景
    fn iter_scenes(&self) -> impl Iterator<Item = (String, Self::Scene)> + Send;

    /// 读取动画资源
    fn get_animation(&self, name: &str) -> Option<&'a AnimationList>;
}

/// 项目信息 (Simulate)
#[derive(Debug, Getters, MutGetters)]
pub struct Project<'a, P: ProjectView<'a>> {
    #[getset(get = "pub")]
    view: P,
    // 配置和资源
    #[getset(get = "pub")]
    config: &'a Config,
    #[getset(get = "pub", get_mut = "pub")]
    scenes: HashMap<String, Scene<'a>>,
}

impl<'a, P: ProjectView<'a>> Project<'a, P> {
    pub fn new(view: P) -> Self {
        let config = view.get_config();
        let scenes = view
            .iter_scenes()
            .par_bridge()
            .map(|(path, sentences)| (path, Scene::from_iter(sentences)))
            .collect();

        Self {
            view,
            config,
            scenes,
        }
    }

    pub fn into_diagnostics(self) -> DiagnosticList {
        DiagnosticList(
            self.scenes
                .into_par_iter()
                .map(|(path, scene)| (path, scene.into_diagnostics()))
                .collect(),
        )
    }
}

impl<'a, P: ProjectView<'a>> From<P> for Project<'a, P> {
    fn from(value: P) -> Self {
        Self::new(value)
    }
}

// -------- scene --------

/// 场景信息 (Simulate)
#[derive(Debug, Default, From, Into, Deref, DerefMut)]
pub struct Scene<'a>(Vec<SentenceInfo<'a>>);

impl<'a> Scene<'a> {
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.0
            .into_par_iter()
            .enumerate()
            .flat_map(|(line, sentence)| sentence.into_diagnostics(line))
            .collect()
    }
}

impl<'a> FromIterator<&'a Sentence> for Scene<'a> {
    fn from_iter<I: IntoIterator<Item = &'a Sentence>>(iter: I) -> Self {
        let mut sentences: Vec<_> = iter.into_iter().map(SentenceInfo::new).collect();

        // 在场景开头设立检查点
        if let Some(sentence) = sentences.first_mut() {
            sentence.executions = Some(BTreeSet::new());
        }

        Self(sentences)
    }
}

// -------- sentence --------

/// 语句位置信息
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into)]
pub struct SentenceLocation {
    pub scene: String,
    pub line: usize,
}

impl Default for SentenceLocation {
    fn default() -> Self {
        Self {
            scene: START_SCENE.to_string(),
            line: 0,
        }
    }
}

impl fmt::Display for SentenceLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.scene, self.line)
    }
}

/// 语句信息 (Simulate)
#[derive(Debug, Getters, MutGetters, CopyGetters)]
pub struct SentenceInfo<'a> {
    #[getset(get = "pub")]
    sentence: &'a Sentence,
    // 语句信息
    #[getset(get_copy = "pub")]
    forward: Forward,
    #[getset(get = "pub")]
    condition: Option<&'a str>,
    // 诊断结果
    #[getset(get_mut = "pub")]
    diagnostics: Vec<DiagnosticKind>,
    // 遍历信息
    visited: bool,
    executions: Option<BTreeSet<ExecutionHash>>,
}

impl<'a> SentenceInfo<'a> {
    pub fn new(sentence: &'a Sentence) -> Self {
        let forward = sentence.forward();
        let condition = sentence.condition();
        let mut diagnostics = Vec::new();

        // 判断语句是否为检查点
        let is_checkpoint = condition.is_some()
            || matches!(
                sentence,
                Sentence::Label(_) | Sentence::SetVar(_) | Sentence::GetUserInput(_)
            );
        let executions = is_checkpoint.then_some(BTreeSet::new());

        // 检查条件执行表达式是否为常量
        if let Some(condition) = condition
            && let Some(value) = evaluate_constantly(condition)
        {
            diagnostics.push(DiagnosticKind::ConstantCondition(
                condition.to_string(),
                value.to_string(),
            ));
        }

        Self {
            sentence,
            forward,
            condition,
            diagnostics,
            visited: false,
            executions,
        }
    }

    pub fn is_visited(&self) -> bool {
        self.visited
    }

    /// 尝试注册当前模拟执行上下文状态到检查点
    ///
    /// # Returns
    /// 是否允许通过检查点.
    ///
    /// # Behavior
    /// * 不为检查点是一律允许通过.
    /// * 检查点已达到通过次数上限时, 一律不允许通过.
    /// * 可以尝试注册时, 才哈希状态并尝试加入, 若已存在则不允许通过.
    /// * 不允许通过检查点时, 调用者需自行记录模拟中断信息.
    pub fn register_execution<F>(&mut self, f: F) -> bool
    where
        F: FnOnce() -> ExecutionHash,
    {
        let permitted = self.executions.as_mut().is_none_or(|executions| {
            executions.len() < MAX_CHECKPOINT_VISITS && executions.insert(f())
        });

        // 标记为已访问
        if permitted {
            self.visited = true;
        }

        permitted
    }

    pub fn into_diagnostics(mut self, line: usize) -> Vec<Diagnostic> {
        // 检查死代码
        if !self.is_visited() {
            self.diagnostics.push(DiagnosticKind::Unused);
        }

        // 诊断去重
        self.diagnostics.sort();
        self.diagnostics.dedup();

        // 附加诊断行号
        self.diagnostics
            .into_iter()
            .map(|detail| Diagnostic { line, detail })
            .collect()
    }
}
