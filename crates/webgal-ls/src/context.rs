use path_tree::{Entry, Folder, Node, canonicalize};
use ranked_count::Counter;
use webgal_model::{
    resource::{
        Config, FigureInfo, FigureKind, Live2dModel, WmdlModel, WmdlSubModel, figure_type_of,
    },
    sentence::*,
};

/// WebGAL 单项目信息
#[derive(Debug, Default)]
pub struct Context {
    pub resource: Resource,
    pub ident: IdentTable,
}

impl Context {
    pub fn new() -> Self {
        Self {
            resource: Resource::new(),
            ident: IdentTable::new(),
        }
    }

    pub fn update_scene(&mut self, path: &str, scene: Scene) -> anyhow::Result<()> {
        let entry = self.resource.scene.entry(path).map_err(|(last, _)| {
            anyhow::anyhow!("尝试修改位于 `{path}` 的场景, 但路径上已存在场景 `{last}`")
        })?;
        // 记录符号
        for SentenceInfo { sentence, .. } in scene.sentences() {
            self.ident.insert(sentence);
        }
        // 移除符号
        if let Entry::Occupied(o) = &entry
            && let Node::Item(scene) = o.get()
        {
            for SentenceInfo { sentence, .. } in scene.sentences() {
                self.ident.remove(sentence);
            }
        }
        // 添加场景
        entry.insert_entry(Node::Item(scene));
        Ok(())
    }
}

/// 配置和资源
#[derive(Debug, Default)]
pub struct Resource {
    pub config: Config,
    // 场景
    pub scene: Folder<Scene>,
    // 动画
    pub animation: Folder<()>,
    // 立绘和图像
    pub background: Folder<()>,
    pub figure: Folder<FigureInfo>,
    // 音视频
    pub bgm: Folder<()>,
    pub vocal: Folder<()>,
    pub video: Folder<()>,
}

impl Resource {
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入 / 修改立绘文件
    pub fn insert_figure(&mut self, path: &str, data: &str) -> anyhow::Result<()> {
        let (path, kind) = figure_type_of(path);
        let info = match kind {
            FigureKind::Live2d => {
                let model: Live2dModel = serde_json::from_str(data)?;
                FigureInfo::from_live2d(&model)
            }
            FigureKind::Wmdl => {
                let WmdlModel { sub_models, .. } = serde_json::from_str(data)?;
                let mut info = FigureInfo::new();
                // 逐一添加子模型
                for FigureInfo {
                    motions,
                    expressions,
                    ..
                } in sub_models.iter().filter_map(|WmdlSubModel { model, .. }| {
                    self.figure.get(model).and_then(Node::as_item)
                }) {
                    info.extend_motions(motions);
                    info.extend_expressions(expressions);
                }
                info
            }
            _ => Default::default(),
        };
        // 加入模型
        self.figure.insert(path, Node::Item(info));
        Ok(())
    }

    pub fn contains_animation(&self, animation: &str) -> bool {
        let animation = canonicalize(animation).unwrap_or(animation.to_string());
        self.animation.contains(&format!("{animation}.json"))
    }
}

/// 符号表
///
/// 同步维护, 用于自动补全
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentTable {
    pub id: Counter<String>,
    pub speaker: Counter<String>,
    pub label: Counter<String>,
    pub series: Counter<String>,
    pub duration: Counter<usize>,
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

    /// 记录一条语句的符号
    pub fn insert(&mut self, sentence: &Sentence) {
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

    /// 移除一条语句的符号
    pub fn remove(&mut self, sentence: &Sentence) {
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
    Duration(usize),
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
