use std::{collections::HashMap, iter};

use itertools::Either;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericArgument, Path, PathArguments, Type, TypePath};

use crate::info::{ArgumentInfo, ArgumentKind, FieldInfo, SentenceInfo};

// -------- get_command --------

pub fn impl_get_command(info: &SentenceInfo) -> TokenStream {
    let SentenceInfo { ident, command, .. } = info;
    quote! {
        #[automatically_derived]
        impl #ident {
            pub fn get_command(&self) -> &'static str {
                #command
            }
        }
    }
}

// -------- FromPrimary --------

pub fn impl_from_primary(info: &SentenceInfo) -> TokenStream {
    let SentenceInfo {
        ident,
        validate,
        obsolete,
        content,
        arguments,
        ..
    } = info;
    let arguments_map = try_map_arguments(info).expect("出现重复的参数");

    let validate = validate.as_ref().map(gen_validate).unwrap_or_default();
    let content_parse = gen_content_parse(content);
    let content_collect = gen_content_collect(content);

    let arguments_variable = arguments.iter().map(gen_argument_variable);
    let arguments_parse = arguments.iter().map(gen_argument_parse);
    let arguments_collect = arguments.iter().map(gen_argument_collect);
    let arguments_require_validate = arguments
        .iter()
        .map(|argument| gen_argument_requires_validate(&arguments_map, argument));
    let obsoletes_validate = obsolete
        .iter()
        .map(|(argument, reason)| gen_obsolete_validate(argument, reason));

    quote! {
        #[automatically_derived]
        impl crate::sentence::FromPrimary for #ident {
            fn from_primary(
                primary: &crate::sentence::PrimarySentence,
                errors: &mut Vec<crate::sentence::Error>
            ) -> Self {
                use ::std::str::FromStr;

                use crate::sentence::{Error::*, PrimarySentence};

                let PrimarySentence {
                    command, content, arguments, ..
                } = primary;

                #content_parse

                #(#arguments_variable)*
                for (i, &(name, value)) in arguments.iter().enumerate() {
                    match name {
                        #(#arguments_parse)*
                        #(#obsoletes_validate)*
                        _ => errors.push(ArgumentUnknown(i))
                    }
                }
                #(#arguments_require_validate)*

                let sentence = Self {
                    #content_collect
                    #(#arguments_collect)*
                };
                #validate
                sentence
            }
        }
    }
}

fn gen_validate(validate: &Path) -> TokenStream {
    quote! {
        #validate(&sentence, primary, errors);
    }
}

fn gen_content_parse(content: &Option<FieldInfo>) -> TokenStream {
    match content {
        // 有参数 + 自定义反序列化
        Some(FieldInfo {
            deserialize_with: Some(deserialize_with),
            ..
        }) => quote! {
            let (content, error) = #deserialize_with(content.unwrap_or(""));
            if let Some(error) = error {
                errors.push(ContentType(error));
            }
        },

        // 有参数 + String 类型
        Some(FieldInfo { ty, .. }) if is_string_type(ty) => quote! {
            let content = content.unwrap_or("").to_string();
        },

        // 有参数
        Some(FieldInfo { ty, .. }) => quote! {
            let content = content
                .unwrap_or("")
                .parse::<#ty>()
                .unwrap_or_else(|error| {
                    errors.push(ContentType(::anyhow::anyhow!(error)));
                    Default::default()
                });
        },

        // 空参数
        None => quote! {
            if !matches!(content, Some("") | None) {
                errors.push(ContentType(::anyhow::anyhow!("主参数应为空")));
            }
        },
    }
}

fn gen_content_collect(content: &Option<FieldInfo>) -> TokenStream {
    match content {
        Some(FieldInfo { ident, .. }) => quote! {
            #ident: content,
        },
        None => Default::default(),
    }
}

