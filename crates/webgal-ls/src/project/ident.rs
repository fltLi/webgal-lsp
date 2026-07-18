//! 全场景符号收集与维护

use count::HashCounter;
use webgal_language_core::sentence::*;

/// 场景符号表
///
/// 场景更改时实时维护, 用于记录全局符号信息.
#[derive(Debug, Clone, Default)]
pub struct IdentTable {
    pub id: HashCounter<String>,
    pub speaker: HashCounter<String>,
    pub label: HashCounter<String>,
    pub series: HashCounter<String>,
    pub duration: HashCounter<u32>,
}

impl IdentTable {
    /// 创建带有预设符号的符号表
    pub fn new() -> Self {
        Self {
            id: [
                "stage-main",
                "bg-main",
                "fig-center",
                "fig-left",
                "fig-right",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            duration: [800, 1200, 1600, 1700, 2400].into_iter().collect(),
            ..Default::default()
        }
    }

    /// 记录一个场景的符号
    pub fn insert_scene(&mut self, scene: &Scene) {
        for SentenceInfo { sentence, .. } in scene.sentences() {
            self.insert_sentence(sentence);
        }
    }

    /// 记录一条语句的符号
    pub fn insert_sentence(&mut self, sentence: &Sentence) {
        ident_of(sentence, |ident| {
            match ident {
                IdentKind::Id(id) => self.id.insert(id.to_string()),
                IdentKind::Speaker(speaker) => self.speaker.insert(speaker.to_string()),
                IdentKind::Label(label) => self.label.insert(label.to_string()),
                IdentKind::Series(series) => self.series.insert(series.to_string()),
                IdentKind::Duration(duration) => self.duration.insert(duration),
            };
        });
    }

    /// 移除一个场景的符号
    pub fn remove_scene(&mut self, scene: &Scene) {
        for SentenceInfo { sentence, .. } in scene.sentences() {
            self.remove_sentence(sentence);
        }
    }

    /// 移除一条语句的符号
    pub fn remove_sentence(&mut self, sentence: &Sentence) {
        ident_of(sentence, |ident| {
            match ident {
                IdentKind::Id(id) => self.id.remove(&id),
                IdentKind::Speaker(speaker) => self.speaker.remove(&speaker),
                IdentKind::Label(label) => self.label.remove(&label),
                IdentKind::Series(series) => self.series.remove(&series),
                IdentKind::Duration(duration) => self.duration.remove(&duration),
            };
        });
    }
}

enum IdentKind {
    Id(String),
    Speaker(String),
    Label(String),
    Series(String),
    Duration(u32),
}

fn ident_of<F>(sentence: &Sentence, mut f: F)
where
    F: FnMut(IdentKind),
{
    use IdentKind::*;
    use Sentence::*;

    match sentence {
        // 常规演出
        Say(SaySentence {
            speaker, figure, ..
        }) => {
            speaker.clone().map(Speaker).map(&mut f);
            if let Some(figure) = figure {
                f(Id(figure.get_id().to_string()));
            }
        }
        ChangeBackground(sentence) => {
            let ChangeBackgroundSentence {
                series,
                duration,
                enter_duration,
                exit_duration,
                ..
            } = &**sentence;
            series.clone().map(Series).map(&mut f);
            duration.map(Duration).map(&mut f);
            enter_duration.map(Duration).map(&mut f);
            exit_duration.map(Duration).map(&mut f);
        }
        ChangeFigure(sentence) => {
            let ChangeFigureSentence {
                id,
                duration,
                enter_duration,
                exit_duration,
                ..
            } = &**sentence;
            id.clone().map(Id).map(&mut f);
            duration.map(Duration).map(&mut f);
            enter_duration.map(Duration).map(&mut f);
            exit_duration.map(Duration).map(&mut f);
        }
        Bgm(BgmSentence { enter, series, .. }) => {
            enter.map(Duration).map(&mut f);
            series.clone().map(Series).map(&mut f);
        }
        PlayEffect(PlayEffectSentence { id: Some(id), .. }) => f(Id(id.clone())),

        // 舞台对象控制
        SetAnimation(SetAnimationSentence {
            target: Some(target),
            ..
        })
        | SetTempAnimation(SetTempAnimationSentence {
            target: Some(target),
            ..
        })
        | SetTransition(SetTransitionSentence {
            target: Some(target),
            ..
        }) => f(Id(target.get_id().to_string())),
        SetComplexAnimation(SetComplexAnimationSentence {
            target, duration, ..
        }) => {
            if let Some(target) = target {
                f(Id(target.get_id().to_string()));
            }
            duration.map(Duration).map(&mut f);
        }
        SetTransform(transform) => {
            let SetTransformSentence {
                target, duration, ..
            } = &**transform;
            if let Some(target) = target {
                f(Id(target.get_id().to_string()));
            }
            duration.map(Duration).map(&mut f);
        }

        // 特殊演出
        Intro(IntroSentence {
            delay: Some(delay), ..
        }) => f(Duration(*delay)),

        // 场景与分支
        Sentence::Label(LabelSentence { label, .. })
        | JumpLabel(JumpLabelSentence { label, .. }) => f(IdentKind::Label(label.clone())),

        // 鉴赏
        UnlockCg(UnlockCgSentence {
            series: Some(series),
            ..
        })
        | UnlockBgm(UnlockBgmSentence {
            series: Some(series),
            ..
        }) => f(Series(series.clone())),
        // 游戏控制
        Wait(WaitSentence { duration, .. }) => f(Duration(*duration)),

        _ => {}
    }
}
