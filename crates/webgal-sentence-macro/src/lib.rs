use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields, parse_macro_input};

use crate::{
    code::{impl_display, impl_from_primary, impl_sentence_ext},
    info::SentenceInfo,
};

mod attr;
mod code;
mod info;

// TODO: 支持对 Option<T> 类型主参数提供 require 校验
// TODO: 为语句创建 Span 类型, 记录每个参数的区间, FromPrimary 时额外返回

/// WebGAL 语句序列化 / 反序列化派生宏
///
/// 此宏为类型实现:
/// * [`Self::get_command`] - 返回语句类型.
/// * [`FromPrimary`] - 支持从初级语句反序列化.
/// * [`Display`] - 支持序列化为语句字符串.
///
/// 此宏依赖 crate `webgal-model`,
/// 并由其重新导出至 [`webgal_model::sentence::Sentence`].
///
/// # Attributes
///
/// 此宏通过 `#[sentence(...)]` 接收下列选项:
///
/// ## 语句类型
/// 标注在结构体上, 形如: `command = "语句类型"`.
///
/// ## 语句校验
/// 标注在结构体上, 可选, 形如: `validate = 校验函数`.
/// ```rust,ignore
/// fn(&self, primary: &PrimarySentence, errors: &mut Vec<Error>)
/// ```
///
/// ## 弃用参数
/// 标注在结构体上, 可选, 形如: `obsolete = { "参数名": "弃用理由", ... }`.
///
/// ## 主参数
/// 标注在字段上, 形如: `content`.
/// 至多存在一个, 没有则默认主参数为 `None` 或空字符串.
///
/// 序列化时调用 [`Display`]; 反序列化时非 [`String`] 类型调用 [`FromStr`], 失败回退 [`Default`].
/// 支持[自定义序列化 / 反序列化](#自定义序列化-/-反序列化).
///
/// 若反序列化时为 `None`, 将按空字符串处理.
///
/// ## 语句参数
/// 标注在字段上, 不可与 `content` 重复出现, 具备两种类型:
///
/// ### 常规参数
/// * `rename = "参数名"` - 自定义参数名.
/// * `default` - (可选) 序列化时跳过默认值, 反序列化找不到参数时填充默认值.
///
/// 字段值即为序列化 / 反序列化对象.
/// 序列化时调用 [`Display`]; 反序列化时非 [`String`] 类型调用 [`FromStr`], 失败回退 [`Default`].
/// 支持[自定义序列化 / 反序列化](#自定义序列化-/-反序列化).
///
/// 下列类型有特殊的序列化 / 反序列化行为:
/// * [`bool`] - 识别 `-name=` / `-name=false` / `-name=0` 为 `false`, 其余为 `true`;
/// * [`Option`] - 序列化时将忽略 `None`, 反序列化时将默认填充 `None` (自定义序列化 / 反序列化仍适用).
///
/// ### 枚举型参数
/// * `variant = { "参数名": 枚举项, ... }` - 提供枚举映射, 用于序列化和反序列化.
///
/// 要求实现下列 trait:
/// * [`Default`] + [`Eq`] - 序列化时其将作为默认值, 反序列化时则忽略默认值.
/// * [`Ord`] - 反序列化时若出现多个枚举, 将保留最大值.
///
/// ## 参数依赖
/// 标注在参数上, 可选, 形如 `require = ["依赖参数名"]`.
/// 反序列化时, 若存在依赖者的同时不存在被依赖者, 则记录错误.
///
/// ## 自定义序列化 / 反序列化
/// * `serialize_with = 序列化函数`
///   ```rust,ignore
///   fn(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
///   ```
/// * `deserialize_with = 反序列化函数`
///   ```rust,ignore
///   fn(&str) -> (Self, Option<anyhow::Error>)
///   ```
#[proc_macro_derive(Sentence, attributes(sentence))]
pub fn derive_sentence(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(f),
            ..
        }) => f,
        _ => panic!("WebGAL 语句派生宏只能作用于具名结构体"),
    };

    let info = SentenceInfo::from_ast(name, &input.attrs, &fields).expect("语句结构体属性不合法");

    let sentence_ext = impl_sentence_ext(&info);
    let from_primary = impl_from_primary(&info);
    let display = impl_display(&info);

    let expanded = quote! {
        #sentence_ext
        #from_primary
        #display
    };
    expanded.into()
}
