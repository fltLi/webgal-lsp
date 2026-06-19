use json_complete::{ToJsonSchema, Value};
use once_cell::sync::Lazy;
use path_tree::{Folder, Node, PATH_SEPARATORS};
use ranked_count::Counter;
use tower_lsp::lsp_types::*;
use webgal_model::{
    element::{
        AnimationList, BasicForward, FigureSide, KeepableForward, Live2dBlink, Live2dFocus,
        Transform,
    },
    resource::{FigureInfo, FigureKind},
    sentence::*,
};

use crate::{project::Project, service::complete::make_span};

/// 语句的代码补全服务
///
/// 为 [`webgal_model::sentence::Sentence`] 枚举项及其自身实现.
pub trait Complete {
    /// 补全主参数
    #[allow(unused_variables)]
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        Default::default()
    }

    /// 补全参数名
    #[allow(unused_variables)]
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        Default::default()
    }

    /// 补全参数值
    #[allow(unused_variables)]
    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        Default::default()
    }
}

impl Complete for Sentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        use Sentence::*;

        macro_rules! complete_content_match {
            (
                ($sentence:ident, $input:ident, $position:ident, $project:ident):
                {$($variant:ident),* $(,)?}
            ) => {{
                match $sentence {
                    $($variant(sentence) => sentence.complete_content($input, $position, $project),)*
                }
            }};
        }

        complete_content_match! {
            (self, input, position, project): {
                // 常规演出
                Say, ChangeBackground, ChangeFigure, Bgm, PlayVideo, PlayEffect,
                // 舞台对象控制
                SetAnimation, SetComplexAnimation, SetTransform, SetTempAnimation, SetTransition,
                // 特殊演出
                PixiPerform, PixiInit, Intro, MiniAvatar, SetTextbox, FilmMode,
                // 场景与分支
                CallScene, ChangeScene, Choose, Label, JumpLabel,
                // 鉴赏
                UnlockCg, UnlockBgm,
                // 游戏控制
                GetUserInput, SetVar, ShowVars, Wait, ApplyStyle, CallSteam, End,
                // 空白注释
                Comment,
            }
        }
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        use Sentence::*;

        macro_rules! complete_argument_name_match {
            (
                ($sentence:ident, $input:ident, $position:ident, $project:ident):
                {$($variant:ident),* $(,)?}
            ) => {{
                match $sentence {
                    $($variant(sentence) => sentence.complete_argument_name(
                        $input,
                        $position,
                        $project,
                    ),)*
                }
            }};
        }

        complete_argument_name_match! {
            (self, input, position, project): {
                // 常规演出
                Say, ChangeBackground, ChangeFigure, Bgm, PlayVideo, PlayEffect,
                // 舞台对象控制
                SetAnimation, SetComplexAnimation, SetTransform, SetTempAnimation, SetTransition,
                // 特殊演出
                PixiPerform, PixiInit, Intro, MiniAvatar, SetTextbox, FilmMode,
                // 场景与分支
                CallScene, ChangeScene, Choose, Label, JumpLabel,
                // 鉴赏
                UnlockCg, UnlockBgm,
                // 游戏控制
                GetUserInput, SetVar, ShowVars, Wait, ApplyStyle, CallSteam, End,
                // 空白注释
                Comment,
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        use Sentence::*;

        macro_rules! complete_argument_value_match {
            (
                ($sentence:ident, $name:ident, $input:ident, $position:ident, $project:ident):
                {$($variant:ident),* $(,)?}
            ) => {{
                match $sentence {
                    $($variant(sentence) => sentence.complete_argument_value(
                        $name,
                        $input,
                        $position,
                        $project,
                    ),)*
                }
            }};
        }

        complete_argument_value_match! {
            (self, name, input, position, project): {
                // 常规演出
                Say, ChangeBackground, ChangeFigure, Bgm, PlayVideo, PlayEffect,
                // 舞台对象控制
                SetAnimation, SetComplexAnimation, SetTransform, SetTempAnimation, SetTransition,
                // 特殊演出
                PixiPerform, PixiInit, Intro, MiniAvatar, SetTextbox, FilmMode,
                // 场景与分支
                CallScene, ChangeScene, Choose, Label, JumpLabel,
                // 鉴赏
                UnlockCg, UnlockBgm,
                // 游戏控制
                GetUserInput, SetVar, ShowVars, Wait, ApplyStyle, CallSteam, End,
                // 空白注释
                Comment,
            }
        }
    }
}

