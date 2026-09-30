use lsp_types::*;
use path_tree::canonicalize;
use webgal_language_core::{
    element::ChoiceSplit,
    resource::{FigureKind, ResourceKind},
    sentence::{
        Sentence, SentenceInfo, SentenceLocation, is_call_scene_variable_argument,
        is_implicit_vocal_argument,
    },
    util::span_of,
};

use crate::{
    project::Project,
    service::{position_in_range, variable_location_to_range},
};

/// 符号类型
///
/// # Notes
/// 不完全保证符号在项目中真实有效, 需调用者自行校验.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ident<'a> {
    Resource(ResourceKind, &'a str),
    Speaker(&'a str),
    Object(&'a str),
    Sound(&'a str),
    Label(&'a str),
    Variable(&'a str),
    Series(&'a str),
    Unlockname(&'a str),
}

impl<'a> Ident<'a> {
    /// 获取符号名称
    ///
    /// # Behavior
    /// * 名称是语句原始字符串的子串, 可用 [`span_of`] 求其在语句中的区间.
    pub(super) fn name(&self) -> &'a str {
        match *self {
            Self::Resource(_, path) => path,
            Self::Speaker(name)
            | Self::Object(name)
            | Self::Sound(name)
            | Self::Label(name)
            | Self::Variable(name)
            | Self::Series(name)
            | Self::Unlockname(name) => name,
        }
    }

    /// 以新名称构造同类符号
    ///
    /// # Notes
    /// 名称可以来自项目外 (如客户端请求的新名称), 不要求与符号同生命周期.
    pub(super) fn with_name(self, name: &'a str) -> Self {
        match self {
            Self::Resource(kind, _) => Self::Resource(kind, name),
            Self::Speaker(_) => Self::Speaker(name),
            Self::Object(_) => Self::Object(name),
            Self::Sound(_) => Self::Sound(name),
            Self::Label(_) => Self::Label(name),
            Self::Variable(_) => Self::Variable(name),
            Self::Series(_) => Self::Series(name),
            Self::Unlockname(_) => Self::Unlockname(name),
        }
    }
}

/// 识别光标指向的符号类型
pub fn recognize_ident<'a>(
    scene_path: &str,
    position: Position,
    project: &'a Project,
) -> Option<Ident<'a>> {
    let scene = project.resource().scene.get(scene_path)?.as_item()?;
    let sentence = scene.sentences().get(position.line as usize)?;

    // 定位输入
    match sentence.primary.locate(position.character as usize) {
        SentenceLocation::Command(command, offset) => {
            recognize_ident_in_command(command, offset, sentence)
        }
        SentenceLocation::Content(content, offset) => recognize_ident_in_content(
            content,
            offset,
            position,
            sentence.content,
            sentence,
            scene_path,
            project,
        ),
        SentenceLocation::ArgumentName(_, name, ..) => {
            recognize_ident_in_argument_name(name, sentence)
        }
        SentenceLocation::ArgumentValue(_, name, value, ..) => recognize_ident_in_argument_value(
            name,
            value,
            position,
            sentence.content,
            sentence,
            scene_path,
            project,
        ),
        _ => None,
    }
}

/// 语句中可能承载标识符的位置
///
/// # Returns
/// 语句内 UTF-8 字节偏移, 依次为:
/// * 语句类型 (对话人物名);
/// * 主参数;
/// * 参数名 (对话的隐式语音参数);
/// * 参数值;
/// * 分支选项的跳转目标.
///
/// # Notes
/// 必须与 [`recognize_ident`] 判定的位置保持一致.
/// 变量插值仅可能识别为变量, 而变量由变量表提供, 故不在此列.
pub(super) fn candidate_offsets(info: &SentenceInfo) -> Vec<usize> {
    let primary = &info.primary;
    let mut offsets = Vec::with_capacity(2 + primary.arguments.len() * 2);
    offsets.push(span_of(info.content, primary.command).start);

    if let Some(content) = primary.content {
        offsets.push(span_of(info.content, content).start);
    }

    for &(name, value) in &primary.arguments {
        offsets.push(span_of(info.content, name).start);
        if let Some(value) = value {
            offsets.push(span_of(info.content, value).start);
        }
    }

    // 分支选项的跳转目标可指向场景或标签
    if let Some(content) = primary.content
        && matches!(info.sentence, Sentence::Choose(_))
    {
        offsets.extend(
            ChoiceSplit::new(content)
                .filter_map(|choice| choice.target)
                .map(|target| span_of(info.content, target).start),
        );
    }

    offsets
}

fn recognize_ident_in_command<'a>(
    command: &'a str,
    offset: usize,
    sentence: &SentenceInfo,
) -> Option<Ident<'a>> {
    match sentence.sentence {
        // 无主参数时语句类型即为对话正文, 与 `say` 语句类型一样不构成对话人物
        Sentence::Say(_) if sentence.primary.content.is_some() && command != "say" => Some(
            try_recognize_interpolate(command, offset)
                .map_or(Ident::Speaker(command), Ident::Variable),
        ),
        _ => None,
    }
}

fn recognize_ident_in_content<'a>(
    content: &'a str,
    offset: usize,
    position: Position,
    line: &'a str,
    sentence: &Sentence,
    scene_path: &str,
    project: &'a Project,
) -> Option<Ident<'a>> {
    match sentence {
        Sentence::Say(_) | Sentence::Intro(_) | Sentence::Return(_) | Sentence::SetVariable(_) => {
            try_recognize_interpolate(content, offset)
                .or_else(|| try_recognize_variable(position, line, scene_path, project))
                .map(Ident::Variable)
        }
        Sentence::GetUserInput(_) => Some(Ident::Variable(content)),

        Sentence::ChangeBackground(s) if s.background.is_some() => {
            Some(Ident::Resource(ResourceKind::Background, content))
        }
        Sentence::ChangeFigure(s) if s.figure.is_some() => Some(Ident::Resource(
            ResourceKind::Figure,
            FigureKind::from_path(content).1,
        )),
        Sentence::Bgm(s) if s.bgm.is_some() => Some(Ident::Resource(ResourceKind::Bgm, content)),
        Sentence::PlayVideo(_) => Some(Ident::Resource(ResourceKind::Video, content)),
        Sentence::PlayEffect(s) if s.vocal.is_some() => {
            Some(Ident::Resource(ResourceKind::Vocal, content))
        }
        Sentence::MiniAvatar(s) if s.avatar.is_some() => {
            Some(Ident::Resource(ResourceKind::Figure, content))
        }
        Sentence::UnlockCg(_) => Some(Ident::Resource(ResourceKind::Background, content)),
        Sentence::UnlockBgm(_) => Some(Ident::Resource(ResourceKind::Bgm, content)),

        Sentence::SetAnimation(_) if project.resource().contains_animation(content) => {
            Some(Ident::Resource(ResourceKind::Animation, content))
        }

        Sentence::CallScene(_) | Sentence::ChangeScene(_) => {
            Some(Ident::Resource(ResourceKind::Scene, content))
        }
        Sentence::Label(_) | Sentence::JumpLabel(_) => Some(Ident::Label(content)),

        Sentence::Choose(_) => {
            // 光标所在选项的跳转目标 (提示文本在其之前, 故不能按提示文本定位)
            let target = ChoiceSplit::new(content)
                .filter_map(|choice| choice.target)
                .find(|&target| span_of(content, target).contains(&offset));

            if let Some(target) = target {
                if project.resource().scene.contains(target)
                    || canonicalize(target)
                        .is_some_and(|path| project.resource().scene.contains(&path))
                {
                    return Some(Ident::Resource(ResourceKind::Scene, target));
                } else if project.ident().label.contains(&target.to_string()) {
                    return Some(Ident::Label(target));
                }
            }

            try_recognize_variable(position, line, scene_path, project).map(Ident::Variable)
        }

        _ => None,
    }
}

