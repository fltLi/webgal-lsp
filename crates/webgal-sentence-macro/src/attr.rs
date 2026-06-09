use std::vec;

use syn::{
    Ident, LitStr, Path, Result, Token, braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub enum SentenceAttr {
    Command(String),
    Validate(Path),
    Obsolete(Vec<(String, String)>),
    Content,
    Rename(String),
    Default,
    SerializeWith(Path),
    DeserializeWith(Path),
    Variant(Vec<(String, Ident)>),
    Require(Vec<String>),
}

pub struct SentenceAttrList(pub Vec<SentenceAttr>);

impl Parse for SentenceAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        let ident = if lookahead.peek(Ident) {
            input.parse::<Ident>()?.to_string()
        } else {
            return Err(lookahead.error());
        };

        match ident.as_str() {
            "command" => {
                input.parse::<Token![=]>()?;
                Ok(Self::Command(input.parse::<LitStr>()?.value()))
            }
            "obsolete" => {
                input.parse::<Token![=]>()?;
                Ok(Self::Obsolete(
                    input
                        .parse::<MapExpr<LitStr, LitStr>>()?
                        .into_iter()
                        .map(|(argument, reason)| (argument.value(), reason.value()))
                        .collect(),
                ))
            }
            "validate" => {
                input.parse::<Token![=]>()?;
                Ok(Self::Validate(input.parse()?))
            }
            "content" => Ok(Self::Content),
            "rename" => {
                input.parse::<Token![=]>()?;
                Ok(Self::Rename(input.parse::<LitStr>()?.value()))
            }
            "default" => Ok(Self::Default),
            "serialize_with" => {
                input.parse::<Token![=]>()?;
                Ok(Self::SerializeWith(input.parse()?))
            }
            "deserialize_with" => {
                input.parse::<Token![=]>()?;
                Ok(Self::DeserializeWith(input.parse()?))
            }
            "variant" => {
                input.parse::<Token![=]>()?;
                Ok(Self::Variant(
                    input
                        .parse::<MapExpr<LitStr, Ident>>()?
                        .into_iter()
                        .map(|(name, variant)| (name.value(), variant))
                        .collect(),
                ))
            }
            "require" => {
                input.parse::<Token![=]>()?;
                let content;
                bracketed!(content in input);
                let list = Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?;
                let requires = list.into_iter().map(|lit| lit.value()).collect();
                Ok(Self::Require(requires))
            }
            _ => Err(lookahead.error()),
        }
    }
}

impl Parse for SentenceAttrList {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Punctuated::<SentenceAttr, Token![,]>::parse_terminated(input)?;
        Ok(SentenceAttrList(attrs.into_iter().collect()))
    }
}

impl IntoIterator for SentenceAttrList {
    type Item = SentenceAttr;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

struct MapExpr<K: Parse, V: Parse>(Vec<(K, V)>);

impl<K: Parse, V: Parse> Parse for MapExpr<K, V> {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        braced!(content in input);

        let mut map = Vec::new();
        while !content.is_empty() {
            let key: K = content.parse()?;
            content.parse::<Token![:]>()?;
            let value: V = content.parse()?;
            map.push((key, value));

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if content.is_empty() {
                break;
            } else {
                return Err(content.error("映射需要以 `,` 或者 `}` 结尾"));
            }
        }

        Ok(Self(map))
    }
}

impl<K: Parse, V: Parse> From<MapExpr<K, V>> for Vec<(K, V)> {
    fn from(value: MapExpr<K, V>) -> Self {
        value.0
    }
}

impl<K: Parse, V: Parse> IntoIterator for MapExpr<K, V> {
    type Item = (K, V);
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
