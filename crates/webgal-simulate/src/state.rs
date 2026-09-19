//! 状态计算

use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    mem,
    rc::Rc,
    result,
    str::FromStr,
};

use derive_more::{Deref, Into};
use expression::{Expression, Value};
use getset::{Getters, MutGetters};
use webgal_language_core::{resource::Config, sentence::*};

use crate::{
    DiagnosticKind, PrimaryDiagnostic, ProjectView, SentenceLocation,
    scene::Project,
    state::{effect::*, stage::*, variable::VariableTable},
};

mod effect;
mod stage;
mod variable;

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
    /// # Notes
    /// 舞台状态增量仅针对舞台状态 (`stage`), 变量修改和场景跳转等操作将不会作为舞台效果加入等待.
    pending_deltas: Vec<EffectList>,

    /// 变量表 (普通变量基底层 + 局部变量作用域栈)
    ///
    /// # Notes
    /// 设置变量的效果为立即执行, 这会导致待应用的舞台变换增量中关于其的引用过时.
    /// 为了解决这个问题, 需要在 [`StageEffect`] 构造时就从当前变量表取出需要的值.
    variables: VariableTable,

    /// 标签表, 键为标签名, 值为标签语句所在位置
    #[getset(get = "pub", get_mut = "pub")]
    labels: HashMap<String, SentenceLocation>,

    /// 场景调用栈, 记录 `callScene` 的调用方位置与返回值写入目标
    ///
    /// # Notes
    /// 场景调用栈与局部变量作用域栈同步增减, 必须经由 [`Self::push_call`] 与 [`Self::pop_call`] 操作.
    call_stack: Vec<CallFrame>,
}

/// 场景调用帧
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallFrame {
    /// 调用方位置 (场景返回后恢复)
    pub location: SentenceLocation,
    /// 返回值写入的变量名
    pub write_return_to: Option<String>,
}

impl State {
    /// 读取项目配置构建初始状态
    ///
    /// # Behavior
    /// * 将所有条目视为普通变量插入基底层.
    pub fn from_config(config: &Config) -> Self {
        let mut variables = VariableTable::default();
        for config in config.iter() {
            variables.set_base(
                config.name.clone(),
                Value::from_str(&config.value)
                    .unwrap_or_else(|_| Value::String(config.value.clone())),
            );
        }
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

        // 哈希调用栈 (位置 + 返回值写入目标)
        for frame in &self.call_stack {
            frame.location.hash(&mut hasher);
            frame.write_return_to.hash(&mut hasher);
        }

        // 哈希标签表 (为了保证顺序无关性, 先排序再哈希键值对)
        let mut labels: Vec<_> = self.labels.iter().collect();
        labels.sort_by_key(|(name, _)| name.as_str());
        for label in labels {
            label.hash(&mut hasher);
        }

        // 哈希变量表 (基底层 + 各层局部变量, 每层按名称排序)
        for scope in self.variables.scopes() {
            let mut variables: Vec<_> = scope.iter().collect();
            variables.sort_by_key(|(name, _)| name.as_str());
            for variable in variables {
                variable.hash(&mut hasher);
            }
        }

        ExecutionHash(hasher.finish())
    }

    /// 从语句构造舞台变换增量并加入待处理队列
    pub fn push_sentence_deltas<'a, P: ProjectView<'a>>(
        &mut self,
        sentence: &Sentence,
        primary: &PrimarySentence<'a>,
        project: &Project<'a, P>,
        diagnostics: Rc<RefCell<Vec<PrimaryDiagnostic>>>,
    ) {
        let delta =
            EffectList::from_sentence(sentence, primary, &self.variables, project, diagnostics);
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

    /// 设置普通变量
    pub fn set_variable(&mut self, variable: String, value: Value) -> Option<Value> {
        self.variables.set_base(variable, value)
    }

    /// 压入场景调用帧 (callScene)
    ///
    /// # Behavior
    /// * 保存调用方位置与返回值写入目标, 并压入被调用场景的局部变量作用域.
    pub fn push_call(
        &mut self,
        location: SentenceLocation,
        locals: HashMap<String, Value>,
        write_return_to: Option<String>,
    ) {
        self.call_stack.push(CallFrame {
            location,
            write_return_to,
        });
        self.variables.push_locals(locals);
        debug_assert_eq!(self.call_stack.len(), self.variables.locals_depth());
    }

    /// 弹出场景调用帧 (return / 场景自然结束)
    ///
    /// # Returns
    /// 被弹出的调用帧, 包含调用方位置与返回值写入目标.
    pub fn pop_call(&mut self) -> Option<CallFrame> {
        let frame = self.call_stack.pop()?;
        self.variables.pop_locals();
        debug_assert_eq!(self.call_stack.len(), self.variables.locals_depth());
        Some(frame)
    }

    /// 当前场景调用栈深度
    pub fn call_depth(&self) -> usize {
        self.call_stack.len()
    }

    /// 表达式求值
    pub fn evaluate_expression(
        &self,
        expression: &Expression,
    ) -> result::Result<Value, DiagnosticKind> {
        expression.evaluate(&self.variables).map_err(|error| {
            DiagnosticKind::ExpressionError(expression.to_string(), error.to_string())
        })
    }

    /// 布尔表达式求值
    pub fn evaluate_expression_as_bool(
        &self,
        expression: &Expression,
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

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    #[test]
    fn push_pop_call_keeps_locals_synced() {
        let mut state = State::default();

        // 压入调用帧: 局部变量在帧内可见
        let mut locals = HashMap::new();
        locals.insert("hp".to_string(), Value::from(100));
        state.push_call(SentenceLocation::default(), locals, Some("ret".to_string()));
        assert_eq!(state.call_depth(), 1);
        assert_eq!(
            state
                .evaluate_expression(&Expression::from_str("hp").unwrap())
                .unwrap(),
            Value::from(100)
        );

        // 弹出调用帧: 局部变量消失, 帧携带返回值写入目标
        let frame = state.pop_call().unwrap();
        assert_eq!(frame.location, SentenceLocation::default());
        assert_eq!(frame.write_return_to, Some("ret".to_string()));
        assert_eq!(state.call_depth(), 0);
        assert!(
            state
                .evaluate_expression(&Expression::from_str("hp").unwrap())
                .is_err()
        );
    }

    #[test]
    fn set_variable_writes_base_layer() {
        let mut state = State::default();
        let mut locals = HashMap::new();
        locals.insert("hp".to_string(), Value::from(100));
        state.push_call(SentenceLocation::default(), locals, None);

        // setVar 写入基底层, 不影响局部变量
        state.set_variable("hp".to_string(), Value::from(200));
        assert_eq!(
            state
                .evaluate_expression(&Expression::from_str("hp").unwrap())
                .unwrap(),
            Value::from(100)
        );

        // 弹出后基底层值可见
        assert!(state.pop_call().is_some());
        assert_eq!(
            state
                .evaluate_expression(&Expression::from_str("hp").unwrap())
                .unwrap(),
            Value::from(200)
        );
    }
}
