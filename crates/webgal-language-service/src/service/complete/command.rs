use count::HashCounter;
use lsp_types::*;

use crate::{
    project::Project,
    service::complete::{PrimaryCompletion, make_span},
};

/// 补全语句类型
pub fn complete_command(
    input: &str,
    position: Position,
    project: &Project,
) -> Vec<PrimaryCompletion> {
    let mut completions = complete_speaker(&project.ident().speaker, input, position);
    default_commands()
        .iter()
        .for_each(|command| command.complete(input, position, &mut completions));
    completions
}

fn complete_speaker(
    speakers: &HashCounter<String>,
    input: &str,
    position: Position,
) -> Vec<PrimaryCompletion> {
    speakers
        .iter_with_count()
        .filter(|(name, _)| name.starts_with(input))
        .map(|(name, count)| PrimaryCompletion {
            name: name.clone(),
            kind: CompletionItemKind::VARIABLE,
            description: Some("人物".to_string()),
            sort_key: Some(format!("b{:016x}{name}", !count)),
            span: make_span(position, input.len()),
            insert_text: Some(format!("{name}:")),
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
    template: &'static str,
}

impl CommandInfo {
    fn complete(&self, input: &str, position: Position, completions: &mut Vec<PrimaryCompletion>) {
        // 语句类型补全
        if self.name.starts_with(input) {
            completions.push(PrimaryCompletion {
                name: self.name.to_string(),
                kind: CompletionItemKind::FUNCTION,
                description: Some(self.description.to_string()),
                sort_key: Some(format!("a{}", self.name)),
                span: make_span(position, input.len()),
                insert_text: Some(if self.with_content {
                    format!("{}:", self.name)
                } else {
                    format!("{};", self.name)
                }),
            });
        }

        // 语句模板补全
        completions.extend(
            self.templates
                .iter()
                .filter(|CommandTemplate { name, .. }| name.starts_with(input))
                .map(
                    |&CommandTemplate {
                         name,
                         description,
                         template,
                     }| PrimaryCompletion {
                        name: name.to_string(),
                        kind: CompletionItemKind::SNIPPET,
                        description: Some(if description.is_empty() {
                            self.description.to_string()
                        } else {
                            format!("{} ({description})", self.description)
                        }),
                        sort_key: Some(format!("a{name}")),
                        span: make_span(position, input.len()),
                        insert_text: Some(template.to_string()),
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
                    template: "$1:$2;$0",
                },
                CommandTemplate {
                    name: "say.figure",
                    description: "指定立绘",
                    template: "$1:$2 -figureId=$3;$0",
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
                    template: "changeBg:$1 -transform=$2 -next;$0",
                },
                CommandTemplate {
                    name: "changeBg.exit",
                    description: "清除",
                    template: "changeBg:$1 -next;$0",
                },
                CommandTemplate {
                    name: "changeBg.unlock",
                    description: "鉴赏",
                    template: "changeBg:$1 -transform=$2 -unlockname=$3 -series=$4 -next;$0",
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
                    template: "changeFigure:$1 -id=$2 -motion=$3 -next;$0",
                },
                CommandTemplate {
                    name: "changeFigure.expression",
                    description: "修改动作和表情",
                    template: "changeFigure:$1 -id=$2 -motion=$3 -expression=$4 -next;$0",
                },
                CommandTemplate {
                    name: "changeFigure.enter",
                    description: "入场",
                    template: "changeFigure:$1 -id=$2 -motion=$3 -transform=$4 -next;$0",
                },
                CommandTemplate {
                    name: "changeFigure.enterLeft",
                    description: "入场 (左侧)",
                    template: "changeFigure:$1 -left -id=$2 -motion=$3 -transform=$4 -next;$0",
                },
                CommandTemplate {
                    name: "changeFigure.enterRight",
                    description: "入场 (右侧)",
                    template: "changeFigure:$1 -right -id=$2 -motion=$3 -transform=$4 -next;$0",
                },
                CommandTemplate {
                    name: "changeFigure.exit",
                    description: "退场",
                    template: "changeFigure: -id=$1 -next;$0",
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
                template: "bgm:$1 -unlockname=$2 -series=$3;$0",
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
                template: "playEffect:$1 -id=$2;$0",
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
                    template: "setAnimation:$1 -target=$2 -next;$0",
                },
                CommandTemplate {
                    name: "setAnimation.keep",
                    description: "持续执行",
                    template: "setAnimation:$1 -target=$2 -keep -next;$0",
                },
                CommandTemplate {
                    name: "setAnimation.parallel",
                    description: "同步执行",
                    template: "setAnimation:$1 -target=$2 -parallel -next;$0",
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
                template: "setComplexAnimation:$1 -target=$2 -next;$0",
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
                    template: "setTransform:$1 -target=$2 -duration=$3 -next;$0",
                },
                CommandTemplate {
                    name: "setTransform.keep",
                    description: "持续执行",
                    template: "setTransform:$1 -target=$2 -duration=$3 -keep -next;$0",
                },
                CommandTemplate {
                    name: "setTransform.parallel",
                    description: "同步执行",
                    template: "setTransform:$1 -target=$2 -duration=$3 -parallel -next;$0",
                },
                CommandTemplate {
                    name: "setTransform.clear",
                    description: "清除",
                    template: "setTransform:$1 -target=$2 -writeDefault -next;$0",
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
                    template: "setTempAnimation:$1 -target=$2 -next;$0",
                },
                CommandTemplate {
                    name: "setTempAnimation.keep",
                    description: "持续执行",
                    template: "setTempAnimation:$1 -target=$2 -keep -next;$0",
                },
                CommandTemplate {
                    name: "setTempAnimation.parallel",
                    description: "同步执行",
                    template: "setTempAnimation:$1 -target=$2 -parallel -next;$0",
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
                    template: "setTempAnimation:$1 -target=$2 -enter=$3;$0",
                },
                CommandTemplate {
                    name: "setTransition.exit",
                    description: "出场",
                    template: "setTempAnimation:$1 -target=$2 -exit=$3;$0",
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
            templates: &[
                CommandTemplate {
                    name: "choose.three",
                    description: "三选一",
                    template: "choose:$1:$2|$3:$4|$5:$6;$0",
                },
                CommandTemplate {
                    name: "choose.default",
                    description: "默认选项",
                    template: "choose:$1 -defaultChoice=$2;$0",
                },
            ],
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
                template: "unlockCg:$1 -name=$2 -series=$3;$0",
            }],
        },
        CommandInfo {
            name: "unlockBgm",
            description: "鉴赏音乐",
            with_content: true,
            templates: &[CommandTemplate {
                name: "unlockBgm",
                description: "",
                template: "unlockBgm:$1 -name=$2 -series=$3;$0",
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
                    template: "getUserInput:$1 -title=$2 -buttonText=$3 -defaultValue=$4;$0",
                },
                CommandTemplate {
                    name: "getUserInput.validate",
                    description: "校验",
                    template: concat!(
                        "getUserInput:$1 -title=$2 -buttonText=$3 -defaultValue=$4",
                        " -rule=$5 -ruleFlag=$6 -ruleText=$7 -ruleButtonText=$8;$0"
                    ),
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
                    template: "setVar:$1=$2;$0",
                },
                CommandTemplate {
                    name: "setVar.global",
                    description: "",
                    template: "setVar:$1=$2 -global;$0",
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
                template: "applyStyle:$1->$2;$0",
            }],
        },
        CommandInfo {
            name: "callSteam",
            description: "调用 Steam",
            with_content: true,
            templates: &[CommandTemplate {
                name: "callSteam.achivement",
                description: "解锁成就",
                template: "callSteam:$1 -achivementId=$2;$0",
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
