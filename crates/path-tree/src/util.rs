/// 路径分隔符
pub const PATH_SEPARATORS: &[char] = &['/', '\\'];

/// 规范化路径
///
/// 此操作将对路径进行以下处理:
/// * 统一 `\`, `\\`, `//` 等分隔符为 `/`, 并移除空段和末尾分隔符.
/// * 识别并转换相对路径 `.`, 当目录超过根时返回 `None`.
///
/// 规范后的路径形如: `game/scene/start.txt`.
///
/// # Examples
/// ```
/// # use path_tree::canonicalize;
///
/// assert_eq!(canonicalize("a\\b//c"), Some("a/b/c".to_string()));
/// assert_eq!(canonicalize("./a/../b"), Some("b".to_string()));
/// assert_eq!(canonicalize(".."), None);
/// assert_eq!(canonicalize("a/"), Some("a".to_string()));
/// assert_eq!(canonicalize("a/./b"), Some("a/b".to_string()));
/// assert_eq!(canonicalize("a/../../"), None);
/// assert_eq!(canonicalize(""), Some(String::new()));
/// ```
pub fn canonicalize<P: AsRef<str>>(path: P) -> Option<String> {
    let mut route = Vec::new();
    for name in ancestors_of(path.as_ref()) {
        match name {
            "." => {}
            ".." => {
                let _ = route.pop()?;
            }
            name => route.push(name),
        }
    }
    Some(route.join("/"))
}

/// 返回路径的各个祖先段的迭代器
///
/// 此函数将路径按分隔符 (`/` 或 `\`) 拆分, 并过滤掉所有空段 (例如由连续分隔符造成的空字符串).
/// 返回的迭代器按原始顺序依次产生每个非空的路径段.
///
/// # Examples
/// ```
/// # use path_tree::ancestors_of;
///
/// let segments: Vec<_> = ancestors_of("a/b/c").collect();
/// assert_eq!(segments, vec!["a", "b", "c"]);
///
/// let segments: Vec<_> = ancestors_of("a//b\\c/").collect();
/// assert_eq!(segments, vec!["a", "b", "c"]);
///
/// let segments: Vec<_> = ancestors_of("").collect();
/// assert_eq!(segments.len(), 0);
///
/// let segments: Vec<_> = ancestors_of("/").collect();
/// assert_eq!(segments.len(), 0);
/// ```
pub fn ancestors_of(path: &str) -> impl Iterator<Item = &str> {
    path.split(PATH_SEPARATORS).filter(|name| !name.is_empty())
}

/// 获取路径的父目录
///
/// # Behavior
/// * 对于空路径或不含分隔符的单级路径, 返回空字符串表示根.
/// * 可处理不规范路径.
/// * 返回路径末尾不含分隔符.
///
/// # Examples
/// ```
/// # use path_tree::parent_of;
///
/// assert_eq!(parent_of("a/b/c"), "a/b");
/// assert_eq!(parent_of("a/b/c/"), "a/b");
/// assert_eq!(parent_of("a\\b\\c"), "a\\b");
/// assert_eq!(parent_of("a"), "");
/// assert_eq!(parent_of(""), "");
/// assert_eq!(parent_of("a/"), "");
/// ```
pub fn parent_of(path: &str) -> &str {
    match path
        .trim_end_matches(PATH_SEPARATORS)
        .rsplit_once(PATH_SEPARATORS)
    {
        Some((parent, _)) => parent.trim_end_matches(PATH_SEPARATORS),
        None => "",
    }
}

/// 获取路径的最后一项
///
/// # Behavior
/// * 对于空路径或不含分隔符的单级路径, 返回空字符串表示根.
/// * 可处理不规范路径.
///
/// # Examples
/// ```
/// # use path_tree::name_of;
///
/// assert_eq!(name_of("a/b/c"), "c");
/// assert_eq!(name_of("a/b/c/"), "c");
/// assert_eq!(name_of("a\\b\\c"), "c");
/// assert_eq!(name_of("a"), "a");
/// assert_eq!(name_of(""), "");
/// assert_eq!(name_of("a/"), "a");
/// ```
pub fn name_of(path: &str) -> &str {
    let path = path.trim_end_matches(PATH_SEPARATORS);
    match path.rsplit_once(PATH_SEPARATORS) {
        Some((_, name)) => name,
        None => path.trim_start_matches(PATH_SEPARATORS),
    }
}

/// 切分一次路径
///
/// # Behavior
/// * 此函数假设路径规范 ([`crate::canonicalize`]), 若否将出现 `.`, `\` 等非法节点名.
///
/// # Examples
/// ```
/// # use path_tree::split_path_once;
///
/// let (a, b) = split_path_once("a/b/c");
/// assert_eq!(a, "a");
/// assert_eq!(b, Some("b/c"));
///
/// let (a, b) = split_path_once("file.txt");
/// assert_eq!(a, "file.txt");
/// assert_eq!(b, None);
///
/// let (a, b) = split_path_once("");
/// assert_eq!(a, "");
/// assert_eq!(b, None);
/// ```
pub fn split_path_once(path: &str) -> (&str, Option<&str>) {
    match path.split_once(PATH_SEPARATORS) {
        Some((name, path)) => (name, Some(path)),
        None => (path, None),
    }
}