fn recognize_ident_in_argument_name<'a>(name: &'a str, sentence: &Sentence) -> Option<Ident<'a>> {
    match sentence {
        Sentence::Say(_) if is_implicit_vocal_argument(name) => {
            Some(Ident::Resource(ResourceKind::Vocal, name))
        }
        Sentence::CallScene(_) if is_call_scene_variable_argument(name) => {
            Some(Ident::Variable(name))
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn recognize_ident_in_argument_value<'a>(
    name: &'a str,
    value: &'a str,
    position: Position,
    line: &'a str,
    sentence: &Sentence,
    scene_path: &str,
    project: &'a Project,
) -> Option<Ident<'a>> {
    match sentence {
        _ if name == "when" => {
            try_recognize_variable(position, line, scene_path, project).map(Ident::Variable)
        }
        Sentence::CallScene(_) if is_call_scene_variable_argument(name) => {
            try_recognize_variable(position, line, scene_path, project).map(Ident::Variable)
        }
        Sentence::CallScene(_) if name == "writeReturnTo" => Some(Ident::Variable(value)),

        _ if matches!(name, "enter" | "exit") && project.resource().contains_animation(value) => {
            Some(Ident::Resource(ResourceKind::Animation, value))
        }
        _ if name == "series" => Some(Ident::Series(value)),
        _ if name == "unlockname" => Some(Ident::Unlockname(value)),
        Sentence::UnlockCg(_) | Sentence::UnlockBgm(_) if name == "name" => {
            Some(Ident::Unlockname(value))
        }

        Sentence::Say(_) if name == "speaker" => Some(Ident::Speaker(value)),
        Sentence::Say(_) if name == "vocal" => Some(Ident::Resource(ResourceKind::Vocal, value)),
        Sentence::Say(_) if name == "figureId" => Some(Ident::Object(value)),

        Sentence::ChangeFigure(_) if name == "id" => Some(Ident::Object(value)),
        Sentence::ChangeFigure(_)
            if matches!(
                name,
                "mouthOpen" | "mouthHalfOpen" | "mouthClose" | "eyesOpen" | "eyesClose"
            ) =>
        {
            Some(Ident::Resource(ResourceKind::Figure, value))
        }
        Sentence::PlayEffect(_) if name == "id" => Some(Ident::Sound(value)),
        Sentence::Intro(_) if name == "backgroundImage" => {
            Some(Ident::Resource(ResourceKind::Background, value))
        }

        Sentence::SetAnimation(_)
        | Sentence::SetComplexAnimation(_)
        | Sentence::SetTransform(_)
        | Sentence::SetTempAnimation(_)
        | Sentence::SetTransition(_)
            if name == "target" =>
        {
            Some(Ident::Object(value))
        }

        _ => None,
    }
}

