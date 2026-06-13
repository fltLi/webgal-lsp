//! 单语句资源及全局上下文诊断

use std::{borrow::Borrow, ops};

use path_tree::{Folder, Node, canonicalize};
use webgal_model::sentence::*;

use crate::{
    context::Context,
    service::diagnose::{DiagnosticLevel, PrimaryDiagnostic},
};

/// 语句环境诊断 (资源 + 全局上下文)
pub fn diagnose_environment<F>(
    content: &str,
    primary: &PrimarySentence,
    sentence: &Sentence,
    context: &Context,
    mut diagnose: F,
) where
    F: FnMut(PrimaryDiagnostic),
{
    diagnose_resource(content, primary, sentence, context, &mut diagnose);
}

// -------- resource --------

/// 语句资源依赖检查
fn diagnose_resource<F>(
    content: &str,
    primary: &PrimarySentence,
    sentence: &Sentence,
    context: &Context,
    mut diagnose: F,
) where
    F: FnMut(PrimaryDiagnostic),
{
    use Sentence::*;

    match sentence {
        // 常规演出
        Say(SaySentence {
            vocal: Some(vocal), ..
        }) if !context
            .resource
            .vocal
            .contains(canonicalize(vocal).as_ref().unwrap_or(vocal)) =>
        {
            let span = match content.find(vocal) {
                Some(index) => index..index + vocal.len(), // TODO: 绝对精确地定位
                None => 0..content.len(),
            };
            diagnose(PrimaryDiagnostic {
                span,
                code: "WG007",
                level: DiagnosticLevel::Warning,
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

            if !matches!(background.as_str(), "" | "none")
                && !context
                    .resource
                    .background
                    .contains(canonicalize(background).as_ref().unwrap_or(background))
                && let Some(content) = primary.content
            {
                diagnose(PrimaryDiagnostic {
                    span: primary.get_span(content),
                    code: "WG007",
                    level: DiagnosticLevel::Warning,
                    message: format!("找不到或无法识别背景: {background}"),
                })
            }

            if let Some(enter) = enter
                && !context.resource.contains_animation(enter)
                && let Some(span) = argument_span_of("enter", primary)
            {
                diagnose(PrimaryDiagnostic {
                    span,
                    code: "WG007",
                    level: DiagnosticLevel::Warning,
                    message: format!("找不到或无法识别动画: {enter}"),
                })
            }
            if let Some(exit) = exit
                && !context.resource.contains_animation(exit)
                && let Some(span) = argument_span_of("exit", primary)
            {
                diagnose(PrimaryDiagnostic {
                    span,
                    code: "WG007",
                    level: DiagnosticLevel::Warning,
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
                primary,
                mouth_open,
                &context.resource.figure,
                "图片立绘",
            )
            .map(&mut diagnose);
            diagnose_argument_resource(
                "mouthHalfOpen",
                primary,
                mouth_half_open,
                &context.resource.figure,
                "图片立绘",
            )
            .map(&mut diagnose);
            diagnose_argument_resource(
                "mouthClose",
                primary,
                mouth_close,
                &context.resource.figure,
                "图片立绘",
            )
            .map(&mut diagnose);
            diagnose_argument_resource(
                "eyesOpen",
                primary,
                eyes_open,
                &context.resource.figure,
                "图片立绘",
            )
            .map(&mut diagnose);
            diagnose_argument_resource(
                "eyesClose",
                primary,
                eyes_close,
                &context.resource.figure,
                "图片立绘",
            )
            .map(&mut diagnose);

            if !matches!(figure.as_str(), "" | "none") {
                let info = match context
                    .resource
                    .figure
                    .get(canonicalize(figure).as_ref().unwrap_or(figure))
                    .and_then(Node::as_item)
                {
                    Some(info) => info,
                    None => {
                        if let Some(content) = primary.content {
                            diagnose(PrimaryDiagnostic {
                                span: primary.get_span(content),
                                code: "WG007",
                                level: DiagnosticLevel::Warning,
                                message: format!("找不到或无法识别立绘: {figure}"),
                            })
                        }
                        return;
                    }
                };

                if let Some(motion) = motion
                    && !info.motions.contains(motion)
                    && let Some(span) = argument_span_of("motion", primary)
                {
                    diagnose(PrimaryDiagnostic {
                        span,
                        code: "WG007",
                        level: DiagnosticLevel::Warning,
                        message: format!("找不到或无法识别立绘动作: {motion}"),
                    })
                }
                if let Some(expression) = expression
                    && !info.expressions.contains(expression)
                    && let Some(span) = argument_span_of("expression", primary)
                {
                    diagnose(PrimaryDiagnostic {
                        span,
                        code: "WG007",
                        level: DiagnosticLevel::Warning,
                        message: format!("找不到或无法识别 Live2D 表情: {expression}"),
                    })
                }
            }

            if let Some(enter) = enter
                && !context.resource.contains_animation(enter)
                && let Some(span) = argument_span_of("enter", primary)
            {
                diagnose(PrimaryDiagnostic {
                    span,
                    code: "WG007",
                    level: DiagnosticLevel::Warning,
                    message: format!("找不到或无法识别动画: {enter}"),
                })
            }
            if let Some(exit) = exit
                && !context.resource.contains_animation(exit)
                && let Some(span) = argument_span_of("exit", primary)
            {
                diagnose(PrimaryDiagnostic {
                    span,
                    code: "WG007",
                    level: DiagnosticLevel::Warning,
                    message: format!("找不到或无法识别动画: {exit}"),
                })
            }
        }

        Bgm(BgmSentence { bgm, .. }) if !matches!(bgm.as_str(), "" | "none") => {
            diagnose_content_resource(primary, bgm, &context.resource.bgm, "音乐")
                .map(&mut diagnose);
        }

        PlayVideo(PlayVideoSentence { video, .. }) => {
            diagnose_content_resource(primary, video, &context.resource.video, "视频")
                .map(&mut diagnose);
        }

        PlayEffect(PlayEffectSentence { vocal, id, .. })
            if !matches!(vocal.as_str(), "" | "none") && id.is_none() =>
        {
            diagnose_content_resource(primary, vocal, &context.resource.bgm, "语音 (音效)")
                .map(&mut diagnose);
        }

        // 舞台对象控制
        SetAnimation(SetAnimationSentence { animation, .. })
            if !context.resource.contains_animation(animation)
                && let Some(content) = primary.content =>
        {
            diagnose(PrimaryDiagnostic {
                span: primary.get_span(content),
                code: "WG007",
                level: DiagnosticLevel::Warning,
                message: format!("找不到或无法识别动画: {animation}"),
            });
        }

        SetComplexAnimation(SetComplexAnimationSentence { animation, .. })
            if !matches!(animation.as_str(), "universalSoftIn" | "universalSoftOut")
                && let Some(content) = primary.content =>
        {
            diagnose(PrimaryDiagnostic {
                span: primary.get_span(content),
                code: "WG007",
                level: DiagnosticLevel::Warning,
                message: format!("找不到或无法识别复杂动画: {animation}"),
            })
        }

        SetTransition(SetTransitionSentence { enter, exit, .. }) => {
            if let Some(enter) = enter
                && !context.resource.contains_animation(enter)
                && let Some(span) = argument_span_of("enter", primary)
            {
                diagnose(PrimaryDiagnostic {
                    span,
                    code: "WG007",
                    level: DiagnosticLevel::Warning,
                    message: format!("找不到或无法识别动画: {enter}"),
                })
            }
            if let Some(exit) = exit
                && !context.resource.contains_animation(exit)
                && let Some(span) = argument_span_of("exit", primary)
            {
                diagnose(PrimaryDiagnostic {
                    span,
                    code: "WG007",
                    level: DiagnosticLevel::Warning,
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
                primary,
                background_image,
                &context.resource.background,
                "背景",
            )
            .map(&mut diagnose);
        }

        MiniAvatar(MiniAvatarSentence { avatar, .. }) => {
            diagnose_content_resource(primary, avatar, &context.resource.figure, "小头像")
                .map(&mut diagnose);
        }

        // 场景与分支
        CallScene(CallSceneSentence { scene, .. }) => {
            diagnose_content_resource(primary, scene, &context.resource.scene, "场景")
                .map(&mut diagnose);
        }

        ChangeScene(ChangeSceneSentence { scene, .. }) => {
            diagnose_content_resource(primary, scene, &context.resource.scene, "场景")
                .map(&mut diagnose);
        }

        Choose(_) if let Some(content) = primary.content => {
            content
                .split('|')
                .filter_map(|choice| choice.split_once(':'))
                .map(|(_, scene)| scene.trim())
                .filter(|scene| {
                    !context.ident.label.contains(&scene.to_string())
                        && !context.resource.scene.contains(scene)
                })
                .for_each(|scene| {
                    diagnose(PrimaryDiagnostic {
                        span: primary.get_span(scene),
                        code: "WG007",
                        level: DiagnosticLevel::Warning,
                        message: format!("找不到或无法识别场景选项: {scene}"),
                    })
                });
        }

        // 鉴赏
        UnlockCg(UnlockCgSentence { image, .. }) => {
            diagnose_content_resource(primary, image, &context.resource.background, "图片")
                .map(&mut diagnose);
        }

        UnlockBgm(UnlockBgmSentence { bgm, .. }) => {
            diagnose_content_resource(primary, bgm, &context.resource.bgm, "音乐")
                .map(&mut diagnose);
        }

        _ => {}
    }
}

fn diagnose_content_resource<T>(
    primary: &PrimarySentence,
    path: &str,
    folder: &Folder<T>,
    description: &str,
) -> Option<PrimaryDiagnostic> {
    if !folder.contains(&canonicalize(path).unwrap_or_else(|| path.to_string()))
        && let Some(content) = primary.content
    {
        Some(PrimaryDiagnostic {
            span: primary.get_span(content),
            code: "WG007",
            level: DiagnosticLevel::Warning,
            message: format!("找不到或无法识别{description}: {path}"),
        })
    } else {
        None
    }
}

fn diagnose_argument_resource<P, T>(
    name: &str,
    primary: &PrimarySentence,
    path: impl Borrow<Option<P>>,
    folder: &Folder<T>,
    description: &str,
) -> Option<PrimaryDiagnostic>
where
    P: AsRef<str>,
{
    if let Some(path) = path.borrow().as_ref().map(P::as_ref)
        && !folder.contains(&canonicalize(path).unwrap_or_else(|| path.to_string()))
        && let Some(span) = argument_span_of(name, primary)
    {
        Some(PrimaryDiagnostic {
            span,
            code: "WG007",
            level: DiagnosticLevel::Warning,
            message: format!("找不到或无法识别{description}: {path}"),
        })
    } else {
        None
    }
}

// -------- util --------

fn argument_span_of(name: &str, primary: &PrimarySentence) -> Option<ops::Range<usize>> {
    let (index, _) = primary.get_argument(name)?;
    let argument = primary.get_full_argument(index);
    Some(primary.get_span(argument))
}
