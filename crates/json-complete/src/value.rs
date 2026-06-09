use crate::{Ident, Location, Node};

/// JSON 结构信息
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Object(Vec<Field>),
    Array(Box<Value>),
    String,
    Number,
    Bool,
}

impl Default for Value {
    fn default() -> Self {
        Self::Object(Vec::default())
    }
}

/// JSON 具名字段信息
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Field {
    pub key: String,
    pub value: Value,
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
    pub text: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentKind {
    Key,
    String,
    Number,
    Bool,
}

impl Field {
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
    pub value: &'a Value,
    pub description: &'a str,
}

impl<'a> FieldView<'a> {
    pub fn with_value(value: &'a Value) -> Self {
        Self {
            key: "",
            value,
            description: "",
        }
    }
}

impl From<FieldView<'_>> for Field {
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

impl Value {
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
                .filter(|Field { key, .. }| key.starts_with(input))
                .map(
                    |Field {
                         key,
                         value,
                         description,
                     }| {
                        let text = match value {
                            Self::Object(_) => vec![format!("{key}\": {{"), '}'.to_string()],
                            Self::Array(_) => vec![format!("{key}\": ["), ']'.to_string()],
                            Self::String => vec![format!("{key}\": \""), '"'.to_string()],
                            _ => vec![format!("{key}\": ")],
                        };
                        Completion {
                            name: key.to_string(),
                            kind: IdentKind::Key,
                            len: input.len(),
                            text,
                            description: description.to_string(),
                        }
                    },
                )
                .collect(),

            Ident::Value(input) if matches!(value, Self::Bool) => ["true", "false"]
                .iter()
                .filter(|name| name.starts_with(input))
                .map(|name| Completion {
                    name: name.to_string(),
                    kind: IdentKind::Bool,
                    len: input.len(),
                    text: vec![name.to_string()],
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
    fn schema() -> Value;

    fn to_schema(&self) -> Value {
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
/// # use json_complete::{Field, Value, json};
///
/// // 用于表达式演示的变量和函数
/// let dynamic = json!({ "foo": string });
/// fn computed() -> Value {
///     json!({ "bar": number })
/// }
///
/// let schema = json! {{
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
///     Value::Object(vec![
///         Field {
///             key: "name".to_string(),
///             value: Value::String,
///             description: "用户名".to_string(),
///         },
///         Field {
///             key: "age".to_string(),
///             value: Value::Number,
///             description: "".to_string(),
///         },
///         Field {
///             key: "tags".to_string(),
///             value: Value::Array(Box::new(Value::String)),
///             description: "标签列表".to_string(),
///         },
///         Field {
///             key: "address".to_string(),
///             value: Value::Object(vec![
///                 Field {
///                     key: "city".to_string(),
///                     value: Value::String,
///                     description: "城市".to_string(),
///                 },
///                 Field {
///                     key: "zip".to_string(),
///                     value: Value::Number,
///                     description: "".to_string(),
///                 },
///             ]),
///             description: "".to_string(),
///         },
///         Field {
///             key: "field".to_string(),
///             value: Value::Object(vec![
///                 Field {
///                     key: "foo".to_string(),
///                     value: Value::String,
///                     description: "".to_string(),
///                 }
///             ]),
///             description: "".to_string(),
///         },
///         Field {
///             key: "from_call".to_string(),
///             value: Value::Object(vec![
///                 Field {
///                     key: "bar".to_string(),
///                     value: Value::Number,
///                     description: "".to_string(),
///                 }
///             ]),
///             description: "".to_string(),
///         },
///     ])
/// );
/// ```
#[macro_export]
macro_rules! json {
    // 表达式
    ( ( $expr:expr ) ) => { $expr };
    // 基本类型
    (string) => {
        $crate::Value::String
    };
    (number) => {
        $crate::Value::Number
    };
    (bool) => {
        $crate::Value::Bool
    };
    // 数组: [ type ], 支持可选描述
    ([ $($inner:tt)+ ] $($desc:literal)?) => {
        $crate::Value::Array(Box::new($crate::json!($($inner)+)))
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
            fields.push($crate::Field {
                key: $key.to_string(),
                value: $crate::json!($value),
                description,
            });
        )*
        $crate::Value::Object(fields)
    }};
}

impl Value {
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
