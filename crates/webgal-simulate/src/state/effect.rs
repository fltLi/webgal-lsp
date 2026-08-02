//! 舞台状态变换

use std::{borrow::Cow, cell::RefCell, collections::HashMap, rc::Rc, result};

use serde_json::Value;
use webgal_language_core::{
    dispatch_sentence,
    element::{AnimationList, Live2dFocus, TokenSplit, Transform},
    sentence::*,
};

use crate::{DiagnosticKind, ProjectView, SymbolKind, scene::Project, state::stage::Stage};

/// 单条语句产生的舞台变换组
///
/// 一条语句可能产生多个原子效果, 这些效果组合在一起构成一个逻辑上的变换单元
#[derive(Debug, Clone)]
pub struct EffectList {
    effects: Vec<StageEffect>,
    diagnostics: Rc<RefCell<Vec<DiagnosticKind>>>,
}

impl EffectList {
    pub fn from_sentence<'a, P: ProjectView<'a>>(
        sentence: &Sentence,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        diagnostics: Rc<RefCell<Vec<DiagnosticKind>>>,
    ) -> Self {
        let effects =
            StageEffect::from_sentence(sentence, variables, project, &mut diagnostics.borrow_mut());
        Self {
            effects,
            diagnostics,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn apply_to_stage(self, prev_stage: &Stage, next_stage: &mut Stage) {
        let mut diagnostics = self.diagnostics.borrow_mut();
        for effect in self.effects {
            effect.apply_to_stage(prev_stage, next_stage, &mut diagnostics);
        }
    }
}

/// 对舞台的原子变换操作
#[derive(Debug, Clone)]
enum StageEffect {
    // 对话
    SetTextboxSpeaker(String),
    SetTextboxContent(Vec<String>),
    AddTextboxContent(Vec<String>),
    SetTextboxVisibility(bool),

    // 背景
    SetBackground(String),
    RemoveBackground,
    SetBackgroundTransform(Box<Transform>),
    SetBackgroundAnimation(AnimationList),
    SetBackgroundExitAnimation(AnimationList),
    SetBackgroundComplexAnimation(String),

    // 立绘
    SetFigure(String, String),
    RemoveFigure(String),
    SetFigureTransform(String, Box<Transform>),
    SetFigureAnimation(String, AnimationList),
    SetFigureExitAnimation(String, AnimationList),
    SetFigureComplexAnimation(String, String),
    SetFigureMotion(String, String),
    SetFigureExpression(String, String),
    SetFigureFocus(String, Live2dFocus),
    SetFigureMouthOpen(String, String),
    SetFigureMouthHalfOpen(String, String),
    SetFigureMouthClose(String, String),
    SetFigureEyesOpen(String, String),
    SetFigureEyesClose(String, String),

    // 背景音乐
    SetBgm(String),
    RemoveBgm,

    // 效果音
    // TODO: 维护临时效果音
    SetLoopingSound(String, String),
    RemoveLoopingSound(String),
    RemoveAllLoopingSound,

    // 舞台效果
    SetStageTransform(Box<Transform>),
    SetStageAnimation(AnimationList),
    SetStagePixiEffect(String),
    RemoveStagePixiEffect,
}

impl StageEffect {
    fn from_sentence<'a, P: ProjectView<'a>>(
        sentence: &Sentence,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) -> Vec<Self> {
        sentence.to_effects(variables, project, diagnostics)
    }

    /// 将单个舞台变换效果应用到目标状态, 同时返回诊断结果
    fn apply_to_stage(
        self,
        prev_stage: &Stage,
        next_stage: &mut Stage,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        todo!()
        // match self {}
    }
}

trait ToEffects {
    /// 生成舞台变换效果
    ///
    /// # Notes
    /// 此函数为 [`Self::extend_effects`] 的封装, 若要实现相关功能请重载其.
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) -> Vec<StageEffect> {
        let mut effects = Vec::new();
        self.extend_effects(variables, project, &mut effects, diagnostics);
        effects
    }

    /// 生成舞台变换效果
    #[allow(unused_variables)]
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        Default::default()
    }
}