fn gen_argument_variable(argument: &ArgumentInfo) -> TokenStream {
    let variable = argument.get_variable();
    quote! { let mut #variable = None; }
}

fn gen_argument_parse(argument: &ArgumentInfo) -> TokenStream {
    let ArgumentInfo { ty, kind, .. } = argument;
    let variable = argument.get_variable();

    let argument_repeated_check = quote! {
        if #variable.is_some() {
            errors.push(ArgumentRepeated(i));
            continue;
        }
    };

    match kind {
        ArgumentKind::Named { .. } if is_option_bool_type(ty) => {
            panic!("Option<bool> 类型没有意义, 请改为 bool")
        }

        // 具名参数 + 自定义反序列化
        ArgumentKind::Named {
            name,
            deserialize_with: Some(deserialize_with),
            ..
        } => quote! {
            #name => {
                #argument_repeated_check
                #variable = Some({
                    let (value, error) = #deserialize_with(value.unwrap_or("true"));
                    if let Some(error) = error {
                        errors.push(ArgumentType(i, error));
                    }
                    (i, value)
                });
            }
        },

        // 具名参数 + bool 类型
        ArgumentKind::Named { name, .. } if is_bool_type(ty) => quote! {
            #name => {
                #argument_repeated_check
                #variable = Some((
                    i,
                    !matches!(value, Some("") | Some("false") | Some("0")),
                ));
            }
        },

        // 具名参数 + Option<String> 类型
        ArgumentKind::Named { name, .. } if is_option_string_type(ty) => quote! {
            #name => {
                #argument_repeated_check
                #variable = Some((i, value.unwrap_or("true").to_string()));
            }
        },

        // 具名参数
        ArgumentKind::Named { name, .. } => {
            let ty = option_inner_type_of(ty).unwrap_or(ty); // 解包 Option 内部类型
            quote! {
                #name => {
                    #argument_repeated_check
                    match value.unwrap_or("true").parse::<#ty>() {
                        Ok(value) => #variable = Some((i, value)),
                        Err(error) => errors.push(ArgumentType(i, ::anyhow::anyhow!(error))),
                    }
                }
            }
        }

        // 枚举型参数
        // TODO: 独立校验每个枚举项是否重复
        ArgumentKind::Enum { variant } => {
            let variant_display = variant.iter().map(|(name, variant)| {
                quote! {
                    #name => {
                        if matches!(value, Some("") | Some("false") | Some("0")) {
                            continue;
                        }
                        match #variable {
                            Some((_, previous)) => {
                                errors.push(ArgumentRepeated(i));
                                if #ty::#variant > previous {
                                    #variable = Some((i, #ty::#variant));
                                }
                            },
                            None => #variable = Some((i, #ty::#variant)),
                        }
                    }
                }
            });
            quote! { #(#variant_display)* }
        }
    }
}

fn gen_argument_requires_validate(
    arguments_map: &HashMap<&str, Option<&ArgumentInfo>>,
    argument: &ArgumentInfo,
) -> TokenStream {
    let ArgumentInfo {
        ident, requires, ..
    } = argument;
    let variable = argument.get_variable();

    if requires.is_empty() {
        return Default::default();
    }

    let validates = requires
        .iter()
        .map(|require| {
            (
                require,
                arguments_map
                    .get(require.as_str())
                    .unwrap_or_else(|| panic!("`{ident}` 参数依赖的参数 `{require}` 不存在"))
                    .unwrap_or_else(|| panic!("`{ident}` 参数依赖的参数 `{require}` 已弃用")),
            )
        })
        .map(|(require, argument)| {
            let ArgumentInfo { ty, kind, .. } = argument;
            let variable = argument.get_variable();

            match kind {
                ArgumentKind::Named { .. } => quote! {
                    if #variable.is_none() {
                        missings.push(#require);
                    }
                },

                ArgumentKind::Enum { variant } => {
                    let variant = variant
                        .iter()
                        .find_map(|(name, ident)| (name == require).then_some(ident))
                        .unwrap();
                    quote! {
                        if !matches!(#variable, Some((_, #ty::#variant))) {
                            missings.push(#require);
                        }
                    }
                }
            }
        });

    quote! {
        if let Some((i, _)) = &#variable {
            let mut missings = Vec::new();
            #(#validates)*
            if !missings.is_empty() {
                errors.push(ArgumentMissingDependencies(*i, missings));
            }
        }
    }
}

fn gen_argument_collect(argument: &ArgumentInfo) -> TokenStream {
    let ArgumentInfo {
        ident, ty, kind, ..
    } = argument;
    let variable = argument.get_variable();

    match kind {
        // 具名参数 + Option 类型
        ArgumentKind::Named { .. } if is_option_type(ty) => quote! {
            #ident: #variable.map(|(_, value)| value),
        },

        ArgumentKind::Named { default: false, .. } if !is_bool_type(ty) => {
            panic!(
                "非 `Option` 或 `bool` 类型的参数需要标注 `default` 在反序列化找不到参数时填充默认值"
            )
        }

        // 具名参数
        _ => quote! {
            #ident: #variable.map(|(_, value)| value).unwrap_or_default(),
        },
    }
}

fn gen_obsolete_validate(argument: &str, reason: &str) -> TokenStream {
    quote! {
        #argument => errors.push(ArgumentObsolete(i, #reason)),
    }
}

// -------- Display --------

pub fn impl_display(info: &SentenceInfo) -> TokenStream {
    let SentenceInfo {
        ident,
        command,
        content,
        arguments,
        ..
    } = info;

    let (content_display, no_content) = match content {
        Some(content) => (gen_content_display(content), false),
        None => (quote! { let mut has_argument = false; }, true),
    };
    let arguments_display = arguments
        .iter()
        .map(|argument| gen_argument_display(argument, no_content));

    quote! {
        #[automatically_derived]
        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                use ::std::fmt::Write;

                f.write_str(#command)?;
                #content_display
                #(#arguments_display)* // 编译器会优化掉必定触发 has_argument 检查的参数后的检查
                f.write_char(';')
            }
        }
    }
}