/// 尝试识别光标处的插值变量
///
/// # Behavior
/// * 变量名为光标前最近的 `{` 与其后最近的 `}` 之间的内容;
/// * 光标不在花括号内 (含未闭合) 时返回 `None`.
fn try_recognize_interpolate(input: &str, offset: usize) -> Option<&str> {
    let start = input[..offset].rfind('{')?;
    let end = start + input[start..].find('}')?;
    if end < offset {
        return None;
    }
    Some(&input[start + 1..end])
}

fn try_recognize_variable<'a>(
    position: Position,
    sentence: &'a str,
    scene_path: &str,
    project: &'a Project,
) -> Option<&'a str> {
    project
        .variable()
        .values()
        .find_map(|variable| {
            variable.iter_references().find_map(|location| {
                (location.scene == scene_path
                    && position_in_range(position, variable_location_to_range(location)))
                .then_some(location.span.clone())
            })
        })
        .map(|span| &sentence[span])
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use webgal_language_core::resource::Config;

    use super::*;

    /// 构建测试项目
    fn project(scenes: &[(&str, &str)]) -> Project {
        let mut project = Project::new(Config::default());
        for (path, content) in scenes {
            project
                .insert(&format!("scene/{path}"), || Ok(content.to_string()))
                .unwrap();
        }
        project
    }

    /// 识别场景某行中名称首次出现处的标识符
    fn recognize<'a>(
        scene: &str,
        project: &'a Project,
        line: u32,
        name: &str,
    ) -> Option<Ident<'a>> {
        let text = scene.lines().nth(line as usize).expect("行应在场景中");
        let position = Position {
            line,
            character: text.find(name).expect("名称应在语句中") as u32,
        };
        recognize_ident("1.txt", position, project)
    }

    // -------- 插值 --------

    #[test]
    fn recognizes_interpolation_inside_text() {
        let scene = "chara:你好 {coin};\nsetVar:coin=1;";
        let project = project(&[("1.txt", scene)]);

        assert_eq!(
            recognize(scene, &project, 0, "coin"),
            Some(Ident::Variable("coin"))
        );
    }

    #[test]
    fn ignores_unclosed_interpolation() {
        let scene = "chara:你好 {coin;";
        let project = project(&[("1.txt", scene)]);

        assert_eq!(recognize(scene, &project, 0, "coin"), None);
    }

    // -------- 变量 --------

    #[test]
    fn recognizes_variable_definition() {
        let scene = "setVar:hp=100;";
        let project = project(&[("1.txt", scene)]);

        assert_eq!(
            recognize(scene, &project, 0, "hp"),
            Some(Ident::Variable("hp"))
        );
    }

    #[test]
    fn recognizes_variable_in_expression() {
        let scene = "setVar:base=5;\nsetVar:hp=base+1;\nsay:hello -when=hp;";
        let project = project(&[("1.txt", scene)]);

        assert_eq!(
            recognize(scene, &project, 1, "base"),
            Some(Ident::Variable("base"))
        );
        assert_eq!(
            recognize(scene, &project, 2, "hp"),
            Some(Ident::Variable("hp"))
        );
    }

    // -------- 对话人物 --------

    #[test]
    fn distinguishes_speaker_from_dialogue_text() {
        // 有主参数时语句类型为对话人物
        let scene = "chara:你好;\nchara:;";
        let speaker = project(&[("1.txt", scene)]);
        assert_eq!(
            recognize(scene, &speaker, 0, "chara"),
            Some(Ident::Speaker("chara"))
        );
        assert_eq!(
            recognize(scene, &speaker, 1, "chara"),
            Some(Ident::Speaker("chara"))
        );

        // 无主参数时语句类型为对话正文, 由 `-speaker` 参数指定对话人物
        let scene = "你好 -speaker=chara;";
        let narration = project(&[("1.txt", scene)]);
        assert_eq!(recognize(scene, &narration, 0, "你好"), None);
        assert_eq!(
            recognize(scene, &narration, 0, "chara"),
            Some(Ident::Speaker("chara"))
        );

        // `say` 语句类型不是对话人物
        let scene = "say:hello;";
        let say = project(&[("1.txt", scene)]);
        assert_eq!(recognize(scene, &say, 0, "say"), None);
    }

    // -------- 分支选项 --------

    #[test]
    fn recognizes_choose_scene_target() {
        let scene = "choose:去往A:2.txt|留在这里:3.txt;";
        let project = project(&[("1.txt", scene), ("2.txt", ""), ("3.txt", "")]);

        assert_eq!(
            recognize(scene, &project, 0, "2.txt"),
            Some(Ident::Resource(ResourceKind::Scene, "2.txt"))
        );
        assert_eq!(
            recognize(scene, &project, 0, "3.txt"),
            Some(Ident::Resource(ResourceKind::Scene, "3.txt"))
        );
    }

    #[test]
    fn recognizes_choose_label_target() {
        let scene = "label:start;\nchoose:去往A:start|留在这里:2.txt;";
        let project = project(&[("1.txt", scene), ("2.txt", "")]);

        assert_eq!(
            recognize(scene, &project, 1, "start"),
            Some(Ident::Label("start"))
        );
    }
}