/// 补全文件目录
fn complete_file<T, F>(
    folder: &Folder<T>,
    describe: F,
    input: &str,
    position: Position,
) -> Vec<CompletionItem>
where
    F: Fn(&str, &T) -> (CompletionItemKind, String),
{
    let (parent, current) = input.rsplit_once(PATH_SEPARATORS).unwrap_or(("", input));
    let folder = if parent.is_empty() {
        folder
    } else {
        match folder.get(parent).and_then(Node::as_folder) {
            Some(folder) => folder,
            None => return Vec::default(),
        }
    };

    folder
        .iter()
        .filter(|(name, _)| name.starts_with(current))
        .map(|(name, node)| {
            let span = make_span(position, current.len());
            match node {
                Node::Item(item) => {
                    let (kind, description) = describe(name, item);
                    CompletionItem {
                        label: name.to_string(),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: Some(description),
                        }),
                        kind: Some(kind),
                        sort_text: Some(format!("b_{name}")),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                            range: span,
                            new_text: name.to_string(),
                        })),
                        ..Default::default()
                    }
                }
                Node::Folder(_) => CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FOLDER),
                    sort_text: Some(format!("a_{name}")),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range: span,
                        new_text: format!("{name}/"),
                    })),
                    ..Default::default()
                },
            }
        })
        .collect()
}

/// 补全枚举迭代器
fn complete_enum<I, N, D>(
    iter: I,
    kind: CompletionItemKind,
    input: &str,
    position: Position,
) -> Vec<CompletionItem>
where
    I: IntoIterator<Item = (N, D)>,
    N: AsRef<str>,
    D: AsRef<str>,
{
    iter.into_iter()
        .enumerate()
        .filter_map(|(i, (name, description))| {
            let name = name.as_ref();
            name.starts_with(input).then(|| CompletionItem {
                label: name.to_string(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: None,
                    description: Some(description.as_ref().to_string()),
                }),
                kind: Some(kind),
                sort_text: Some(format!("{i:03}_{name}")),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: make_span(position, input.len()),
                    new_text: name.to_string(),
                })),
                ..Default::default()
            })
        })
        .collect()
}

fn complete_image_figure_file(
    description: &str,
    input: &str,
    position: Position,
    project: &Project,
) -> Vec<CompletionItem> {
    complete_file(
        &project.resource().figure,
        |_, info| {
            (
                CompletionItemKind::FILE,
                matches!(info, FigureInfo::Image)
                    .then_some(description)
                    .unwrap_or_default()
                    .to_string(),
            )
        },
        input,
        position,
    )
}

fn complete_scene_file(input: &str, position: Position, project: &Project) -> Vec<CompletionItem> {
    complete_file(
        &project.resource().scene,
        |name, _| {
            (
                CompletionItemKind::FILE,
                if name.ends_with(".txt") {
                    "场景".to_string()
                } else {
                    Default::default()
                },
            )
        },
        input,
        position,
    )
}

fn complete_animation_enum(
    input: &str,
    position: Position,
    project: &Project,
) -> Vec<CompletionItem> {
    complete_enum(
        project
            .resource()
            .animation
            .iter()
            .map(|(name, _)| (name.strip_suffix(".json").unwrap_or(name), "动画")),
        CompletionItemKind::ENUM_MEMBER,
        input,
        position,
    )
}

fn complete_ident_enum<T: Ord + AsRef<str>>(
    ident: &Counter<T>,
    description: &str,
    input: &str,
    position: Position,
) -> Vec<CompletionItem> {
    complete_enum(
        ident.iter_by_count().map(|(name, _)| (name, description)),
        CompletionItemKind::VARIABLE,
        input,
        position,
    )
}

fn complete_duration_enum(
    description: &str,
    input: &str,
    position: Position,
    project: &Project,
) -> Vec<CompletionItem> {
    complete_enum(
        project
            .ident()
            .duration
            .iter_by_count()
            .map(|(name, _)| (name.to_string(), description)),
        CompletionItemKind::VARIABLE,
        input,
        position,
    )
}

fn complete_font_size_enum(input: &str, position: Position) -> Vec<CompletionItem> {
    complete_enum(
        [("small", "小号"), ("medium", "中号"), ("large", "大号")],
        CompletionItemKind::ENUM_MEMBER,
        input,
        position,
    )
}

