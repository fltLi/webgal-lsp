//! 模拟执行

use std::{collections::VecDeque, iter};

use derive_more::From;
use serde_json::Value;
use webgal_language_core::{
    element::Forward,
    sentence::{GetUserInputSentence, Sentence},
};

use crate::{DiagnosticKind, DiagnosticList, StopReason, SymbolKind, scene::*, state::*};

/// 模拟执行入口场景
pub const START_SCENE: &str = "start.txt";

/// 单个检查点最大通过次数
pub const MAX_CHECKPOINT_VISITS: usize = 32;

/// 模拟执行 WebGAL 项目, 提供诊断信息
pub fn simulate<'a, P: ProjectView<'a>>(project_view: P) -> DiagnosticList {
    // 初始化项目和执行器
    let project = Project::new(project_view);
    let simulator = match Simulator::new(&project) {
        Some(v) => v,
        None => return DiagnosticList::default(),
    };

    // 模拟执行循环 (BFS 分支处理)
    let mut simulators = VecDeque::new();
    simulators.push_back(simulator);

    while let Some(simulator) = simulators.pop_front() {
        match simulator.next() {
            StepOutcome::Continue(s) => simulators.push_back(s),
            StepOutcome::Branch(s) => simulators.extend(s),
            StepOutcome::Halt => {}
        }
    }

    // 检查死代码, 收集诊断信息并返回
    project.check_unused();
    project.into_diagnostics()
}

// -------- simulate --------

#[derive(Debug)]
struct Simulator<'a, 'b, P: ProjectView<'a>> {
    // 读取
    project: &'b Project<'a, P>,
    scene: &'b Scene<'a>,
    last_sentence: Option<&'b SentenceInfo<'a>>,
    // 状态
    location: SentenceLocation,
    state: Box<State>,
}

