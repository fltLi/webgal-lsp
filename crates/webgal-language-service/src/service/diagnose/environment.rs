//! 单语句资源及全局上下文诊断

use std::borrow::Borrow;

use path_tree::{Folder, canonicalize};
use webgal_language_core::{
    element::{ChoiceSplit, ChoiceView},
    resource::FigureInfo,
    sentence::*,
};

use crate::{
    project::Project,
    service::diagnose::{DiagnosticLevel, DiagnosticLocation, PrimaryDiagnostic},
};

/// 语句环境诊断 (资源 + 全局上下文)
pub fn diagnose_environment<F>(
    content: &str,
    primary: &PrimarySentence,
    sentence: &Sentence,
    project: &Project,
    mut diagnose: F,
) where
    F: FnMut(PrimaryDiagnostic),
{
    diagnose_resource(content, primary, sentence, project, &mut diagnose);
}

// -------- resource --------

/// 语句资源依赖检查
fn diagnose_resource<F>(
    content: &str,
    primary: &PrimarySentence,
    sentence: &Sentence,
    project: &Project,
    mut diagnose: F,
) where
    F: FnMut(PrimaryDiagnostic),
{
    use Sentence::*;

    match sentence {
        // 常规演出
        Say(SaySentence {
            vocal: Some(vocal), ..
        }) if !project
            .resource()
            .vocal
            .contains(canonicalize(vocal).as_ref().unwrap_or(vocal)) =>
        {
            let span = match content.find(vocal) {
                Some(index) => index..index + vocal.len(), // TODO: 绝对精确地定位
                None => 0..content.len(),
            };
            diagnose(PrimaryDiagnostic {
                span: span.into(),
                code: "WG007",
                level: DiagnosticLevel::Error,
                message: format!("找不到或无法识别语音: {vocal}"),
            });
        }

        ChangeBackground(sentence) => {
            let ChangeBackgroundSentence {
                background,
                enter,
                exit,
                ..
            } = &**sentence;

            if let Some(background) = background
                && !project
                    .resource()
                    .background
                    .contains(canonicalize(background).as_ref().unwrap_or(background))
            {
                diagnose(PrimaryDiagnostic {
                    span: DiagnosticLocation::Content,
                    code: "WG007",
                    level: DiagnosticLevel::Error,
                    message: format!("找不到或无法识别背景: {background}"),
                })
            }

            if let Some(enter) = enter
                && !project.resource().contains_animation(enter)
            {
                diagnose(PrimaryDiagnostic {
                    span: DiagnosticLocation::ArgumentValue("enter"),
                    code: "WG007",
                    level: DiagnosticLevel::Error,
                    message: format!("找不到或无法识别动画: {enter}"),
                })
            }
            if let Some(exit) = exit
                && !project.resource().contains_animation(exit)
            {
                diagnose(PrimaryDiagnostic {
                    span: DiagnosticLocation::ArgumentValue("exit"),
                    code: "WG007",
                    level: DiagnosticLevel::Error,
                    message: format!("找不到或无法识别动画: {exit}"),
                })
            }
        }

        ChangeFigure(sentence) => {
            let ChangeFigureSentence {
                figure,
                mouth_open,
                mouth_half_open,
                mouth_close,
                eyes_open,
                eyes_close,
                motion,
                expression,
                enter,
                exit,
                ..
            } = &**sentence;

            diagnose_argument_resource(
                "mouthOpen",
                mouth_open,
                &project.resource().figure,
                "图片立绘",
            )
            .map(&mut diagnose);
            diagnose_argument_resource(
                "mouthHalfOpen",
                mouth_half_open,
                &project.resource().figure,
                "图片立绘",
            )
            .map(&mut diagnose);
            diagnose_argument_resource(
                "mouthClose",
                mouth_close,
                &project.resource().figure,
                "图片立绘",
            )
            .map(&mut diagnose);
            diagnose_argument_resource(
                "eyesOpen",
                eyes_open,
                &project.resource().figure,
                "图片立绘",
            )
            .map(&mut diagnose);
            diagnose_argument_resource(
                "eyesClose",
                eyes_close,
                &project.resource().figure,
                "图片立绘",
            )
            .map(&mut diagnose);

            if let Some(figure) = figure {
                let info = match project
                    .resource()
                    .get_figure_redirected(canonicalize(figure).as_ref().unwrap_or(figure))
                {
                    Some(info) => info,
                    None => {
                        diagnose(PrimaryDiagnostic {
                            span: DiagnosticLocation::Content,
                            code: "WG007",
                            level: DiagnosticLevel::Error,
                            message: format!("找不到或无法识别立绘: {figure}"),
                        });
                        return;
                    }
                };

                if let Some(motion) = motion
                    && let FigureInfo::Live2d { motions, .. } = info
                    && !motions.contains(motion)
                {
                    diagnose(PrimaryDiagnostic {
                        span: DiagnosticLocation::ArgumentValue("motion"),
                        code: "WG007",
                        level: DiagnosticLevel::Error,
                        message: format!("找不到或无法识别立绘动作: {motion}"),
                    })
                }
                if let Some(expression) = expression
                    && let FigureInfo::Live2d { expressions, .. } = info
                    && !expressions.contains(expression)
                {
                    diagnose(PrimaryDiagnostic {
                        span: DiagnosticLocation::ArgumentValue("expression"),
                        code: "WG007",
                        level: DiagnosticLevel::Error,
                        message: format!("找不到或无法识别 Live2D 表情: {expression}"),
                    })
                }
            }

            if let Some(enter) = enter
                && !project.resource().contains_animation(enter)
            {
                diagnose(PrimaryDiagnostic {
                    span: DiagnosticLocation::ArgumentValue("enter"),
                    code: "WG007",
                    level: DiagnosticLevel::Error,
                    message: format!("找不到或无法识别动画: {enter}"),
                })
            }
            if let Some(exit) = exit
                && !project.resource().contains_animation(exit)
            {
                diagnose(PrimaryDiagnostic {
                    span: DiagnosticLocation::ArgumentValue("exit"),
                    code: "WG007",
                    level: DiagnosticLevel::Error,
                    message: format!("找不到或无法识别动画: {exit}"),
                })
            }
        }

        Bgm(BgmSentence { bgm: Some(bgm), .. }) => {
            diagnose_content_resource(bgm, &project.resource().bgm, "音乐").map(&mut diagnose);
        }

        PlayVideo(PlayVideoSentence { video, .. }) => {
            diagnose_content_resource(video, &project.resource().video, "视频").map(&mut diagnose);
        }

        PlayEffect(PlayEffectSentence {
            vocal: Some(vocal), ..
        }) => {
            diagnose_content_resource(vocal, &project.resource().bgm, "语音 (音效)")
                .map(&mut diagnose);
        }

        // 舞台对象控制
        SetAnimation(SetAnimationSentence { animation, .. })
            if !project.resource().contains_animation(animation) =>
        {
            diagnose(PrimaryDiagnostic {
                span: DiagnosticLocation::Content,
                code: "WG007",
                level: DiagnosticLevel::Error,
                message: format!("找不到或无法识别动画: {animation}"),
            });
        }

        SetComplexAnimation(SetComplexAnimationSentence { animation, .. })
            if !matches!(animation.as_str(), "universalSoftIn" | "universalSoftOut") =>
        {
            diagnose(PrimaryDiagnostic {
                span: DiagnosticLocation::Content,
                code: "WG007",
                level: DiagnosticLevel::Error,
                message: format!("找不到或无法识别复杂动画: {animation}"),
            })
        }

        SetTransition(SetTransitionSentence { enter, exit, .. }) => {
            if let Some(enter) = enter
                && !project.resource().contains_animation(enter)
            {
                diagnose(PrimaryDiagnostic {
                    span: DiagnosticLocation::ArgumentValue("enter"),
                    code: "WG007",
                    level: DiagnosticLevel::Error,
                    message: format!("找不到或无法识别动画: {enter}"),
                })
            }
            if let Some(exit) = exit
                && !project.resource().contains_animation(exit)
            {
                diagnose(PrimaryDiagnostic {
                    span: DiagnosticLocation::ArgumentValue("exit"),
                    code: "WG007",
                    level: DiagnosticLevel::Error,
                    message: format!("找不到或无法识别动画: {exit}"),
                })
            }
        }

        // 特殊演出
        Intro(IntroSentence {
            background_image, ..
        }) => {
            diagnose_argument_resource(
                "backgroundImage",
                background_image,
                &project.resource().background,
                "背景",
            )
            .map(&mut diagnose);
        }

        MiniAvatar(MiniAvatarSentence {
            avatar: Some(avatar),
            ..
        }) => {
            diagnose_content_resource(avatar, &project.resource().figure, "小头像")
                .map(&mut diagnose);
        }

        // 场景与分支
        CallScene(CallSceneSentence { scene, .. }) => {
            diagnose_content_resource(scene, &project.resource().scene, "场景").map(&mut diagnose);
        }

        ChangeScene(ChangeSceneSentence { scene, .. }) => {
            diagnose_content_resource(scene, &project.resource().scene, "场景").map(&mut diagnose);
        }

        Choose(_) if let Some(content) = primary.content => {
            ChoiceSplit::new(content)
                .filter_map(|ChoiceView { target, .. }| target.map(str::trim))
                .filter(|target| {
                    !project.resource().scene.contains(target)
                        && !project.ident().label.contains(&target.to_string())
                })
                .for_each(|target| {
                    diagnose(PrimaryDiagnostic {
                        span: primary.get_span(target).into(),
                        code: "WG007",
                        level: DiagnosticLevel::Error,
                        message: format!("找不到或无法识别场景选项: {target}"),
                    })
                });
        }

        // 鉴赏
        UnlockCg(UnlockCgSentence { image, .. }) => {
            diagnose_content_resource(image, &project.resource().background, "图片")
                .map(&mut diagnose);
        }

        UnlockBgm(UnlockBgmSentence { bgm, .. }) => {
            diagnose_content_resource(bgm, &project.resource().bgm, "音乐").map(&mut diagnose);
        }

        _ => {}
    }
}

fn diagnose_content_resource<T>(
    path: &str,
    folder: &Folder<T>,
    description: &str,
) -> Option<PrimaryDiagnostic> {
    if !folder.contains(&canonicalize(path).unwrap_or_else(|| path.to_string())) {
        Some(PrimaryDiagnostic {
            span: DiagnosticLocation::Content,
            code: "WG007",
            level: DiagnosticLevel::Error,
            message: format!("找不到或无法识别{description}: {path}"),
        })
    } else {
        None
    }
}

fn diagnose_argument_resource<P, T>(
    name: &'static str,
    path: impl Borrow<Option<P>>,
    folder: &Folder<T>,
    description: &str,
) -> Option<PrimaryDiagnostic>
where
    P: AsRef<str>,
{
    if let Some(path) = path.borrow().as_ref().map(P::as_ref)
        && !folder.contains(&canonicalize(path).unwrap_or_else(|| path.to_string()))
    {
        Some(PrimaryDiagnostic {
            span: DiagnosticLocation::ArgumentValue(name),
            code: "WG007",
            level: DiagnosticLevel::Error,
            message: format!("找不到或无法识别{description}: {path}"),
        })
    } else {
        None
    }
}