fn complete_ease_enum(input: &str, position: Position) -> Vec<CompletionItem> {
    complete_enum(
        [
            ("linear", "线性"),
            ("easeIn", "缓入"),
            ("easeOut", "缓出"),
            ("easeInOut", "缓入缓出"),
            ("circIn", "圆形缓入"),
            ("circOut", "圆形缓出"),
            ("circInOut", "圆形缓入缓出"),
            ("backIn", "起点回弹"),
            ("backOut", "终点回弹"),
            ("backInOut", "起止回弹"),
            ("bounceIn", "起点弹跳"),
            ("bounceOut", "终点弹跳"),
            ("bounceInOut", "起止弹跳"),
            ("anticipate", "预先反向"),
        ],
        CompletionItemKind::ENUM_MEMBER,
        input,
        position,
    )
}

fn live2d_blink_json_schema() -> &'static Value {
    static SCHEMA: Lazy<Value> = Lazy::new(Live2dBlink::schema);
    &SCHEMA
}

fn live2d_focus_json_schema() -> &'static Value {
    static SCHEMA: Lazy<Value> = Lazy::new(Live2dFocus::schema);
    &SCHEMA
}

fn transform_json_schema() -> &'static Value {
    static SCHEMA: Lazy<Value> = Lazy::new(Transform::schema);
    &SCHEMA
}

fn animation_list_json_schema() -> &'static Value {
    static SCHEMA: Lazy<Value> = Lazy::new(AnimationList::schema);
    &SCHEMA
}

macro_rules! complete_argument_name_collect {
    (
        ($input:ident, $position:ident):
        {$($guard:expr => ($name:literal, $insert:literal, $description:literal)),* $(,)?}
    ) => {{
        let mut completions = Vec::new();
        $(
            if $guard && $name.starts_with($input)  {
                completions.push(CompletionItem {
                    label: $name.to_string(),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: Some($description.to_string()),
                    }),
                    kind: Some(CompletionItemKind::PROPERTY),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range: make_span($position, $input.len()),
                        new_text: $insert.to_string(),
                    })),
                    ..Default::default()
                });
            }
        )*
        completions
    }};
}

// -------- 常规演出 --------

impl Complete for SaySentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        // 补全参数
        let mut arguments = complete_argument_name_collect! {
            (input, position): {
                self.speaker.is_none() => ("speaker", "speaker=", "人物"),
                self.vocal.is_none() => ("vocal", "vocal=", "播放语音"),
                self.figure.is_none() => ("figureId", "figureId=", "指定立绘 ID"),
                self.figure.is_none() => ("center", "center", "指定中间立绘"),
                self.figure.is_none() => ("left", "left", "指定左侧立绘"),
                self.figure.is_none() => ("right", "right", "指定右侧立绘"),
                self.font_size == Default::default() => ("fontSize", "fontSize", "字体大小"),
                !self.concat => ("concat", "concat", "将该对话与上一句连接"),
                !self.notend => ("notend", "notend", "文字展示完执行下一句"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        };
        // 补全语音
        let mut vocal = complete_file(
            &project.resource().vocal,
            |_, _| (CompletionItemKind::FILE, "语音".to_string()),
            input,
            position,
        );

        arguments.append(&mut vocal);
        arguments
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "speaker" => complete_ident_enum(&project.ident().speaker, "人物", input, position),
            "vocal" => complete_file(
                &project.resource().vocal,
                |_, _| (CompletionItemKind::FILE, "语音".to_string()),
                input,
                position,
            ),
            "figureId" => complete_ident_enum(&project.ident().id, "立绘 ID", input, position),
            "fontSize" => complete_font_size_enum(input, position),
            _ => Vec::default(),
        }
    }
}