fn gen_content_display(content: &FieldInfo) -> TokenStream {
    let FieldInfo {
        ident,
        serialize_with,
        ..
    } = content;

    match serialize_with {
        Some(serialize_with) => quote! {
            f.write_char(':')?;
            #serialize_with(&self.#ident, f)?;
        },
        None => quote! { write!(f, ":{}", self.#ident)?; },
    }
}

fn gen_argument_display(argument: &ArgumentInfo, no_content: bool) -> TokenStream {
    let ArgumentInfo {
        ident, ty, kind, ..
    } = argument;

    let no_content_check = if no_content {
        quote! {
            if !has_argument {
                f.write_char(':')?;
                has_argument = true;
            }
        }
    } else {
        Default::default()
    };

    match kind {
        ArgumentKind::Named { default: true, .. } if is_option_type(ty) => {
            panic!("Option 类型默认跳过 None, 不支持 `default` 参数")
        }

        ArgumentKind::Named { default: true, .. } if is_bool_type(ty) => {
            panic!("bool 类型默认跳过 false, 不支持 `default` 参数")
        }

        // 具名参数 + 自定义序列化 + Option 类型
        ArgumentKind::Named {
            name,
            serialize_with: Some(serialize_with),
            ..
        } if is_option_type(ty) => quote! {
            if let Some(value) = &self.#ident {
                #no_content_check
                write!(f, " -{}=", #name)?;
                #serialize_with(value, f)?;
            }
        },

        // 具名参数 + 自定义序列化 + 跳过默认值
        ArgumentKind::Named {
            name,
            default: true,
            serialize_with: Some(serialize_with),
            ..
        } => quote! {
            if self.#ident != Default::default() {
                #no_content_check
                write!(f, " -{}=", #name)?;
                #serialize_with(&self.#ident, f)?;
            }
        },

        // 具名参数 + 自定义序列化
        ArgumentKind::Named {
            name,
            serialize_with: Some(serialize_with),
            ..
        } => quote! {
            #no_content_check
            write!(f, " -{}=", #name)?;
            #serialize_with(&self.#ident, f)?;
        },

        // 具名参数 + bool 类型
        ArgumentKind::Named { name, .. } if is_bool_type(ty) => quote! {
            if self.#ident {
                #no_content_check
                write!(f, " -{}", #name)?;
            }
        },

        // 具名参数 + Option 类型
        ArgumentKind::Named { name, .. } if is_option_type(ty) => quote! {
            if let Some(value) = &self.#ident {
                #no_content_check
                write!(f, " -{}={}", #name, value)?;
            }
        },

        // 具名参数 + 跳过默认值
        ArgumentKind::Named {
            name,
            default: true,
            ..
        } => quote! {
            if self.#ident != Default::default() {
                #no_content_check
                write!(f, " -{}={}", #name, self.#ident)?;
            }
        },

        // 具名参数
        ArgumentKind::Named { name, .. } => quote! {
            #no_content_check
            write!(f, " -{}={}", #name, self.#ident)?;
        },

        // 枚举型参数
        ArgumentKind::Enum { variant } => {
            let variant_display = variant
                .iter()
                .map(|(name, variant)| quote! { #ty::#variant => #name, });
            quote! {
                if self.#ident != Default::default() {
                    #no_content_check
                    write!(
                        f,
                        " -{}",
                        match self.#ident {
                            #(#variant_display)*
                            _ => return Err(::std::fmt::Error),
                        },
                    )?;
                }
            }
        }
    }
}

// -------- util --------

fn is_bool_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => path.is_ident("bool"),
        _ => false,
    }
}

fn is_string_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => path.is_ident("String"),
        _ => false,
    }
}

fn is_option_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            path.segments.last().is_some_and(|s| s.ident == "Option")
        }
        _ => false,
    }
}

fn is_option_bool_type(ty: &Type) -> bool {
    option_inner_type_of(ty).is_some_and(is_bool_type)
}

fn is_option_string_type(ty: &Type) -> bool {
    option_inner_type_of(ty).is_some_and(is_string_type)
}

fn option_inner_type_of(ty: &Type) -> Option<&Type> {
    let seg = match ty {
        Type::Path(TypePath { path, .. }) => match path.segments.last() {
            Some(seg) if seg.ident == "Option" => seg,
            _ => return None,
        },
        _ => return None,
    };

    match &seg.arguments {
        PathArguments::AngleBracketed(args) => {
            if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                Some(inner_ty)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn try_map_arguments(info: &SentenceInfo) -> Result<HashMap<&str, Option<&ArgumentInfo>>, &str> {
    let SentenceInfo {
        obsolete,
        arguments,
        ..
    } = info;
    let mut map = HashMap::new();
    arguments
        .iter()
        .flat_map(|argument| match &argument.kind {
            ArgumentKind::Named { name, .. } => {
                Either::Left(iter::once((name.as_str(), Some(argument))))
            }
            ArgumentKind::Enum { variant } => Either::Right(
                variant
                    .iter()
                    .map(|(name, _)| (name.as_str(), Some(&*argument))),
            ),
        })
        .chain(obsolete.iter().map(|(name, _)| (name.as_str(), None)))
        .find_map(|(name, argument)| map.insert(name, argument).map(|_| name))
        .map_or(Ok(map), Err)
}
