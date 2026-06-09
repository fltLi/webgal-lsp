use std::iter;

use ranked_count::Counter;
use tower_lsp::lsp_types::*;

use crate::{complete::make_span, context::Context};

/// 补全语句类型
pub fn complete_command(input: &str, position: Position, context: &Context) -> Vec<CompletionItem> {
    let mut completions = complete_speaker(&context.ident.speaker, input, position);
    default_commands()
        .iter()
        .for_each(|command| command.complete(input, position, &mut completions));
    completions
}

fn complete_speaker(
    speakers: &Counter<String>,
    input: &str,
    position: Position,
) -> Vec<CompletionItem> {
    speakers
        .iter_by_count()
        .filter_map(|(name, _)| name.starts_with(input).then_some(name))
        .enumerate()
        .map(|(i, name)| CompletionItem {
            label: name.to_string(),
            label_details: Some(CompletionItemLabelDetails {
                detail: None,
                description: Some("人物".to_string()),
            }),
            kind: Some(CompletionItemKind::VARIABLE),
            sort_text: Some(format!("b{i:03}_{name}")),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: make_span(position, input.len()),
                new_text: format!("{name}:"),
            })),
            ..Default::default()
        })
        .collect()
}

struct CommandInfo {
    name: &'static str,
    description: &'static str,
    with_content: bool,
    templates: &'static [CommandTemplate],
}

struct CommandTemplate {
    name: &'static str,
    description: &'static str,
    template: &'static [&'static str],
}

impl CommandInfo {
    fn complete(&self, input: &str, position: Position, completions: &mut Vec<CompletionItem>) {
        // 语句类型补全
        if self.name.starts_with(input) {
            completions.push(CompletionItem {
                label: self.name.to_string(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: None,
                    description: Some(self.description.to_string()),
                }),
                kind: Some(CompletionItemKind::FUNCTION),
                sort_text: Some(format!("a000_{}", self.name)),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: make_span(position, input.len()),
                    new_text: if self.with_content {
                        format!("{}:", self.name)
                    } else {
                        format!("{};", self.name)
                    },
                })),
                ..Default::default()
            });
        }
        // 语句模板补全
        completions.extend(
            self.templates
                .iter()
                .filter(|CommandTemplate { name, .. }| name.starts_with(input))
                .map(
                    |CommandTemplate {
                         name,
                         description,
                         template,
                     }| CompletionItem {
                        label: name.to_string(),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: Some(if description.is_empty() {
                                self.description.to_string()
                            } else {
                                format!("{} ({description})", self.description)
                            }),
                        }),
                        kind: Some(CompletionItemKind::SNIPPET),
                        sort_text: Some(format!("a000_{name}")),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                            range: make_span(position, input.len()),
                            new_text: template
                                .iter()
                                .enumerate()
                                .map(|(i, token)| format!("{}${}", token, i + 1))
                                .chain(iter::once(";$0".to_string()))
                                .collect(),
                        })),
                        ..Default::default()
                    },
                ),
        );
    }
}

