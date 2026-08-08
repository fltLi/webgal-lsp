//! 舞台状态变换

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, hash_map::Entry},
    rc::Rc,
    result,
};

use serde_json::Value;
use webgal_language_core::{
    dispatch_sentence,
    element::{ObjectId, TokenSplit},
    sentence::*,
};

use crate::{DiagnosticKind, ProjectView, SymbolKind, scene::Project, state::stage::*};

// TODO: 检查 Transform 合并有效性

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
    SetDialogueContent(String),
    AddDialogueContent(String),
    SetDialogueFigure(String),
    SetTextboxVisibility(bool),

    // 背景
    SetBackground(String),
    RemoveBackground,
    SetBackgroundTransform,
    SetBackgroundExitAnimation(String),

    // 立绘
    SetFigure(String, String),
    RemoveFigure(String),
    SetFigureTransform(String),
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
        match self {
            // 对话
            Self::SetDialogueSpeaker(speaker) => {
                if let Some(textbox) = Rc::get_mut(&mut next_stage.textbox) {
                    // 进入连续执行块后状态所属分类已更新, 现在要判定该状态是否更新
                    if speaker == textbox.speaker {
                        // 状态在同一连续执行块内发生重复更新, 应当诊断为重复警告
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置对话者为 `{speaker}`",
                        )));
                    } else if speaker == prev_stage.textbox.speaker {
                        // 状态在同一连续执行块内多次更新且最终换回进入连续执行块前的状态, 应诊断为重复警告
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置对话者为 `{speaker}` (原来是 `{}`, 现在还原到了进入连续执行块前的状态)",
                            textbox.speaker,
                        )));
                        textbox.speaker = speaker;
                    } else if textbox.speaker != prev_stage.textbox.speaker {
                        // 状态在同一连续执行块内多次更新, 应当诊断为重复且覆盖警告
                        diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                            "设置对话者为 `{speaker}` (原来是 `{}`)",
                            textbox.speaker,
                        )));
                        textbox.speaker = speaker;
                    } else {
                        textbox.speaker = speaker;
                    }
                } else {
                    // 基于进入连续执行块前的原始状态创建新的状态副本并更新
                    Rc::make_mut(&mut next_stage.textbox).speaker = speaker;
                }
            }

            Self::SetDialogueContent(content) => {
                if let Some(textbox) = Rc::get_mut(&mut next_stage.textbox) {
                    if content == textbox.content {
                        if !content.is_empty() {
                            diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                                "设置对话内容为 `{content}`",
                            )));
                        }
                    } else if content == prev_stage.textbox.content {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置对话内容为 `{content}` (原来是 `{}`, 但现在还原到了进入连续执行块前的状态)",
                            textbox.content,
                        )));
                        textbox.content = content;
                    } else if textbox.content != prev_stage.textbox.content {
                        diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                            "设置对话内容为 `{content}` (原来是 `{}`)",
                            textbox.content,
                        )));
                        textbox.content = content;
                    } else {
                        textbox.content = content;
                    }
                } else {
                    Rc::make_mut(&mut next_stage.textbox).content = content;
                }

                // 检查对话长度
                validate_dialogue_length(&next_stage.textbox.content, diagnostics);
            }

            Self::AddDialogueContent(content) => {
                let content = format!("{}{content}", next_stage.textbox.content);

                if let Some(textbox) = Rc::get_mut(&mut next_stage.textbox) {
                    if content == textbox.content {
                        if !content.is_empty() {
                            diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                                "追加对话内容为 `{content}`",
                            )));
                        }
                    } else if content == prev_stage.textbox.content {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "追加对话内容为 `{content}` (原来是 `{}`, 但现在还原到了进入连续执行块前的状态)",
                            textbox.content,
                        )));
                        textbox.content = content;
                    } else if textbox.content != prev_stage.textbox.content {
                        diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                            "追加对话内容为 `{content}` (原来是 `{}`)",
                            textbox.content,
                        )));
                        textbox.content = content;
                    } else {
                        textbox.content = content;
                    }
                } else {
                    Rc::make_mut(&mut next_stage.textbox).content = content;
                }

                // 检查对话长度
                validate_dialogue_length(&next_stage.textbox.content, diagnostics);
            }

            Self::SetDialogueFigure(id) => {
                // 校验立绘是否存在
                if !next_stage.figures.contains_key(&id) {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(SymbolKind::Figure, id));
                }
            }

            Self::SetTextboxVisibility(visible) => {
                if let Some(textbox) = Rc::get_mut(&mut next_stage.textbox) {
                    if visible == textbox.visible {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置对话框可见性为 `{visible}`",
                        )));
                    } else if visible == prev_stage.textbox.visible {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置对话框可见性为 `{visible}` (原来是 `{}`, 但现在还原到了进入连续执行块前的状态)",
                            textbox.visible,
                        )));
                        textbox.visible = visible;
                    } else if textbox.visible != prev_stage.textbox.visible {
                        diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                            "设置对话框可见性为 `{visible}` (原来是 `{}`)",
                            textbox.visible,
                        )));
                        textbox.visible = visible;
                    } else {
                        textbox.visible = visible;
                    }
                } else {
                    Rc::make_mut(&mut next_stage.textbox).visible = visible;
                }
            }

            // 背景
            Self::SetBackground(path) => {
                if let Some(ref mut background) = next_stage.background {
                    if path == background.path {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景为 `{path}`",
                        )));
                    } else if prev_stage
                        .background
                        .as_ref()
                        .is_some_and(|background| path == background.path)
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景为 `{path}` (原来为 `{}`, 但现在还原到了进入连续执行块前的状态)",
                            background.path,
                        )));
                        *background = Rc::new(Background::new(path));
                    } else if prev_stage
                        .background
                        .as_ref()
                        .is_some_and(|prev_background| !Rc::ptr_eq(background, prev_background))
                    {
                        diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                            "设置背景为 `{path}` (原来是 `{}`)",
                            background.path,
                        )));
                        *background = Rc::new(Background::new(path));
                    } else {
                        *background = Rc::new(Background::new(path));
                    }
                } else {
                    if prev_stage
                        .background
                        .as_ref()
                        .is_some_and(|background| path == background.path)
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景为 `{path}` (原来为空, 但现在还原到了进入连续执行块前的状态)",
                        )));
                    }
                    next_stage.background = Some(Rc::new(Background::new(path)));
                }
            }

            Self::RemoveBackground => {
                if let Some(background) = next_stage.background.take() {
                    if prev_stage.background.is_none() {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景为空 (原来为 `{}`, 但现在还原到了进入连续执行块前的状态)",
                            background.path,
                        )));
                    }
                } else {
                    diagnostics.push(DiagnosticKind::RedundantEffect(
                        "设置背景为空 (原来为空)".to_string(),
                    ));
                }
            }

            Self::SetBackgroundTransform => {
                if next_stage.background.is_none() {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(
                        SymbolKind::Background,
                        "bg-main".to_string(),
                    ));
                }
            }

            Self::SetBackgroundExitAnimation(exit) => {
                if let Some(ref mut background) = next_stage.background {
                    if background
                        .exit
                        .as_ref()
                        .is_some_and(|next_exit| *exit == *next_exit)
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景退场动画为 `{exit}`",
                        )));
                    } else if prev_stage
                        .background
                        .as_ref()
                        .is_some_and(|prev_background| {
                            prev_background.path == background.path
                                && prev_background
                                    .exit
                                    .as_ref()
                                    .is_some_and(|prev_exit| *exit == *prev_exit)
                        })
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景退场动画为 `{exit}` (还原到了进入连续执行块前的状态)",
                        )));
                    }
                    Rc::make_mut(background).exit = Some(exit);
                } else {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(
                        SymbolKind::Background,
                        "bg-main".to_string(),
                    ));
                }
            }

            // 立绘
            Self::SetFigure(id, path) => match next_stage.figures.entry(id.clone()) {
                Entry::Occupied(mut o) => {
                    if path == o.get().path {
                        if Rc::get_mut(o.get_mut()).is_some() {
                            diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                                "设置立绘 `{id}` 为 `{path}`",
                            )));
                        }
                    } else if let Some(figure) = prev_stage.figures.get(&id)
                        && path == figure.path
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘 `{id}` 为 `{path}` (原来为 `{}`, 但现在还原到了进入连续执行块前的状态)",
                            o.get().path,
                        )));
                        Rc::make_mut(o.get_mut()).path = path;
                    } else {
                        Rc::make_mut(o.get_mut()).path = path;
                    }
                }
                Entry::Vacant(v) if prev_stage.figures.contains_key(&id) => {
                    diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘 `{id}` 为 `{path}` (原来为空, 建议移除上文无意义的立绘退场, 改为直接切换)",
                        )));
                    v.insert(Rc::new(Figure::new(path)));
                }
                Entry::Vacant(v) => {
                    v.insert(Rc::new(Figure::new(path)));
                }
            },

            Self::RemoveFigure(id) => {
                if let Some(figure) = next_stage.figures.remove(&id) {
                    if let Some(prev_figure) = prev_stage.figures.get(&id) {
                        if figure.path != prev_figure.path {
                            diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                                "设置立绘 `{id}` 为空 (原来为 `{}`)",
                                figure.path,
                            )));
                        }
                    } else {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘 `{id}` 为空 (原来为 `{}`, 但现在还原到了进入连续执行块前的状态)",
                            figure.path,
                        )));
                    }
                } else {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(SymbolKind::Figure, id));
                }
            }

            Self::SetFigureTransform(id) => {
                if !next_stage.figures.contains_key(&id) {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(SymbolKind::Figure, id));
                }
            }

            Self::SetFigureExitAnimation(id, exit) => {
                if let Some(figure) = next_stage.figures.get_mut(&id) {
                    if figure
                        .exit
                        .as_ref()
                        .is_some_and(|next_exit| *exit == *next_exit)
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘退场动画为 `{exit}`",
                        )));
                    } else if prev_stage.figures.get(&id).is_some_and(|prev_figure| {
                        prev_figure.path == figure.path
                            && prev_figure
                                .exit
                                .as_ref()
                                .is_some_and(|prev_exit| *exit == *prev_exit)
                    }) {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘退场动画为 `{exit}` (还原到了进入连续执行块前的状态)",
                        )));
                    }
                    Rc::make_mut(figure).exit = Some(exit);
                } else {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(SymbolKind::Figure, id));
                }
            }

            Self::SetFigureMotion(id, motion) => {
                if let Some(figure) = next_stage.figures.get_mut(&id) {
                    if figure
                        .motion
                        .as_ref()
                        .is_some_and(|next_motion| *motion == *next_motion)
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘动作为 `{motion}`",
                        )));
                    } else if prev_stage.figures.get(&id).is_some_and(|prev_figure| {
                        prev_figure.path == figure.path
                            && prev_figure
                                .motion
                                .as_ref()
                                .is_some_and(|prev_motion| *motion == *prev_motion)
                    }) {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘动作为 `{motion}` (还原到了进入连续执行块前的状态)",
                        )));
                    }
                    Rc::make_mut(figure).motion = Some(motion);
                } else {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(SymbolKind::Figure, id));
                }
            }

            Self::SetFigureExpression(id, expression) => {
                if let Some(figure) = next_stage.figures.get_mut(&id) {
                    if figure
                        .expression
                        .as_ref()
                        .is_some_and(|next_expression| *expression == *next_expression)
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘 Live2D 表情为 `{expression}`",
                        )));
                    } else if prev_stage.figures.get(&id).is_some_and(|prev_figure| {
                        prev_figure.path == figure.path
                            && prev_figure
                                .expression
                                .as_ref()
                                .is_some_and(|prev_expression| *expression == *prev_expression)
                    }) {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置立绘 Live2D 表情为 `{expression}` (还原到了进入连续执行块前的状态)",
                        )));
                    }
                    Rc::make_mut(figure).expression = Some(expression);
                } else {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(SymbolKind::Figure, id));
                }
            }

            // 背景音乐
            Self::SetBgm(path) => {
                if let Some(ref mut bgm) = next_stage.bgm {
                    if path == **bgm {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景音乐为 `{path}`",
                        )));
                    } else if prev_stage.bgm.as_ref().is_some_and(|bgm| path == **bgm) {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景音乐为 `{path}` (原来为 `{bgm}`, 但现在还原到了进入连续执行块前的状态)",
                        )));
                        *bgm = Rc::new(path);
                    } else if prev_stage
                        .bgm
                        .as_ref()
                        .is_some_and(|prev_bgm| !Rc::ptr_eq(bgm, prev_bgm))
                    {
                        diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                            "设置背景音乐为 `{path}` (原来是 `{bgm}`)",
                        )));
                        *bgm = Rc::new(path);
                    } else {
                        *bgm = Rc::new(path);
                    }
                } else {
                    if prev_stage.bgm.as_ref().is_some_and(|bgm| path == **bgm) {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景音乐为 `{path}` (原来为空, 但现在还原到了进入连续执行块前的状态)",
                        )));
                    }
                    next_stage.bgm = Some(Rc::new(path));
                }
            }

            Self::RemoveBgm => {
                if let Some(bgm) = next_stage.bgm.take() {
                    if prev_stage.bgm.is_none() {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置背景音乐为空 (原来为 `{bgm}`, 但现在还原到了进入连续执行块前的状态)",
                        )));
                    }
                } else {
                    diagnostics.push(DiagnosticKind::RedundantEffect(
                        "设置背景音乐为空 (原来为空)".to_string(),
                    ));
                }
            }

            // 效果音
            Self::SetLoopingSound(id, path) => match next_stage.sounds.entry(id.clone()) {
                Entry::Occupied(mut o) => {
                    if path == **o.get() {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置效果音 `{id}` 为 `{path}`",
                        )));
                    } else if let Some(prev_sound) = prev_stage.sounds.get(&id)
                        && path == **prev_sound
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置效果音 `{id}` 为 `{path}` (原来为 `{}`, 但现在还原到了进入连续执行块前的状态)",
                            **o.get(),
                        )));
                        o.insert(Rc::new(path));
                    } else {
                        diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                            "设置效果音 `{id}` 为 `{path}` (原来为 `{}`)",
                            **o.get(),
                        )));
                        o.insert(Rc::new(path));
                    }
                }
                Entry::Vacant(v) => {
                    if let Some(prev_sound) = prev_stage.sounds.get(&id)
                        && path == **prev_sound
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置效果音 `{id}` 为 `{path}` (原来为空, 现在还原到了进入连续执行块前的状态)",
                        )));
                    }
                    v.insert(Rc::new(path));
                }
            },

            Self::RemoveLoopingSound(id) => {
                if let Some(sound) = next_stage.sounds.remove(&id) {
                    if !prev_stage.sounds.contains_key(&id) {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置效果音 `{id}` 为空 (原来为 `{sound}`, 但现在还原到了进入连续执行块前的状态)",
                        )));
                    }
                } else {
                    diagnostics.push(DiagnosticKind::UndefinedSymbol(SymbolKind::Sound, id));
                }
            }

            Self::RemoveAllLoopingSound => {
                if !next_stage.sounds.is_empty() && prev_stage.sounds.is_empty() {
                    diagnostics.push(DiagnosticKind::RedundantEffect(
                        "清空效果音 (还原到了进入连续执行块前的状态)".to_string(),
                    ));
                }
                next_stage.sounds.clear();
            }

            // 舞台效果
            Self::SetStagePixiEffect(pixi) => {
                if let Some(ref mut next_pixi) = next_stage.pixi {
                    if pixi == **next_pixi {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置舞台 Pixi 特效为 `{pixi}`",
                        )));
                    } else if prev_stage
                        .pixi
                        .as_ref()
                        .is_some_and(|prev_pixi| pixi == **prev_pixi)
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置舞台 Pixi 特效为 `{pixi}` (原来为 `{next_pixi}`, 但现在还原到了进入连续执行块前的状态)",
                        )));
                        *next_pixi = Rc::new(pixi);
                    } else if prev_stage
                        .pixi
                        .as_ref()
                        .is_some_and(|prev_pixi| !Rc::ptr_eq(next_pixi, prev_pixi))
                    {
                        diagnostics.push(DiagnosticKind::OverriddenEffect(format!(
                            "设置舞台 Pixi 特效为 `{pixi}` (原来是 `{next_pixi}`)",
                        )));
                        *next_pixi = Rc::new(pixi);
                    } else {
                        *next_pixi = Rc::new(pixi);
                    }
                } else {
                    if prev_stage
                        .pixi
                        .as_ref()
                        .is_some_and(|prev_pixi| pixi == **prev_pixi)
                    {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置舞台 Pixi 特效为 `{pixi}` (原来为空, 但现在还原到了进入连续执行块前的状态)",
                        )));
                    }
                    next_stage.pixi = Some(Rc::new(pixi));
                }
            }

            Self::RemoveStagePixiEffect => {
                if let Some(pixi) = next_stage.pixi.take() {
                    if prev_stage.pixi.is_none() {
                        diagnostics.push(DiagnosticKind::RedundantEffect(format!(
                            "设置舞台 Pixi 特效为空 (原来为 `{pixi}`, 但现在还原到了进入连续执行块前的状态)",
                        )));
                    }
                } else {
                    diagnostics.push(DiagnosticKind::RedundantEffect(
                        "设置舞台 Pixi 特效为空 (原来为空)".to_string(),
                    ));
                }
            }
        }
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
            effects.push(StageEffect::SetDialogueSpeaker(
                interpolate_or_record(speaker, variables, diagnostics).to_string(),
            ));
        }

        let content = itertools::intersperse(
            self.content.iter().map(|text| {
                TokenSplit::new(text)
                    .map(|token| interpolate_or_record(token.text, variables, diagnostics))
                    .collect()
            }),
            "|".to_string(),
        )
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

        // if let Some(transform) = &self.transform {
        //     effects.push(StageEffect::SetBackgroundTransform(Box::new(
        //         transform.clone(),
        //     )));
        // }
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

        // if let Some(transform) = &self.transform {
        //     effects.push(StageEffect::SetFigureTransform(
        //         id.to_string(),
        //         Box::new(transform.clone()),
        //     ));
        // }
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
        _project: &Project<'a, P>,
        effects: &mut Vec<StageEffect>,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        let id = match &self.target {
            Some(v) => v,
            None => return,
        };

        // let transform = match project.view().get_animation(&self.animation) {
        //     Some(animations) => Box::new(merge_animations(animations, self.write_default)),
        //     None => return,
        // };

        // effects.push(make_transform_effect(transform, id, variables, diagnostics));
        if let Some(effect) = make_transform_effect(id, variables, diagnostics) {
            effects.push(effect);
        }
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

        // let mut transform = Box::new(self.transform.clone());
        // if self.write_default {
        //     transform.merge(&Transform::default_values());
        // }

        // effects.push(make_transform_effect(transform, id, variables, diagnostics));
        if let Some(effect) = make_transform_effect(id, variables, diagnostics) {
            effects.push(effect);
        }
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

        // let transform = Box::new(merge_animations(&self.animation, self.write_default));

        // effects.push(make_transform_effect(transform, id, variables, diagnostics));
        if let Some(effect) = make_transform_effect(id, variables, diagnostics) {
            effects.push(effect);
        }
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