impl Complete for ChangeBackgroundSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_file(
            &project.resource().background,
            |_, _| (CompletionItemKind::FILE, "背景".to_string()),
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.transform.is_none() => ("transform", "transform=", "设置变换效果"),
                self.enter.is_none() => ("enter", "enter=", "入场动画"),
                self.exit.is_none() => ("exit", "exit=", "退场动画"),
                self.ease == Default::default() => ("ease", "ease=", "缓动类型"),
                self.unlockname.is_none() => ("unlockname", "unlockname=", "鉴赏解锁名称"),
                self.series.is_none() => ("series", "series=", "鉴赏系列名称"),
                self.duration.is_none() => ("duration", "duration=", "持续时间 (ms)"),
                self.enter_duration.is_none() => ("enterDuration", "enterDuration=", "入场时长"),
                self.exit_duration.is_none() => ("exitDuration", "exitDuration=", "退场时长"),
                self.forward != BasicForward::Continue => ("continue", "continue", "继续执行"),
                self.forward != BasicForward::Next => ("next", "next", "连续执行"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "transform" => transform_json_schema().complete_lsp(input, position),
            "enter" => complete_animation_enum(input, position, project),
            "exit" => complete_animation_enum(input, position, project),
            "ease" => complete_ease_enum(input, position),
            "series" => complete_ident_enum(&project.ident().series, "鉴赏系列", input, position),
            "duration" => complete_duration_enum("持续时间 (ms)", input, position, project),
            "enterDuration" => complete_duration_enum("持续时间 (ms)", input, position, project),
            "exitDuration" => complete_duration_enum("持续时间 (ms)", input, position, project),
            _ => Vec::default(),
        }
    }
}

impl Complete for ChangeFigureSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_file(
            &project.resource().figure,
            |_, info| {
                (
                    CompletionItemKind::FILE,
                    match info.get_type() {
                        FigureKind::Image => "图片立绘",
                        FigureKind::Spine => "Spine 立绘",
                        FigureKind::Live2d => "Live2D 立绘",
                        FigureKind::Wmdl => "WMDL 立绘",
                        FigureKind::Composite => "Composite 立绘",
                    }
                    .to_string(),
                )
            },
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.side != FigureSide::Left => ("left", "left", "将立绘置于左侧"),
                self.side != FigureSide::Right => ("right", "right", "将立绘置于右侧"),
                self.id.is_none() => ("id", "id=", "设置 ID"),
                self.mouth_open.is_none() => ("mouthOpen", "mouthOpen=", "嘴巴张开的图片立绘"),
                self.mouth_half_open.is_none() => ("mouthHalfOpen", "mouthHalfOpen=", "嘴巴半张开的图片立绘"),
                self.mouth_close.is_none() => ("mouthClose", "mouthClose=", "嘴巴闭上的图片立绘"),
                self.eyes_open.is_none() => ("eyesOpen", "eyesOpen=", "眼睛睁开的图片立绘"),
                self.eyes_close.is_none() => ("eyesClose", "eyesClose=", "眼睛闭上的图片立绘"),
                self.skin.is_none() => ("skin", "skin=", "Spine 皮肤"),
                self.motion.is_none() => ("motion", "motion=", "Live2D 动作"),
                self.expression.is_none() => ("expression", "expression=", "Live2D 表情"),
                self.bounds.is_none() => ("bounds", "bounds=", "Live2D 的边界"),
                self.blink.is_none() => ("blink", "blink=", "Live2D 立绘眨眼"),
                self.focus.is_none() => ("focus", "focus=", "Live2D 立绘注视"),
                self.transform.is_none() => ("transform", "transform=", "设置变换效果"),
                self.enter.is_none() => ("enter", "enter=", "入场动画"),
                self.exit.is_none() => ("exit", "exit=", "退场动画"),
                self.ease == Default::default() => ("ease", "ease=", "缓动类型"),
                self.z_index.is_none() => ("zIndex", "zIndex=", "图层排序 (大号在上)"),
                self.duration.is_none() => ("duration", "duration=", "持续时间 (ms)"),
                self.enter_duration.is_none() => ("enterDuration", "enterDuration=", "入场时长"),
                self.exit_duration.is_none() => ("exitDuration", "exitDuration=", "退场时长"),
                self.forward != BasicForward::Continue => ("continue", "continue", "继续执行"),
                self.forward != BasicForward::Next => ("next", "next", "连续执行"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "id" => complete_ident_enum(&project.ident().id, "立绘 ID", input, position),
            "mouthOpen" => complete_image_figure_file("图片立绘", input, position, project),
            "mouthHalfOpen" => complete_image_figure_file("图片立绘", input, position, project),
            "mouthClose" => complete_image_figure_file("图片立绘", input, position, project),
            "eyesOpen" => complete_image_figure_file("图片立绘", input, position, project),
            "eyesClose" => complete_image_figure_file("图片立绘", input, position, project),
            "skin" => complete_enum(
                [("default", "默认值")],
                CompletionItemKind::ENUM_MEMBER,
                input,
                position,
            ),
            "motion"
                if let Some(Node::Item(info)) = project.resource().figure.get(&self.figure) =>
            {
                let description = match info.get_type() {
                    FigureKind::Spine => "Spine 动作",
                    FigureKind::Live2d => "Live2D 动作",
                    _ => "立绘动作",
                };
                match info {
                    FigureInfo::Live2d { motions, .. } => complete_file(
                        motions,
                        |_, _| (CompletionItemKind::ENUM_MEMBER, description.to_string()),
                        input,
                        position,
                    ),
                    _ => Vec::default(),
                }
            }
            "expression"
                if let Some(Node::Item(FigureInfo::Live2d { expressions, .. })) =
                    project.resource().figure.get(&self.figure) =>
            {
                complete_file(
                    expressions,
                    |_, _| (CompletionItemKind::ENUM_MEMBER, "Live2D 表情".to_string()),
                    input,
                    position,
                )
            }
            "blink" => live2d_blink_json_schema().complete_lsp(input, position),
            "focus" => live2d_focus_json_schema().complete_lsp(input, position),
            "transform" => transform_json_schema().complete_lsp(input, position),
            "enter" => complete_animation_enum(input, position, project),
            "exit" => complete_animation_enum(input, position, project),
            "ease" => complete_ease_enum(input, position),
            "duration" => complete_duration_enum("持续时间 (ms)", input, position, project),
            "enterDuration" => complete_duration_enum("持续时间 (ms)", input, position, project),
            "exitDuration" => complete_duration_enum("持续时间 (ms)", input, position, project),
            _ => Vec::default(),
        }
    }
}

