//! 舞台状态变换

use std::{borrow::Cow, cell::RefCell, collections::HashMap, rc::Rc, result};

use serde_json::Value;
use webgal_language_core::{
    dispatch_sentence,
    element::{AnimationList, ObjectId, TokenSplit, Transform},
    sentence::*,
};

use crate::{
    DiagnosticKind, ProjectView, SymbolKind,
    scene::Project,
    state::{effect::StageEffect::SetDialogueSpeaker, stage::Stage},
};

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
    SetDialogueSpeaker(String),
    SetDialogueContent(Vec<String>),
    AddDialogueContent(Vec<String>),
    SetDialogueFigure(String),
    SetTextboxVisibility(bool),

    // 背景
    SetBackground(String),
    RemoveBackground,
    SetBackgroundTransform(Box<Transform>),
    SetBackgroundExitAnimation(String),

    // 立绘
    SetFigure(String, String),
    RemoveFigure(String),
    SetFigureTransform(String, Box<Transform>),
    SetFigureExitAnimation(String, String),
    SetFigureMotion(String, String),
    SetFigureExpression(String, String),

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

// TODO: 检查变换操作未计入的字段的变量插值

impl ToEffects for SaySentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        if let Some(speaker) = &self.speaker {
            effects.push(SetDialogueSpeaker(
                interpolate_or_record(speaker, variables, diagnostics).to_string(),
            ));
        }

        let content = self
            .content
            .iter()
            .map(|text| {
                TokenSplit::new(text)
                    .map(|token| interpolate_or_record(token.text, variables, diagnostics))
                    .collect()
            })
            .collect();
        if self.concat {
            effects.push(StageEffect::AddDialogueContent(content));
        } else {
            effects.push(StageEffect::SetDialogueContent(content));
        }

        if let Some(figure) = &self.figure {
            effects.push(StageEffect::SetDialogueFigure(
                interpolate_or_record(figure.get_id(), variables, diagnostics).to_string(),
            ));
        }
    }
}

impl ToEffects for ChangeBackgroundSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        if let Some(path) = &self.background {
            effects.push(StageEffect::SetBackground(
                interpolate_or_record(path, variables, diagnostics).to_string(),
            ))
        } else {
            effects.push(StageEffect::RemoveBackground);
            return;
        }

        if let Some(transform) = &self.transform {
            effects.push(StageEffect::SetBackgroundTransform(Box::new(
                transform.clone(),
            )));
        }
        if let Some(exit) = &self.exit {
            effects.push(StageEffect::SetBackgroundExitAnimation(
                interpolate_or_record(exit, variables, diagnostics).to_string(),
            ));
        }
    }
}

impl ToEffects for ChangeFigureSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        let id_raw = self.get_id();
        let id = interpolate_or_record(&id_raw, variables, diagnostics);

        if let Some(path) = &self.figure {
            effects.push(StageEffect::SetFigure(
                id.to_string(),
                interpolate_or_record(path, variables, diagnostics).to_string(),
            ));
        } else {
            effects.push(StageEffect::RemoveFigure(id.to_string()));
            return;
        }

        if let Some(transform) = &self.transform {
            effects.push(StageEffect::SetFigureTransform(
                id.to_string(),
                Box::new(transform.clone()),
            ));
        }
        if let Some(exit) = &self.exit {
            effects.push(StageEffect::SetFigureExitAnimation(
                id.to_string(),
                interpolate_or_record(exit, variables, diagnostics).to_string(),
            ));
        }

        if let Some(motion) = &self.motion {
            effects.push(StageEffect::SetFigureMotion(
                id.to_string(),
                interpolate_or_record(motion, variables, diagnostics).to_string(),
            ));
        }
        if let Some(expression) = &self.expression {
            effects.push(StageEffect::SetFigureExpression(
                id.to_string(),
                interpolate_or_record(expression, variables, diagnostics).to_string(),
            ));
        }
    }
}

impl ToEffects for BgmSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        if let Some(path) = &self.bgm {
            effects.push(StageEffect::SetBgm(
                interpolate_or_record(path, variables, diagnostics).to_string(),
            ));
        } else {
            effects.push(StageEffect::RemoveBgm);
        }
    }
}

impl ToEffects for PlayVideoSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        _effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        interpolate_or_record(&self.video, variables, diagnostics);
    }
}

