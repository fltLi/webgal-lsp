//! 状态计算

use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    mem,
    rc::Rc,
    result,
};

use derive_more::{Deref, Into};
use getset::{Getters, MutGetters};
use serde_json::Value;
use webgal_language_core::{resource::Config, sentence::*};

use crate::{
    DiagnosticKind, ProjectView, SentenceLocation,
    expression::evaluate_with_context,
    scene::Project,
    state::{effect::*, stage::*},
};

mod effect;
mod stage;

// TODO: 使用可持久化数据结构维护 [`State`] 的字段, 减少频繁拷贝的开销

/// 模拟执行的核心状态容器
///
/// 包含: 当前已应用的舞台状态, 待应用的增量队列, 变量表, 调用栈.
#[derive(Debug, Clone, Default, Getters, MutGetters)]
pub struct State {
    /// 当前已应用的舞台状态 (不含连续执行语句产生的待处理增量)
    stage: Stage,

    /// 待应用的舞台状态增量队列
    ///
    /// 连续执行 (`-next`) 的语句将其效果累积于此;
    /// 遇到非连续执行语句时, 队列中的增量将被依次应用到 `stage` 并清空.
    ///
    /// # Note
    /// 舞台状态增量仅针对舞台状态 (`stage`), 变量修改和场景跳转等操作将不会作为舞台效果加入等待.
    pending_deltas: Vec<EffectList>,

    /// 变量表, 键为变量名, 值为 JSON 值
    ///
    /// # Note
    /// 设置变量的效果为立即执行, 这会导致待应用的舞台变换增量中关于其的引用过时.
    /// 为了解决这个问题, 需要在 [`StageEffect`] 构造时就从当前变量表取出需要的值.
    variables: HashMap<String, Value>,

    /// 标签表, 键为标签名, 值为标签语句所在位置
    #[getset(get = "pub", get_mut = "pub")]
    labels: HashMap<String, SentenceLocation>,

    /// 场景调用栈, 用于跟踪 `changeScene` 语句的跳转路径
    #[getset(get_mut = "pub")]
    call_stack: Vec<SentenceLocation>,
}

impl State {
    /// 读取项目配置构建初始状态
    ///
    /// # Behavior
    /// * 将所有条目视为变量插入变量表.
    pub fn from_config(config: &Config) -> Self {
        let variables = config
            .iter()
            .map(|config| {
                (
                    config.name.clone(),
                    serde_json::from_str(&config.value)
                        .unwrap_or_else(|_| Value::String(config.value.clone())),
                )
            })
            .collect();
        Self {
            variables,
            ..Default::default()
        }
    }

    /// 计算当前执行上下文的紧凑指纹, 用于检查点去重
    ///
    /// 舞台对象不参与指纹计算, 因为其不影响后续语句的执行效果.
    pub fn hash_execution(&self) -> ExecutionHash {
        let mut hasher = DefaultHasher::new();

        // 哈希调用栈
        for location in &self.call_stack {
            location.hash(&mut hasher);
        }

        // 哈希标签表 (为了保证顺序无关性, 先排序再哈希键值对)
        let mut labels: Vec<_> = self.labels.iter().collect();
        labels.sort_by_key(|(name, _)| name.as_str());
        for label in labels {
            label.hash(&mut hasher);
        }

        // 哈希变量表 (为了保证顺序无关性, 先排序再哈希键值对)
        let mut variables: Vec<_> = self.variables.iter().collect();
        variables.sort_by_key(|(name, _)| name.as_str());
        for variable in variables {
            variable.hash(&mut hasher);
        }

        ExecutionHash(hasher.finish())
    }

    /// 从语句构造舞台变换增量并加入待处理队列
    pub fn push_sentence_deltas<'a, P: ProjectView<'a>>(
        &mut self,
        sentence: &Sentence,
        project: &Project<'a, P>,
        diagnostics: Rc<RefCell<Vec<DiagnosticKind>>>,
    ) {
        let delta = EffectList::from_sentence(sentence, &self.variables, project, diagnostics);
        if !delta.is_empty() {
            self.pending_deltas.push(delta);
        }
    }

    /// 将待处理的舞台增量队列依次应用到当前状态, 并清空队列
    ///
    /// # Returns
    /// 是否执行状态累加.
    ///
    /// # Safety
    /// 确保诊断列表指针有效, 其将在应用变换时被解引用, 以推送诊断.
    pub fn apply_pending_deltas(&mut self) -> bool {
        if self.pending_deltas.is_empty() {
            return false;
        }

        let prev_state = self.stage.clone();
        for delta in mem::take(&mut self.pending_deltas) {
            delta.apply_to_stage(&prev_state, &mut self.stage);
        }
        true
    }

    /// 设置变量值
    pub fn set_variable(&mut self, variable: String, value: Value) -> Option<Value> {
        self.variables.insert(variable, value)
    }

    /// 表达式求值 (不修改变量)
    pub fn evaluate_expression(&self, expression: &str) -> result::Result<Value, DiagnosticKind> {
        evaluate_with_context(expression, &self.variables).map_err(|error| {
            DiagnosticKind::ExpressionError(expression.to_string(), error.to_string())
        })
    }

    /// 布尔表达式求值 (不修改变量)
    pub fn evaluate_expression_as_bool(
        &self,
        expression: &str,
    ) -> result::Result<bool, DiagnosticKind> {
        let value = self.evaluate_expression(expression)?;
        value.as_bool().ok_or_else(|| {
            DiagnosticKind::ExpressionError(
                expression.to_string(),
                format!("条件表达式结果应为布尔值, 而不是 `{value}`"),
            )
        })
    }
}

/// 执行上下文的紧凑指纹, 用于检查点去重
///
/// 哈希指纹生成方式详见 [`State::hash_execution`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into, Deref)]
pub struct ExecutionHash(u64);
