use crate::complete::locate::*;

/// JSON 结构信息
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Schema {
    Object(Vec<SchemaField>),
    Array(Box<Schema>),
    String,
    Number,
    Bool,
}

impl Default for Schema {
    fn default() -> Self {
        Self::Object(Vec::default())
    }
}

/// JSON 具名字段信息
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaField {
    pub key: String,
    pub value: Schema,
    /// 补全时给出的描述信息
    pub description: String,
}

// -------- complete --------

/// JSON 输入补全信息
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Completion {
    pub name: String,
    pub kind: IdentKind,
    pub len: usize,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentKind {
    Key,
    String,
    Number,
    Bool,
}

impl SchemaField {
    pub fn as_view<'a>(&'a self) -> FieldView<'a> {
        let Self {
            key,
            value,
            description,
        } = self;
        FieldView {
            key,
            value,
            description,
        }
    }
}

/// JSON 具名字段信息视图
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldView<'a> {
    pub key: &'a str,
    pub value: &'a Schema,
    pub description: &'a str,
}

impl<'a> FieldView<'a> {
    pub fn with_value(value: &'a Schema) -> Self {
        Self {
            key: "",
            value,
            description: "",
        }
    }
}

impl From<FieldView<'_>> for SchemaField {
    fn from(value: FieldView) -> Self {
        let FieldView {
            key,
            value,
            description,
        } = value;
        Self {
            key: key.to_string(),
            value: value.clone(),
            description: description.to_string(),
        }
    }
}

impl Schema {
    /// 宽松解析 JSON 字符串并提供补全
    ///
    /// 简单封装 [`Self::complete_by_location`] 和 [`Location::locate`] 实现.
    pub fn complete(&self, s: &str) -> Vec<Completion> {
        self.complete_by_location(&Location::locate(s))
    }

    /// 依据 JSON 位置信息提供补全
    pub fn complete_by_location(&self, location: &Location) -> Vec<Completion> {
        let Location { path, ident } = location;
        let FieldView {
            value, description, ..
        } = match self.get_at_path(path) {
            Some(field) => field,
            None => return Vec::default(),
        };

        match *ident {
            Ident::Key(input) if let Self::Object(fields) = value => fields
                .iter()
                // .filter(|SchemaField { key, .. }| key.starts_with(input))
                .map(|field| {
                    // TODO: 根据光标后的内容更好地提供下列补全
                    // let text = match value {
                    //     Self::Object(_) => format!("{key}\":{{$1},$0"),
                    //     Self::Array(_) => format!("{key}\":[$1],$0"),
                    //     Self::String => format!("{key}\":\"$1\"$0"),
                    //     _ => format!("{key}\":"),
                    // };
                    Completion {
                        name: field.key.to_string(),
                        kind: IdentKind::Key,
                        len: input.len(),
                        description: field.description.to_string(),
                    }
                })
                .collect(),

            Ident::Value(input) if matches!(value, Self::Bool) => ["true", "false"]
                .iter()
                .filter(|name| name.starts_with(input))
                .map(|name| Completion {
                    name: name.to_string(),
                    kind: IdentKind::Bool,
                    len: input.len(),
                    description: description.to_string(),
                })
                .collect(),

            _ => Vec::default(),
        }
    }

    /// 依据 JSON 路径信息定位字段
    pub fn get_at_path<'a>(&'a self, path: &[Node]) -> Option<FieldView<'a>> {
        let mut field = FieldView::with_value(self);
        for &node in path {
            match node {
                Node::Field(key) if let Self::Object(fields) = field.value => {
                    field = fields.iter().find(|field| field.key == key)?.as_view();
                }
                Node::Array if let Self::Array(child) = field.value => {
                    field.value = child;
                }
                _ => return None,
            }
        }
        Some(field)
    }
}

// -------- schema --------

/// 支持获取 JSON 格式的类型
pub trait ToJsonSchema {
    fn schema() -> Schema;

    fn to_schema(&self) -> Schema {
        Self::schema()
    }
}