impl<'a, 'b, P: ProjectView<'a>> Simulator<'a, 'b, P> {
    fn new(project: &'b Project<'a, P>) -> Option<Self> {
        // 定位初始场景
        let location = SentenceLocation::default();
        let scene = project.scenes().get(&location.scene)?;

        // 从项目配置初始化状态
        let state = Box::new(State::from_config(project.config()));

        Some(Self {
            project,
            location,
            scene,
            last_sentence: None,
            state,
        })
    }

    fn next(mut self) -> StepOutcome<'a, 'b, P> {
        // 读取语句 (读完场景时尝试弹出调用栈)
        let sentence = match self.scene.get(self.location.line) {
            Some(v) => v,
            None => return self.pop_call_stack().into(),
        };
        self.location.line += 1; // 移动到下一条语句

        // 判定语句条件执行
        let execute = sentence.condition().is_none_or(|condition| {
            self.state
                .evaluate_expression_as_bool(condition)
                .unwrap_or_else(|error| {
                    sentence.push_diagnostic(error);
                    false
                })
        });

        if !execute {
            return self.into();
        }

        // 尝试注册执行上下文状态到检查点
        if !sentence.register_execution(|| self.state.hash_execution()) {
            sentence.push_diagnostic(DiagnosticKind::Stopped(StopReason::Checkpoint));
            return StepOutcome::Halt;
        }

        // 计算舞台状态增量
        self.state.push_sentence_deltas(
            sentence.sentence(),
            self.project,
            sentence.diagnostics().clone(),
        );

        // 若语句不为连续执行, 则应用累积的舞台状态增量
        let delta_applied = if sentence.forward() != Forward::Next {
            self.state.apply_pending_deltas()
        } else {
            false
        };

        // 更新维护的上一条语句
        self.last_sentence = Some(sentence);

        // 针对不同语句执行不同补充策略
        match sentence.sentence() {
            // 调用场景
            Sentence::CallScene(s) => {
                // 获取跳转目标
                let next_scene_name = &s.scene;
                let next_scene = match self.project.scenes().get(next_scene_name) {
                    Some(v) => v,
                    None => {
                        sentence.push_diagnostic(DiagnosticKind::UndefinedSymbol(
                            SymbolKind::Scene,
                            next_scene_name.clone(),
                        ));
                        return StepOutcome::Halt;
                    }
                };

                // 压栈并执行跳转
                self.state.call_stack_mut().push(self.location);
                self.scene = next_scene;
                self.location = SentenceLocation {
                    scene: next_scene_name.clone(),
                    line: 0,
                };
                self.into()
            }

            // 切换场景
            Sentence::ChangeScene(s) => {
                // 获取跳转目标
                let next_scene_name = &s.scene;
                let next_scene = match self.project.scenes().get(next_scene_name) {
                    Some(v) => v,
                    None => {
                        sentence.push_diagnostic(DiagnosticKind::UndefinedSymbol(
                            SymbolKind::Scene,
                            next_scene_name.clone(),
                        ));
                        return StepOutcome::Halt;
                    }
                };

                // 替换栈顶的场景并执行跳转
                self.scene = next_scene;
                self.location = SentenceLocation {
                    scene: next_scene_name.to_string(),
                    line: 0,
                };
                self.into()
            }

            // 分支选择
            Sentence::Choose(s) => {
                s.choices
                    .iter()
                    .filter_map(|choice| {
                        let target = match &choice.target {
                            Some(v) => v,
                            None => return None,
                        };

                        // 检查选项是否启用
                        let disabled = iter::once(&choice.show)
                            .chain(iter::once(&choice.enable))
                            .filter_map(|v| v.as_deref())
                            .any(|condition| {
                                self.state
                                    .evaluate_expression_as_bool(condition)
                                    .unwrap_or_else(|error| {
                                        sentence.push_diagnostic(error);
                                        false
                                    })
                            });
                        if disabled {
                            return None;
                        }

                        // 查找跳转位置
                        let location = if target.ends_with(".txt") {
                            SentenceLocation {
                                scene: target.clone(),
                                line: 0,
                            }
                        } else {
                            match self.state.labels().get(target) {
                                Some(location) => location.clone(),
                                None => {
                                    sentence.push_diagnostic(DiagnosticKind::UndefinedSymbol(
                                        SymbolKind::Label,
                                        target.clone(),
                                    ));
                                    return None;
                                }
                            }
                        };

                        // 获取跳转目标
                        let next_scene = match self.project.scenes().get(&location.scene) {
                            Some(v) => v,
                            None => {
                                sentence.push_diagnostic(DiagnosticKind::UndefinedSymbol(
                                    SymbolKind::Scene,
                                    location.scene,
                                ));
                                return None;
                            }
                        };

                        // 构造分支 (替换栈顶的场景并执行跳转)
                        let mut simulator = self.clone();
                        simulator.scene = next_scene;
                        simulator.location = location;
                        Some(simulator)
                    })
                    .collect::<Vec<_>>()
                    .into()
            }

            // 跳转标签
            Sentence::JumpLabel(s) => {
                // 获取跳转目标
                let location = match self.state.labels().get(&s.label) {
                    Some(location) => location.clone(),
                    None => {
                        sentence.push_diagnostic(DiagnosticKind::UndefinedSymbol(
                            SymbolKind::Label,
                            s.label.clone(),
                        ));
                        return StepOutcome::Halt;
                    }
                };
                let next_scene = self.project.scenes().get(&location.scene).unwrap();

                // 替换栈顶的场景并执行跳转
                self.scene = next_scene;
                self.location = location;
                self.into()
            }

            // 用户输入
            Sentence::GetUserInput(s) => {
                // 收集代入值
                let values: Vec<_> = match &**s {
                    GetUserInputSentence { lint_values, .. } if !lint_values.is_empty() => {
                        s.lint_values.iter().collect()
                    }
                    GetUserInputSentence {
                        default_value: Some(value),
                        ..
                    } => vec![value],
                    _ => {
                        sentence.push_diagnostic(DiagnosticKind::Stopped(
                            StopReason::MissingUserInputValue,
                        ));
                        return StepOutcome::Halt;
                    }
                };

                // 构造分支 (变量赋值)
                values
                    .into_iter()
                    .map(|value| {
                        let mut simulator = self.clone();
                        simulator
                            .state
                            .set_variable(s.variable.clone(), Value::String(value.clone()));
                        simulator
                    })
                    .collect::<Vec<_>>()
                    .into()
            }

            // 设置变量
            Sentence::SetVar(s) => {
                let (variable, expression) = &s.expression;

                // 计算新值
                let value = match self.state.evaluate_expression(expression) {
                    Ok(v) => v,
                    Err(error) => {
                        sentence.push_diagnostic(error);
                        return StepOutcome::Halt;
                    }
                };

                // 设置变量
                self.state.set_variable(variable.clone(), value);
                self.into()
            }

            // 等待语句
            Sentence::Wait(_) => {
                // 检查连续执行是否被 wait 语句打断
                if delta_applied {
                    sentence.push_diagnostic(DiagnosticKind::WaitAtEndOfChain);
                }

                self.into()
            }

            // 结束游戏
            Sentence::End(_) => {
                sentence.push_diagnostic(DiagnosticKind::Stopped(StopReason::NormalTermination));
                StepOutcome::Halt
            }

            _ => self.into(),
        }
    }

    /// 尝试弹出调用栈栈顶, 恢复到上一个场景位置
    fn pop_call_stack(mut self) -> Option<Self> {
        match self.state.call_stack_mut().pop() {
            // 向上跳出
            Some(location) => {
                self.scene = self.project.scenes().get(&location.scene).unwrap();
                self.location = location;
                Some(self)
            }

            // 正常结束
            None => {
                if let Some(sentence) = self.last_sentence {
                    sentence
                        .push_diagnostic(DiagnosticKind::Stopped(StopReason::NormalTermination));
                }
                None
            }
        }
    }
}

impl<'a, 'b, P: ProjectView<'a>> Clone for Simulator<'a, 'b, P> {
    fn clone(&self) -> Self {
        Self {
            project: self.project,
            scene: self.scene,
            last_sentence: self.last_sentence,
            location: self.location.clone(),
            state: self.state.clone(),
        }
    }
}

#[derive(Debug, Clone, From)]
enum StepOutcome<'a, 'b, P: ProjectView<'a>> {
    /// 单一路径继续执行
    Continue(Simulator<'a, 'b, P>),
    /// 分裂为多条路径
    Branch(Vec<Simulator<'a, 'b, P>>),
    /// 路径终止
    Halt,
}

impl<'a, 'b, P: ProjectView<'a>> From<Option<Simulator<'a, 'b, P>>> for StepOutcome<'a, 'b, P> {
    fn from(value: Option<Simulator<'a, 'b, P>>) -> Self {
        match value {
            Some(simulator) => Self::Continue(simulator),
            None => Self::Halt,
        }
    }
}