impl Complete for BgmSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_file(
            &project.resource().bgm,
            |_, _| (CompletionItemKind::FILE, "音乐".to_string()),
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.volume.is_none() => ("volume", "volume=", "音量大小 [0..100]"),
                self.enter.is_none() => ("enter", "enter=", "音量淡入淡出时长"),
                self.unlockname.is_none() => ("unlockname", "unlockname=", "鉴赏解锁名称"),
                self.series.is_none() => ("series", "series=", "鉴赏系列名称"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "enter" => complete_duration_enum("淡入淡出时间 (ms)", input, position, project),
            "series" => complete_ident_enum(&project.ident().series, "鉴赏系列", input, position),
            _ => Vec::default(),
        }
    }
}

impl Complete for PlayVideoSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_file(
            &project.resource().video,
            |_, _| (CompletionItemKind::FILE, "视频".to_string()),
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                !self.skip_off => ("skipOff", "skipOff", "禁止跳过"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for PlayEffectSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_file(
            &project.resource().vocal,
            |_, _| (CompletionItemKind::FILE, "语音".to_string()),
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.id.is_none() => ("id", "id=", "设置 ID"),
                self.volume.is_none() => ("volume", "volume=", "音量大小 [0..100]"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "id" => complete_ident_enum(&project.ident().id, "语音 ID", input, position),
            _ => Vec::default(),
        }
    }
}

// -------- 舞台对象控制 --------

impl Complete for SetAnimationSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        // complete_file(
        //     &project.resource().animation,
        //     |name| name.strip_suffix(".json").unwrap_or(name).to_string(),
        //     |_, _| "动画".to_string(),
        //     input,
        //     position,
        // )
        complete_animation_enum(input, position, project)
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.target.is_none() => ("target", "target=", "指定目标"),
                !self.write_default => ("writeDefault", "writeDefault", "补充默认值"),
                self.forward != KeepableForward::Continue => ("continue", "continue", "继续执行"),
                self.forward != KeepableForward::Next => ("next", "next", "连续执行"),
                self.forward != KeepableForward::Keep => ("keep", "keep", "跨语句动画"),
                self.forward != KeepableForward::Parallel => ("parallel", "parallel", "并行动画"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "target" => complete_ident_enum(&project.ident().id, "对象", input, position),
            _ => Vec::default(),
        }
    }
}

