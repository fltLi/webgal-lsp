use proc_macro2::Span;
use quote::format_ident;
use syn::{Attribute, Error, Field, FieldsNamed, Ident, Path, Result, Type, spanned::Spanned};

use crate::attr::{SentenceAttr, SentenceAttrList};

pub struct SentenceInfo {
    pub ident: Ident,
    pub command: String,
    pub validate: Option<Path>,
    pub obsolete: Vec<(String, String)>,
    pub content: Option<FieldInfo>,
    pub arguments: Vec<ArgumentInfo>,
}

pub struct FieldInfo {
    pub ident: Ident,
    pub ty: Type,
    pub serialize_with: Option<Path>,
    pub deserialize_with: Option<Path>,
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
    },
    Enum {
        variant: Vec<(String, Ident)>,
    },
}

impl SentenceInfo {
    pub fn from_ast(ident: Ident, attrs: &[Attribute], fields: &FieldsNamed) -> Result<Self> {
        let (command, validate, obsolete) = collect_struct(attrs)?;
        let (content, arguments) = collect_arguments(fields)?;
        Ok(Self {
            ident,
            command,
            validate,
            obsolete,
            content,
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

fn collect_arguments(fields: &FieldsNamed) -> Result<(Option<FieldInfo>, Vec<ArgumentInfo>)> {
    let mut content = None;
    let mut arguments = Vec::new();

    for field in &fields.named {
        match FieldRole::from_field(field)? {
            FieldRole::Content(info) => {
                if content.is_some() {
                    return Err(Error::new(
                        field.span(),
                        "语句结构体只能有一个 `content` 字段",
                    ));
                }
                content = Some(info);
            }
            FieldRole::Argument(info) => arguments.push(info),
        }
    }

    Ok((content, arguments))
}

enum FieldRole {
    Content(FieldInfo),
    Argument(ArgumentInfo),
}

impl FieldRole {
    fn from_field(field: &Field) -> Result<Self> {
        let ident = field.ident.clone().unwrap();
        let ty = field.ty.clone();

        let mut is_content = false;
        let mut name = None;
        let mut default = false;
        let mut serialize_with = None;
        let mut deserialize_with = None;
        let mut variant = None;
        let mut requires = Vec::new();

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

                    _ => return make_error("字段不能含有 `command` 或 `obsolete` 属性"),
                }
            }
        }

        let make_error = |msg| Err(Error::new(field.span(), msg));

        if is_content {
            return Ok(Self::Content(FieldInfo {
                ident,
                ty,
                serialize_with,
                deserialize_with,
            }));
        }

        let kind = match variant {
            // 枚举型参数
            Some(variant) => {
                if default || serialize_with.is_some() || deserialize_with.is_some() {
                    return make_error(
                        "枚举型参数不能具有 `default`, `serialize_with` 和 `deserialize_with` 参数",
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
            },
        };

        Ok(Self::Argument(ArgumentInfo {
            ident,
            ty,
            kind,
            requires,
        }))
    }
}