impl ToEffects for Sentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        dispatch_sentence!(self.extend_effects(variables, project, effects, diagnostics))
    }
}

// -------- 常规演出 --------

impl ToEffects for SaySentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        if let Some(speaker) = &self.speaker {
            effects.push(StageEffect::SetTextboxSpeaker(speaker.clone()));
        }

        let content = self
            .content
            .iter()
            .map(|text| {
                TokenSplit::new(text)
                    .map(|token| interpolate(token.text, variables))
                    .collect::<result::Result<String, _>>()
            })
            .collect();
        match content {
            Ok(content) if self.concat => effects.push(StageEffect::AddTextboxContent(content)),
            Ok(content) => effects.push(StageEffect::SetTextboxContent(content)),
            Err(error) => diagnostics.push(error),
        }
    }
}

impl ToEffects for ChangeBackgroundSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for ChangeFigureSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for BgmSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for PlayVideoSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for PlayEffectSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

// -------- 舞台对象控制 --------

impl ToEffects for SetAnimationSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for SetComplexAnimationSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for SetTransformSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for SetTempAnimationSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for SetTransitionSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

// -------- 特殊演出 --------

impl ToEffects for PixiPerformSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for PixiInitSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for IntroSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for MiniAvatarSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for SetTextboxSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

impl ToEffects for FilmModeSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        unimplemented!()
    }
}

// -------- 场景与分支 --------

impl ToEffects for CallSceneSentence {}

impl ToEffects for ChangeSceneSentence {}

impl ToEffects for ChooseSentence {}

impl ToEffects for LabelSentence {}

impl ToEffects for JumpLabelSentence {}

// -------- 鉴赏 --------

impl ToEffects for UnlockCgSentence {}

impl ToEffects for UnlockBgmSentence {}

// -------- 游戏控制 --------

impl ToEffects for GetUserInputSentence {}

impl ToEffects for SetVarSentence {}

impl ToEffects for ShowVarsSentence {}

impl ToEffects for WaitSentence {}

impl ToEffects for ApplyStyleSentence {}

impl ToEffects for CallSteamSentence {}

impl ToEffects for EndSentence {}

// -------- 空白注释 --------

impl ToEffects for CommentSentence {}

// -------- util --------

/// 字符串变量插值
///
/// # Returns
/// 插值结果字符串.
/// 若无插值调用, 则返回原始字符串引用.
///
/// # Errors
/// 变量不存在时, 返回 [`DiagnosticKind::UndefinedSymbol`] 错误.
///
/// # Behavior
/// * 贪心匹配 `{` 和 `}`, 尝试替换变量.
/// * 匹配内容直接视为变量名, 不处理转义, 空白字符移除, 表达式求值等.
fn interpolate<'a>(
    input: &'a str,
    variables: &HashMap<String, Value>,
) -> result::Result<Cow<'a, str>, DiagnosticKind> {
    if !input.contains('{') {
        return Ok(Cow::Borrowed(input));
    }

    let mut start = 0;
    let mut result = String::with_capacity(input.len());

    while start < input.len()
        && let Some(idx) = input[start..].find('{')
        && let Some(len) = input[start + idx..].find('}')
    {
        let variable = &input[start + idx + 1..start + idx + len];
        let value = variables.get(variable).ok_or_else(|| {
            DiagnosticKind::UndefinedSymbol(SymbolKind::Variable, variable.to_string())
        })?;

        result.push_str(&input[start..start + idx]);
        match value {
            Value::String(s) => result.push_str(s),
            value => result.push_str(&value.to_string()),
        }

        start += idx + len + 1;
    }

    result.push_str(&input[start..]);
    Ok(Cow::Owned(result))
}
