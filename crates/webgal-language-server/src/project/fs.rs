//! 文件访问 (提供基于自定义语言服务扩展协议的默认实现)

use anyhow::Result;
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};
use tower_lsp::{Client, lsp_types::request::Request};

/// 支持文件访问的类型
#[async_trait::async_trait]
pub trait FileSystem {
    /// 查询文件或目录是否存在
    async fn exists(&self, path: &str) -> Result<ExistsResult>;

    /// 读取目录
    ///
    /// # Behavior
    /// * 只读取单层目录, 不递归遍历.
    /// * 路径不合法, 目录不存在等情况返回错误, 而非空列表.
    async fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>>;

    /// 读取文件
    async fn read_to_string(&self, path: &str) -> Result<String>;
}

/// 文件或目录存在性查询结果
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ExistsResult {
    pub exists: bool,
    pub is_directory: bool,
}

impl ExistsResult {
    pub fn is_file(&self) -> bool {
        !self.is_directory
    }

    pub fn exists_file(&self) -> bool {
        self.exists && self.is_file()
    }

    pub fn exists_directory(&self) -> bool {
        self.exists && self.is_directory
    }
}

/// 路径条目, 包含节点名称及类型
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub is_directory: bool,
}

impl DirEntry {
    pub fn is_file(&self) -> bool {
        !self.is_directory
    }

    pub fn as_file(&self) -> Option<&str> {
        self.is_file().then_some(&self.name)
    }

    pub fn into_file(self) -> Option<String> {
        self.is_file().then_some(self.name)
    }

    pub fn as_directory(&self) -> Option<&str> {
        self.is_directory.then_some(&self.name)
    }

    pub fn into_directory(self) -> Option<String> {
        self.is_directory.then_some(self.name)
    }
}

// -------- client --------

#[async_trait::async_trait]
impl FileSystem for Client {
    /// 查询文件或目录是否存在
    ///
    /// # Requests
    /// 该方法通过自定义请求 `workspace/fs/stat` 与客户端通信.
    /// * 请求参数
    ///   ```json
    ///   { "uri": "file:///..." }
    ///   ```
    /// * 成功响应
    ///   ```json
    ///   { "type": "file", "size": 123, "mtime": 0, "ctime": 0 }
    ///   或 `null` 表示资源不存在.
    ///   ```
    async fn exists(&self, path: &str) -> Result<ExistsResult> {
        let params = StatParams {
            uri: path.to_string(),
        };
        let result = self.send_request::<StatRequest>(params).await?;
        Ok(result
            .map(|stat| ExistsResult {
                exists: true,
                is_directory: stat.file_type == FileType::Directory,
            })
            .unwrap_or(ExistsResult {
                exists: false,
                is_directory: false,
            }))
    }

    /// 读取目录
    ///
    /// # Requests
    /// 该方法通过自定义请求 `workspace/fs/readDirectory` 与客户端通信.
    /// * 请求参数
    ///   ```json
    ///   { "uri": "file:///..." }
    ///   ```
    /// * 成功响应
    ///   ```json
    ///   [{ "uri": "file:///...", "name": "start.txt", "type": "file" }]
    ///   ```
    async fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let params = ReadDirectoryParams {
            uri: path.to_string(),
        };
        let entries = self.send_request::<ReadDirectoryRequest>(params).await?;
        Ok(entries
            .into_iter()
            .map(|entry| DirEntry {
                name: entry.name,
                is_directory: entry.file_type == FileType::Directory,
            })
            .collect())
    }

    /// 读取文件
    ///
    /// # Requests
    /// 该方法通过自定义请求 `workspace/fs/readFile` 与客户端通信.
    /// * 请求参数
    ///   ```json
    ///   { "uri": "file:///...", "encoding": "utf-8" }
    ///   ```
    /// * 成功响应
    ///   ```json
    ///   { "content": "文件内容字符串", "encoding": "utf-8" }
    ///   ```
    async fn read_to_string(&self, path: &str) -> Result<String> {
        let params = ReadFileParams {
            uri: path.to_string(),
            encoding: FileEncoding::Utf8,
        };
        let result = self.send_request::<ReadFileRequest>(params).await?;
        Ok(result.content)
    }
}

struct StatRequest;

impl Request for StatRequest {
    type Params = StatParams;
    type Result = Option<FileStat>;
    const METHOD: &'static str = "workspace/fs/stat";
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatParams {
    uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FileType {
    File,
    Directory,
    SymbolicLink,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileStat {
    #[serde(rename = "type")]
    file_type: FileType,
    size: u64,
    mtime: Option<u64>,
    ctime: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum FileEncoding {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "base64")]
    Base64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProtocolDirEntry {
    uri: String,
    name: String,
    #[serde(rename = "type")]
    file_type: FileType,
    size: Option<u64>,
    mtime: Option<u64>,
}

struct ReadDirectoryRequest;

impl Request for ReadDirectoryRequest {
    type Params = ReadDirectoryParams;
    type Result = Vec<ProtocolDirEntry>;
    const METHOD: &'static str = "workspace/fs/readDirectory";
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadDirectoryParams {
    uri: String,
}

struct ReadFileRequest;

impl Request for ReadFileRequest {
    type Params = ReadFileParams;
    type Result = ReadFileResult;
    const METHOD: &'static str = "workspace/fs/readFile";
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadFileParams {
    uri: String,
    encoding: FileEncoding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadFileResult {
    content: String,
    encoding: FileEncoding,
    size: Option<u64>,
    mtime: Option<u64>,
    etag: Option<String>,
}