// fn make_transform_effect(
//     transform: Box<Transform>,
//     id: &ObjectId,
//     variables: &HashMap<String, Value>,
//     diagnostics: &mut Vec<DiagnosticKind>,
// ) -> StageEffect {
//     match id {
//         ObjectId::Stage => StageEffect::SetStageTransform(transform),
//         ObjectId::Background => StageEffect::SetBackgroundTransform(transform),
//         ObjectId::Figure(id) => StageEffect::SetFigureTransform(
//             interpolate_or_record(id, variables, diagnostics).to_string(),
//             transform,
//         ),
//     }
// }
fn make_transform_effect(
    id: &ObjectId,
    variables: &HashMap<String, Value>,
    diagnostics: &mut Vec<DiagnosticKind>,
) -> Option<StageEffect> {
    match id {
        ObjectId::Background => Some(StageEffect::SetBackgroundTransform),
        ObjectId::Figure(id) => Some(StageEffect::SetFigureTransform(
            interpolate_or_record(id, variables, diagnostics).to_string(),
        )),
        ObjectId::Stage => None,
    }
}

// fn merge_animations(animations: &AnimationList, write_default: bool) -> Transform {
//     if write_default {
//         let mut transform = Transform::default_values();
//         for animation in animations.iter() {
//             transform.merge(&animation.transform);
//         }
//         transform
//     } else {
//         animations.merge_all().transform
//     }
// }

/// 校验对话长度是否超过最大行数
///
/// # Behavior
/// * `|` 视为换行符.
/// * 每 30 个逻辑字符数发生一次自动换行.
/// * 最大行数为 2.
fn validate_dialogue_length(content: &str, diagnostics: &mut Vec<DiagnosticKind>) {
    let total_lines = content
        .split('|')
        .map(|line| line.chars().count().div_ceil(30))
        .sum();
    if total_lines > 2 {
        diagnostics.push(DiagnosticKind::DialogueTooLong(total_lines));
    }
}