impl Complete for SetComplexAnimationSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_enum(
            [
                ("universalSoftIn", "通用透明度淡入"),
                ("universalSoftOut", "通用透明度淡出"),
            ],
            CompletionItemKind::ENUM_MEMBER,
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.target.is_none() => ("target", "target=", "指定目标"),
                !self.write_default => ("writeDefault", "writeDefault", "补充默认值"),
                self.duration.is_none() => ("duration", "duration=", "持续时间 (ms)"),
                self.forward != BasicForward::Continue => ("continue", "continue", "继续执行"),
                self.forward != BasicForward::Next => ("next", "next", "连续执行"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "target" => complete_ident_enum(&project.ident().id, "对象 ID", input, position),
            "duration" => complete_duration_enum("持续时间 (ms)", input, position, project),
            _ => Vec::default(),
        }
    }
}

impl Complete for SetTransformSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        transform_json_schema().complete_lsp(input, position)
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.target.is_none() => ("target", "target=", "指定目标"),
                !self.write_default => ("writeDefault", "writeDefault", "补充默认值"),
                self.ease == Default::default() => ("ease", "ease=", "缓动类型"),
                self.duration.is_none() => ("duration", "duration=", "持续时间 (ms)"),
                self.forward != KeepableForward::Continue => ("continue", "continue", "继续执行"),
                self.forward != KeepableForward::Next => ("next", "next", "连续执行"),
                self.forward != KeepableForward::Keep => ("keep", "keep", "跨语句动画"),
                self.forward != KeepableForward::Parallel => ("parallel", "parallel", "并行动画"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "target" => complete_ident_enum(&project.ident().id, "对象 ID", input, position),
            "ease" => complete_ease_enum(input, position),
            "duration" => complete_duration_enum("持续时间 (ms)", input, position, project),
            _ => Vec::default(),
        }
    }
}

impl Complete for SetTempAnimationSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        animation_list_json_schema().complete_lsp(input, position)
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.target.is_none() => ("target", "target=", "指定目标"),
                !self.write_default => ("writeDefault", "writeDefault", "补充默认值"),
                self.forward != KeepableForward::Continue => ("continue", "continue", "继续执行"),
                self.forward != KeepableForward::Next => ("next", "next", "连续执行"),
                self.forward != KeepableForward::Keep => ("keep", "keep", "跨语句动画"),
                self.forward != KeepableForward::Parallel => ("parallel", "parallel", "并行动画"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "target" => complete_ident_enum(&project.ident().id, "对象", input, position),
            _ => Vec::default(),
        }
    }
}

impl Complete for SetTransitionSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.target.is_none() => ("target", "target=", "指定目标"),
                self.enter.is_none() => ("enter", "enter=", "入场动画"),
                self.exit.is_none() => ("exit", "exit=", "退场动画"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "target" => complete_ident_enum(&project.ident().id, "对象", input, position),
            "enter" => complete_animation_enum(input, position, project),
            "exit" => complete_animation_enum(input, position, project),
            _ => Vec::default(),
        }
    }
}

// -------- 特殊演出 --------

impl Complete for PixiPerformSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_enum(
            [
                ("snow", "雪"),
                ("heavySnow", "大雪"),
                ("rain", "雨"),
                ("cherryBlossoms", "樱花"),
            ],
            CompletionItemKind::ENUM_MEMBER,
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for PixiInitSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for IntroSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.font_size == Default::default() => ("fontSize", "fontSize=", "字体大小"),
                self.font_color.is_none() => ("fontColor", "fontColor=rgba($1,$2,$3,$4)$0", "字体颜色"),
                self.background_color.is_none() => ("backgroundColor", "backgroundColor=rgba($1,$2,$3,$4)$0", "背景颜色"),
                self.background_image == Default::default() => ("backgroundImage", "backgroundImage=", "背景图片"),
                self.animation != Default::default() => ("animation", "animation=", "动画"),
                self.delay.is_none() => ("delayTime", "delayTime=", "延迟时间 (ms)"),
                !self.hold => ("hold", "hold", "结束后保持"),
                !self.use_forward => ("useForward", "useForward", "手动播放每行文本"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "fontSize" => complete_font_size_enum(input, position),
            "backgroundImage" => complete_file(
                &project.resource().background,
                |_, _| (CompletionItemKind::FILE, "背景".to_string()),
                input,
                position,
            ),
            "animation" => complete_enum(
                [
                    ("fadeIn", "透明度淡入"),
                    ("slideIn", "滑入"),
                    ("typingEffect", "打字机效果"),
                    ("pixelateEffect", "模糊"),
                    ("revealAnimation", "卷轴展开"),
                ],
                CompletionItemKind::ENUM_MEMBER,
                input,
                position,
            ),
            "delayTime" => complete_duration_enum("延迟时间 (ms)", input, position, project),
            _ => Vec::default(),
        }
    }
}

