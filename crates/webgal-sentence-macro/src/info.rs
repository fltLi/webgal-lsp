use either::Either;
use proc_macro2::Span;
use quote::format_ident;
use syn::{Attribute, Error, Field, FieldsNamed, Ident, Path, Result, Type, spanned::Spanned};

use crate::attr::{SentenceAttr, SentenceAttrList};

pub struct SentenceInfo {
    pub ident: Ident,
    pub command: String,
    pub validate: Option<Path>,
    pub forward: Either<Path, Ident>,
    pub obsolete: Vec<(String, String)>,
    pub content: Option<ContentInfo>,
    pub condition: Option<Ident>,
    pub arguments: Vec<ArgumentInfo>,
}

pub struct ContentInfo {
    pub ident: Ident,
    pub ty: Type,
    pub serialize_with: Option<Path>,
    pub deserialize_with: Option<Path>,
    pub resource: Option<Either<Path, Ident>>,
}

pub struct ArgumentInfo {
    pub ident: Ident,
    pub ty: Type,
    pub kind: ArgumentKind,
    pub requires: Vec<String>,
}

pub enum ArgumentKind {
    Named {
        name: String,
        default: bool,
        serialize_with: Option<Path>,
        deserialize_with: Option<Path>,
        resource: Option<Either<Path, Ident>>,
    },
    Enum {
        variant: Vec<(String, Ident)>,
    },
}

impl SentenceInfo {
    pub fn from_ast(ident: Ident, attrs: &[Attribute], fields: &FieldsNamed) -> Result<Self> {
        let (command, validate, obsolete) = collect_struct(attrs)?;
        let forward = collect_forward(attrs, fields)?;
        let (content, condition, arguments) = collect_arguments(fields)?;

        Ok(Self {
            ident,
            command,
            validate,
            forward,
            obsolete,
            content,
            condition,
            arguments,
        })
    }
}

impl ArgumentInfo {
    pub fn get_variable(&self) -> Ident {
        format_ident!("__{}", self.ident)
    }
}

#[allow(clippy::type_complexity)]
fn collect_struct(attrs: &[Attribute]) -> Result<(String, Option<Path>, Vec<(String, String)>)> {
    let mut command = None;
    let mut validate = None;
    let mut obsolete = Vec::new();

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("sentence")) {
        let make_error = |msg| Err(Error::new(attr.span(), msg));

        for attr in attr.parse_args::<SentenceAttrList>()? {
            match attr {
                SentenceAttr::Command(cmd) => {
                    if command.is_some() {
                        return make_error("语句结构体上只能有一个 `command` 属性");
                    }
                    command = Some(cmd);
                }

                SentenceAttr::Validate(vld) => {
                    if validate.is_some() {
                        return make_error("语句结构体上只能有一个 `validate` 属性");
                    }
                    validate = Some(vld);
                }

                SentenceAttr::Obsolete(mut map) => {
                    obsolete.append(&mut map);
                }

                SentenceAttr::Forward(_) | SentenceAttr::Resource(_) => {}
                _ => return make_error("`sentence` 标注内只能带有 `command` 和 `obsolete` 属性"),
            }
        }
    }

    let command = command.ok_or_else(|| {
        Error::new(
            Span::call_site(),
            "语句结构体缺少 `#[sentence(command = ...)]` 属性",
        )
    })?;
    Ok((command, validate, obsolete))
}

fn collect_forward(attrs: &[Attribute], fields: &FieldsNamed) -> Result<Either<Path, Ident>> {
    let mut forward = None;

    // 扫描结构体
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("sentence")) {
        let make_error = |msg| Err(Error::new(attr.span(), msg));
        for attr in attr.parse_args::<SentenceAttrList>()? {
            if let SentenceAttr::Forward(fwd) = attr {
                if forward.is_some() {
                    return make_error("语句结构体上只能有一个 `forward` 属性");
                }
                if fwd.is_none() {
                    return make_error("结构体上全局 `forward` 属性必须接收时序枚举或函数");
                }
                forward = Some(Either::Left(fwd.unwrap()));
            }
        }
    }

    // 扫描字段
    for field in &fields.named {
        for attr in field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("sentence"))
        {
            let make_error = |msg| Err(Error::new(attr.span(), msg));
            for attr in attr.parse_args::<SentenceAttrList>()? {
                if let SentenceAttr::Forward(fwd) = attr {
                    if forward.is_some() {
                        return make_error("语句结构体上只能有一个 `forward` 属性");
                    }
                    if fwd.is_some() {
                        return make_error("字段上 `forward` 不能接收参数");
                    }
                    forward = Some(Either::Right(field.ident.clone().unwrap()));
                }
            }
        }
    }

    forward.ok_or_else(|| {
        Error::new(
            Span::call_site(),
            "语句结构体缺少 `#[sentence(forward = ...)]` 属性",
        )
    })
}

