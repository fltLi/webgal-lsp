//! 舞台状态变换

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serde_json::Value;
use webgal_language_core::{dispatch_sentence, sentence::*};

use crate::{DiagnosticKind, ProjectView, scene::Project, state::stage::Stage};

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
        let effects = StageEffect::from_sentence(sentence, variables, project);
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
    // TODO: 具体效果变体待补充
}

impl StageEffect {
    fn from_sentence<'a, P: ProjectView<'a>>(
        sentence: &Sentence,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<Self> {
        sentence.to_effects(variables, project)
    }

    /// 将单个舞台变换效果应用到目标状态, 同时返回诊断结果
    fn apply_to_stage(
        self,
        prev_stage: &Stage,
        next_stage: &mut Stage,
        diagnostics: &mut Vec<DiagnosticKind>,
    ) {
        match self {}
    }
}

trait ToEffects {
    /// 生成舞台变换效果
    #[allow(unused_variables)]
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        Default::default()
    }
}

impl ToEffects for Sentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        dispatch_sentence!(self.to_effects(variables, project))
    }
}

// -------- 常规演出 --------

impl ToEffects for SaySentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for ChangeBackgroundSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for ChangeFigureSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for BgmSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for PlayVideoSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for PlayEffectSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

// -------- 舞台对象控制 --------

impl ToEffects for SetAnimationSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for SetComplexAnimationSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for SetTransformSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for SetTempAnimationSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for SetTransitionSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

// -------- 特殊演出 --------

impl ToEffects for PixiPerformSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for PixiInitSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for IntroSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for MiniAvatarSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for SetTextboxSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

impl ToEffects for FilmModeSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
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

impl ToEffects for EndSentence {
    fn to_effects<'a, P: ProjectView<'a>>(
        &self,
        variables: &HashMap<String, Value>,
        project: &Project<'a, P>,
    ) -> Vec<StageEffect> {
        unimplemented!()
    }
}

// -------- 空白注释 --------

impl ToEffects for CommentSentence {}