/// 构造 JSON 结构信息的宏
///
/// 用于描述 JSON 数据的结构模式, 可用于补全和验证.
/// 支持对象, 数组基本类型 (`string`, `number`, `bool`), 以及表达式.
/// 可以为每个字段添加可选的描述字符串, 直接标注在字段值后.
///
/// # Examples
/// ```
/// # use json_language_service::{Schema, SchemaField, json_schema};
///
/// // 用于表达式演示的变量和函数
/// let dynamic = json_schema!({ "foo": string });
/// fn computed() -> Schema {
///     json_schema!({ "bar": number })
/// }
///
/// let schema = json_schema! {{
///     "name": string "用户名",
///     "age":  number,
///     "tags": [string] "标签列表",
///     "address": {
///         "city": string "城市",
///         "zip":  number
///     },
///     // 嵌入表达式
///     "field":     (dynamic),
///     "from_call": (computed()),
/// }};
///
/// assert_eq!(
///     schema,
///     Schema::Object(vec![
///         SchemaField {
///             key: "name".to_string(),
///             value: Schema::String,
///             description: "用户名".to_string(),
///         },
///         SchemaField {
///             key: "age".to_string(),
///             value: Schema::Number,
///             description: "".to_string(),
///         },
///         SchemaField {
///             key: "tags".to_string(),
///             value: Schema::Array(Box::new(Schema::String)),
///             description: "标签列表".to_string(),
///         },
///         SchemaField {
///             key: "address".to_string(),
///             value: Schema::Object(vec![
///                 SchemaField {
///                     key: "city".to_string(),
///                     value: Schema::String,
///                     description: "城市".to_string(),
///                 },
///                 SchemaField {
///                     key: "zip".to_string(),
///                     value: Schema::Number,
///                     description: "".to_string(),
///                 },
///             ]),
///             description: "".to_string(),
///         },
///         SchemaField {
///             key: "field".to_string(),
///             value: Schema::Object(vec![
///                 SchemaField {
///                     key: "foo".to_string(),
///                     value: Schema::String,
///                     description: "".to_string(),
///                 }
///             ]),
///             description: "".to_string(),
///         },
///         SchemaField {
///             key: "from_call".to_string(),
///             value: Schema::Object(vec![
///                 SchemaField {
///                     key: "bar".to_string(),
///                     value: Schema::Number,
///                     description: "".to_string(),
///                 }
///             ]),
///             description: "".to_string(),
///         },
///     ])
/// );
/// ```
#[macro_export]
macro_rules! json_schema {
    // 表达式
    ( ( $expr:expr ) ) => { $expr };
    // 基本类型
    (string) => {
        $crate::Schema::String
    };
    (number) => {
        $crate::Schema::Number
    };
    (bool) => {
        $crate::Schema::Bool
    };
    // 数组: [ type ], 支持可选描述
    ([ $($inner:tt)+ ] $($desc:literal)?) => {
        $crate::Schema::Array(Box::new($crate::json_schema!($($inner)+)))
    };
    // 对象: { "key": value "desc", ... }
    // 递归规则: 匹配以逗号分隔的键值对, 每对中值后可跟一个可选的字符串描述.
    ({ $($key:literal : $value:tt $( $desc:literal )? ),* $(,)? }) => {{
        let mut fields = Vec::new();
        $(
            // 提取描述, 若无则空字符串
            let description = {
                let mut d = String::new();
                $(d = $desc.to_string();)?
                d
            };
            fields.push($crate::SchemaField {
                key: $key.to_string(),
                value: $crate::json_schema!($value),
                description,
            });
        )*
        $crate::Schema::Object(fields)
    }};
}

impl Schema {
    /// 继承 JSON 结构信息
    ///
    /// 仅支持 [`Self::Object`] 之间的继承.
    pub fn inherit(mut self, base: &Self) -> Self {
        if let Self::Object(child) = &mut self
            && let Self::Object(base) = base
        {
            child.extend_from_slice(base);
        }
        self
    }
}