fn default_commands() -> &'static [CommandInfo] {
    &[
        // 常规演出
        CommandInfo {
            name: "say",
            description: "普通对话",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "say",
                    description: "",
                    template: &["", ":"],
                },
                CommandTemplate {
                    name: "say.figure",
                    description: "指定立绘",
                    template: &["", ":", " -figureId="],
                },
            ],
        },
        CommandInfo {
            name: "changeBg",
            description: "切换背景",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "changeBg.enter",
                    description: "",
                    template: &["changeBg:", " -transform=", " -next"],
                },
                CommandTemplate {
                    name: "changeBg.exit",
                    description: "清除",
                    template: &["changeBg: -next"],
                },
                CommandTemplate {
                    name: "changeBg.unlock",
                    description: "鉴赏",
                    template: &[
                        "changeBg:",
                        " -transform=",
                        " -unlockname=",
                        " -series=",
                        " -next",
                    ],
                },
            ],
        },
        CommandInfo {
            name: "changeFigure",
            description: "切换立绘",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "changeFigure.motion",
                    description: "修改动作",
                    template: &["changeFigure:", " -id=", " -motion=", " -next"],
                },
                CommandTemplate {
                    name: "changeFigure.expression",
                    description: "修改动作和表情",
                    template: &[
                        "changeFigure:",
                        " -id=",
                        " -motion=",
                        " -expression=",
                        " -next",
                    ],
                },
                CommandTemplate {
                    name: "changeFigure.enter",
                    description: "入场",
                    template: &[
                        "changeFigure:",
                        " -id=",
                        " -motion=",
                        " -transform=",
                        " -next",
                    ],
                },
                CommandTemplate {
                    name: "changeFigure.exit",
                    description: "退场",
                    template: &["changeFigure: -id=", " -next"],
                },
            ],
        },
        CommandInfo {
            name: "bgm",
            description: "背景音乐",
            with_content: true,
            templates: &[CommandTemplate {
                name: "bgm.unlock",
                description: "鉴赏",
                template: &["bgm:", " -unlockname=", " -series="],
            }],
        },
        CommandInfo {
            name: "playVideo",
            description: "播放视频",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "playEffect",
            description: "效果声音",
            with_content: true,
            templates: &[CommandTemplate {
                name: "playEffect.repeat",
                description: "循环",
                template: &["playEffect:", " -id"],
            }],
        },
        // 舞台对象控制
        CommandInfo {
            name: "setAnimation",
            description: "调用动画",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "setAnimation.next",
                    description: "连续执行",
                    template: &["setAnimation:", " -target=", " -next"],
                },
                CommandTemplate {
                    name: "setAnimation.keep",
                    description: "持续执行",
                    template: &["setAnimation:", " -target=", " -keep"],
                },
                CommandTemplate {
                    name: "setAnimation.parallel",
                    description: "同步执行",
                    template: &["setAnimation:", " -target=", " -parallel"],
                },
            ],
        },
        CommandInfo {
            name: "setComplexAnimation",
            description: "复杂动画",
            with_content: true,
            templates: &[CommandTemplate {
                name: "setComplexAnimation",
                description: "",
                template: &["setComplexAnimation:", " -target=", " -next"],
            }],
        },
        CommandInfo {
            name: "setTransform",
            description: "单段动画",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "setTransform.next",
                    description: "连续执行",
                    template: &["setTransform:", " -target=", " -duration=", " -next"],
                },
                CommandTemplate {
                    name: "setTransform.keep",
                    description: "持续执行",
                    template: &["setTransform:", " -target=", " -duration=", " -keep"],
                },
                CommandTemplate {
                    name: "setTransform.parallel",
                    description: "同步执行",
                    template: &["setTransform:", " -target=", " -duration=", " -parallel"],
                },
                CommandTemplate {
                    name: "setTransform.clear",
                    description: "清除",
                    template: &["setTransform:{} -target=", " -writeDefault", " -next"],
                },
            ],
        },
        CommandInfo {
            name: "setTempAnimation",
            description: "多段动画",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "setTempAnimation.next",
                    description: "连续执行",
                    template: &["setTempAnimation:", " -target=", " -next"],
                },
                CommandTemplate {
                    name: "setTempAnimation.keep",
                    description: "持续执行",
                    template: &["setTempAnimation:", " -target=", " -keep"],
                },
                CommandTemplate {
                    name: "setTempAnimation.parallel",
                    description: "同步执行",
                    template: &["setTempAnimation:", " -target=", " -parallel"],
                },
            ],
        },
        CommandInfo {
            name: "setTransition",
            description: "进出场动画",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "setTransition.enter",
                    description: "入场",
                    template: &["setTempAnimation:", " -target=", " -enter="],
                },
                CommandTemplate {
                    name: "setTransition.exit",
                    description: "出场",
                    template: &["setTempAnimation:", " -target=", " -exit="],
                },
            ],
        },
        // 特殊演出
        CommandInfo {
            name: "pixiPerform",
            description: "使用特效",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "pixiInit",
            description: "清除特效",
            with_content: false,
            templates: &[],
        },
        CommandInfo {
            name: "intro",
            description: "全屏文字",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "miniAvatar",
            description: "角落头像",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "setTextbox",
            description: "文本显示",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "filmMode",
            description: "电影模式",
            with_content: true,
            templates: &[],
        },
        // 场景与分支
        CommandInfo {
            name: "callScene",
            description: "调用场景",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "changeScene",
            description: "切换场景",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "choose",
            description: "分支选择",
            with_content: true,
            templates: &[CommandTemplate {
                name: "choose.default",
                description: "默认选项",
                template: &["choose:", " -defaultChoice="],
            }],
        },
        CommandInfo {
            name: "label",
            description: "标签",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "jumpLabel",
            description: "跳转标签",
            with_content: true,
            templates: &[],
        },
        // 鉴赏
        CommandInfo {
            name: "unlockCg",
            description: "鉴赏图片",
            with_content: true,
            templates: &[CommandTemplate {
                name: "unlockCg",
                description: "",
                template: &["unlockCg:", " -name=", " -series="],
            }],
        },
        CommandInfo {
            name: "unlockBgm",
            description: "鉴赏音乐",
            with_content: true,
            templates: &[CommandTemplate {
                name: "unlockBgm",
                description: "",
                template: &["unlockBgm:", " -name=", " -series="],
            }],
        },
        // 游戏控制
        CommandInfo {
            name: "getUserInput",
            description: "获取输入",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "getUserInput",
                    description: "",
                    template: &[
                        "getUserInput:",
                        " -title=",
                        " -buttonText=",
                        " -defaultValue=",
                    ],
                },
                CommandTemplate {
                    name: "getUserInput.validate",
                    description: "校验",
                    template: &[
                        "getUserInput:",
                        " -title=",
                        " -buttonText=",
                        " -defaultValue=",
                        " -rule=",
                        " -ruleFlag=",
                        " -ruleText=",
                        " -ruleButtonText=",
                    ],
                },
            ],
        },
        CommandInfo {
            name: "setVar",
            description: "设置变量",
            with_content: true,
            templates: &[
                CommandTemplate {
                    name: "setVar",
                    description: "",
                    template: &["setVar:", "="],
                },
                CommandTemplate {
                    name: "setVar.global",
                    description: "",
                    template: &["setVar:", "=", " -global"],
                },
            ],
        },
        CommandInfo {
            name: "showVars",
            description: "显示变量",
            with_content: false,
            templates: &[],
        },
        CommandInfo {
            name: "wait",
            description: "等待",
            with_content: true,
            templates: &[],
        },
        CommandInfo {
            name: "applyStyle",
            description: "应用样式",
            with_content: true,
            templates: &[CommandTemplate {
                name: "applyStyle",
                description: "",
                template: &["applyStyle:", "->"],
            }],
        },
        CommandInfo {
            name: "callSteam",
            description: "调用 Steam",
            with_content: true,
            templates: &[CommandTemplate {
                name: "callSteam.achivement",
                description: "解锁成就",
                template: &["callSteam:", " -achivementId="],
            }],
        },
        CommandInfo {
            name: "end",
            description: "结束游戏",
            with_content: false,
            templates: &[],
        },
    ]
}