fn collect_arguments(
    fields: &FieldsNamed,
) -> Result<(Option<ContentInfo>, Option<Ident>, Vec<ArgumentInfo>)> {
    let mut content = None;
    let mut condition = None;
    let mut arguments = Vec::new();

    for field in &fields.named {
        let make_error = |msg| Err(Error::new(field.span(), msg));

        match FieldRole::from_field(field)? {
            FieldRole::Content(info) => {
                if content.is_some() {
                    return make_error("语句结构体只能有一个 `content` 字段");
                }
                content = Some(info);
            }

            FieldRole::Argument { info, is_condition } => {
                if is_condition {
                    if condition.is_some() {
                        return make_error("语句结构体只能有一个 `condition` 字段");
                    }
                    condition = Some(info.ident.clone());
                }

                arguments.push(info);
            }
        }
    }

    Ok((content, condition, arguments))
}

enum FieldRole {
    Content(ContentInfo),
    Argument {
        info: ArgumentInfo,
        is_condition: bool,
    },
}

impl FieldRole {
    fn from_field(field: &Field) -> Result<Self> {
        let ident = field.ident.clone().unwrap();
        let ty = field.ty.clone();

        let mut is_content = false;
        let mut is_condition = false;
        let mut name = None;
        let mut default = false;
        let mut serialize_with = None;
        let mut deserialize_with = None;
        let mut variant = None;
        let mut requires = Vec::new();
        let mut resource = None;

        for attr in field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("sentence"))
        {
            let make_error = |msg| Err(Error::new(attr.span(), msg));

            for attr in attr.parse_args::<SentenceAttrList>()? {
                match attr {
                    SentenceAttr::Content => {
                        if is_content {
                            return make_error("重复的 `content` 属性");
                        }
                        if name.is_some() || variant.is_some() {
                            return make_error("字段不能同时为 `content` 和 `argument`");
                        }
                        is_content = true;
                    }

                    SentenceAttr::Condition => {
                        if is_content {
                            return make_error("字段不能同时为 `content` 和 `condition`");
                        }
                        if is_condition {
                            return make_error("重复的 `condition` 属性");
                        }
                        is_condition = true;
                    }

                    SentenceAttr::Rename(rename) => {
                        if is_content {
                            return make_error("字段不能同时为 `content` 和 `argument`");
                        }
                        if name.is_some() {
                            return make_error("重复的 `argument` 属性");
                        }
                        name = Some(rename);
                    }

                    SentenceAttr::Default => {
                        if is_content {
                            return make_error("主参数不能具有 `default` 属性");
                        }
                        if default {
                            return make_error("重复的 `default` 属性");
                        }
                        default = true;
                    }

                    SentenceAttr::SerializeWith(fun) => {
                        if serialize_with.is_some() {
                            return make_error("重复的 `serialize_with` 属性");
                        }
                        serialize_with = Some(fun);
                    }

                    SentenceAttr::DeserializeWith(fun) => {
                        if deserialize_with.is_some() {
                            return make_error("重复的 `deserialize_with` 属性");
                        }
                        deserialize_with = Some(fun);
                    }

                    SentenceAttr::Variant(map) => {
                        if is_content {
                            return make_error("字段不能同时为 `content` 和 `argument (variant)`");
                        }
                        if variant.is_some() {
                            return make_error("重复的 `variant` 属性");
                        }
                        variant = Some(map);
                    }

                    SentenceAttr::Require(mut req) => {
                        if is_content {
                            return make_error("主参数不能具有 `require` 属性");
                        }
                        requires.append(&mut req);
                    }

                    SentenceAttr::Resource(res) => {
                        if resource.is_some() {
                            return make_error("重复的 `resource` 属性");
                        }
                        resource = Some(res);
                    }

                    SentenceAttr::Forward(_) => {}
                    _ => return make_error("字段不能含有 `command` 或 `obsolete` 属性"),
                }
            }
        }

        let make_error = |msg| Err(Error::new(field.span(), msg));

        if is_content {
            return Ok(Self::Content(ContentInfo {
                ident,
                ty,
                serialize_with,
                deserialize_with,
                resource,
            }));
        }

        let kind = match variant {
            // 枚举型参数
            Some(variant) => {
                if default
                    || serialize_with.is_some()
                    || deserialize_with.is_some()
                    || resource.is_some()
                {
                    return make_error(
                        "枚举型参数不能具有 `default`, `serialize_with`, `deserialize_with` 或 `resource` 参数",
                    );
                }
                ArgumentKind::Enum { variant }
            }
            // 具名参数
            None => ArgumentKind::Named {
                name: name.unwrap_or_else(|| ident.to_string()),
                default,
                serialize_with,
                deserialize_with,
                resource,
            },
        };

        Ok(Self::Argument {
            info: ArgumentInfo {
                ident,
                ty,
                kind,
                requires,
            },
            is_condition,
        })
    }
}