impl Complete for MiniAvatarSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_image_figure_file("小头像", input, position, project)
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for SetTextboxSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_enum(
            [("on", "显示"), ("hide", "隐藏")],
            CompletionItemKind::ENUM_MEMBER,
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for FilmModeSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_enum(
            [("enable", "开启"), ("none", "关闭")],
            CompletionItemKind::ENUM_MEMBER,
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

// -------- 场景与分支 --------

impl Complete for CallSceneSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_scene_file(input, position, project)
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for ChangeSceneSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_scene_file(input, position, project)
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for ChooseSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        // 提取最近一个选项
        let (_, choice) = input.rsplit_once('|').unwrap_or(("", input));
        let input = match choice.split_once(':') {
            Some((_, scene)) => scene,
            None => return Vec::default(),
        };

        // 补全场景
        let mut scene = complete_scene_file(input, position, project);
        // 补全标签
        let mut label = complete_ident_enum(&project.ident().label, "标签", input, position);
        scene.append(&mut label);
        scene
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.default_choice.is_none() => ("defaultChoice", "defaultChoice=", "快速预览默认选项"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "defaultChoice" => complete_enum(
                (1..=self.choices.len()).map(|i| (i.to_string(), "选项")),
                CompletionItemKind::VALUE,
                input,
                position,
            ),
            _ => Vec::default(),
        }
    }
}

impl Complete for LabelSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for JumpLabelSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_ident_enum(&project.ident().label, "标签", input, position)
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

// -------- 鉴赏 --------

impl Complete for UnlockCgSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_file(
            &project.resource().background,
            |_, _| (CompletionItemKind::FILE, "背景".to_string()),
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.name.is_none() => ("name", "name=", "鉴赏解锁名称"),
                self.series.is_none() => ("series", "series=", "鉴赏系列名称"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "series" => complete_ident_enum(&project.ident().series, "鉴赏系列", input, position),
            _ => Vec::default(),
        }
    }
}

impl Complete for UnlockBgmSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_file(
            &project.resource().bgm,
            |_, _| (CompletionItemKind::FILE, "音乐".to_string()),
            input,
            position,
        )
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.name.is_none() => ("name", "name=", "鉴赏解锁名称"),
                self.series.is_none() => ("series", "series=", "鉴赏系列名称"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }

    fn complete_argument_value(
        &self,
        name: &str,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        match name {
            "series" => complete_ident_enum(&project.ident().series, "鉴赏系列", input, position),
            _ => Vec::default(),
        }
    }
}

// -------- 游戏控制 --------

impl Complete for GetUserInputSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.title.is_none() => ("title", "title=", "对话框标题"),
                self.button_text.is_none() => ("buttonText", "buttonText=", "确认按钮文本"),
                self.default_value.is_none() => ("defaultValue", "defaultValue=", "默认值"),
                self.rule.is_none() => ("rule", "rule=", "输入校验正则"),
                self.rule_flag.is_none() => ("ruleFlag", "ruleFlag=", "正则标记"),
                self.rule_text.is_none() => ("ruleText", "ruleText=", "校验失败提示"),
                self.rule_button_text.is_none() => ("ruleButtonText", "ruleButtonText=", "提示按钮文本"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for SetVarSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                !self.global => ("global", "global", "全局变量"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for ShowVarsSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for WaitSentence {
    fn complete_content(
        &self,
        input: &str,
        position: Position,
        project: &Project,
    ) -> Vec<CompletionItem> {
        complete_duration_enum("持续时间 (ms)", input, position, project)
    }

    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for ApplyStyleSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for CallSteamSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.achivement_id.is_none() => ("achivementId", "achivementId=", "成就 ID"),
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

impl Complete for EndSentence {
    fn complete_argument_name(
        &self,
        input: &str,
        position: Position,
        _project: &Project,
    ) -> Vec<CompletionItem> {
        complete_argument_name_collect! {
            (input, position): {
                self.when.is_none() => ("when", "when=", "条件执行"),
            }
        }
    }
}

// -------- 空白注释 --------

impl Complete for CommentSentence {}