impl ToEffects for PlayEffectSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        let id = self
            .id
            .as_ref()
            .map(|id| interpolate_or_record(id, variables, diagnostics));
        let path = self
            .vocal
            .as_ref()
            .map(|path| interpolate_or_record(path, variables, diagnostics));

        match (id, path) {
            (Some(id), Some(path)) => effects.push(StageEffect::SetLoopingSound(
                id.to_string(),
                path.to_string(),
            )),
            (Some(id), None) => effects.push(StageEffect::RemoveLoopingSound(id.to_string())),
            (None, Some(_path)) => {}
            (None, None) => effects.push(StageEffect::RemoveAllLoopingSound),
        }
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
        let id = match &self.target {
            Some(v) => v,
            None => return,
        };

        let transform = match project.view().get_animation(&self.animation) {
            Some(animations) => Box::new(merge_animations(animations, self.write_default)),
            None => return,
        };

        effects.push(make_transform_effect(transform, id, variables, diagnostics));
    }
}

impl ToEffects for SetComplexAnimationSentence {}

impl ToEffects for SetTransformSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        let id = match &self.target {
            Some(v) => v,
            None => return,
        };

        let mut transform = Box::new(self.transform.clone());
        if self.write_default {
            transform.merge(&Transform::default_values());
        }

        effects.push(make_transform_effect(transform, id, variables, diagnostics));
    }
}

impl ToEffects for SetTempAnimationSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        let id = match &self.target {
            Some(v) => v,
            None => return,
        };

        let transform = Box::new(merge_animations(&self.animation, self.write_default));

        effects.push(make_transform_effect(transform, id, variables, diagnostics));
    }
}

impl ToEffects for SetTransitionSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        let id = match &self.target {
            Some(v) => v,
            None => return,
        };

        if let Some(exit) = &self.exit {
            let exit = interpolate_or_record(exit, variables, diagnostics);
            match id {
                ObjectId::Stage => {}
                ObjectId::Background => {
                    effects.push(StageEffect::SetBackgroundExitAnimation(exit.to_string()))
                }
                ObjectId::Figure(id) => effects.push(StageEffect::SetFigureExitAnimation(
                    interpolate_or_record(id, variables, diagnostics).to_string(),
                    exit.to_string(),
                )),
            }
        }
    }
}

// -------- 特殊演出 --------

impl ToEffects for PixiPerformSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        effects.push(StageEffect::SetStagePixiEffect(
            interpolate_or_record(&self.effect, variables, diagnostics).to_string(),
        ));
    }
}

impl ToEffects for PixiInitSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        _variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        _diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        effects.push(StageEffect::RemoveStagePixiEffect);
    }
}

impl ToEffects for IntroSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        _effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        for text in &self.content {
            interpolate_or_record(text, variables, diagnostics);
        }
    }
}

impl ToEffects for MiniAvatarSentence {}

impl ToEffects for SetTextboxSentence {
    fn extend_effects<'a, P: ProjectView<'a>>(
        &self,
        _variables: &HashMap<String, Value>,
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        _diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        effects.push(StageEffect::SetTextboxVisibility(self.show));
    }
}

impl ToEffects for FilmModeSentence {}

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

/// 字符串变量插值
///
/// # Behavior
/// * 插值策略详见 [`interpolate`].
/// * 插值失败时, 将记录诊断并返回原始值的引用.
fn interpolate_or_record<'a>(
    input: &'a str,
    variables: &HashMap<String, Value>,
    diagnostics: &mut Vec<DiagnosticKind>,
) -> Cow<'a, str> {
    interpolate(input, variables).unwrap_or_else(|error| {
        diagnostics.push(error);
        Cow::Borrowed(input)
    })
}

fn make_transform_effect(
    transform: Box<Transform>,
    id: &ObjectId,
    variables: &HashMap<String, Value>,
    diagnostics: &mut Vec<DiagnosticKind>,
) -> StageEffect {
    match id {
        ObjectId::Stage => StageEffect::SetStageTransform(transform),
        ObjectId::Background => StageEffect::SetBackgroundTransform(transform),
        ObjectId::Figure(id) => StageEffect::SetFigureTransform(
            interpolate_or_record(id, variables, diagnostics).to_string(),
            transform,
        ),
    }
}

fn merge_animations(animations: &AnimationList, write_default: bool) -> Transform {
    if write_default {
        let mut transform = Transform::default_values();
        for animation in animations.iter() {
            transform.merge(&animation.transform);
        }
        transform
    } else {
        animations.merge_all().transform
    }
}
