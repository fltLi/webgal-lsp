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
    /// 该方法通过自定义请求 `workspace/fs/exists` 与客户端通信.
    /// * 请求参数
    ///   ```json
    ///   { "path": "文件或目录路径" }
    ///   ```
    /// * 成功响应
    ///   ```json
    ///   { "exists": true / false, "isDirectory": true / false }
    ///   ```
    async fn exists(&self, path: &str) -> Result<ExistsResult> {
        let params = ExistsParams {
            path: path.to_string(),
        };
        let result = self.send_request::<ExistsRequest>(params).await?;
        Ok(result)
    }

    /// 读取目录
    ///
    /// # Requests
    /// 该方法通过自定义请求 `workspace/fs/readDirectory` 与客户端通信.
    /// * 请求参数
    ///   ```json
    ///   { "path": "目录路径" }
    ///   ```
    /// * 成功响应
    ///   ```json
    ///   [{ "": "子节点名称", "isDirectory": true / false }]
    ///   ```
    async fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let params = ReadDirectoryParams {
            path: path.to_string(),
        };
        let entries = self.send_request::<ReadDirectoryRequest>(params).await?;
        Ok(entries)
    }

    /// 读取文件
    ///
    /// # Requests
    /// 该方法通过自定义请求 `workspace/fs/readFile` 与客户端通信.
    /// * 请求参数
    ///   ```json
    ///   { "path": "文件路径" }
    ///   ```
    /// * 成功响应
    ///   ```json
    ///   "文件内容字符串"
    ///   ```
    async fn read_to_string(&self, path: &str) -> Result<String> {
        let params = ReadFileParams {
            path: path.to_string(),
        };
        let content = self.send_request::<ReadFileRequest>(params).await?;
        Ok(content)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExistsParams {
    path: String,
}

struct ExistsRequest;

impl Request for ExistsRequest {
    type Params = ExistsParams;
    type Result = ExistsResult;
    const METHOD: &'static str = "workspace/fs/exists";
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadDirectoryParams {
    path: String,
}

struct ReadDirectoryRequest;

impl Request for ReadDirectoryRequest {
    type Params = ReadDirectoryParams;
    type Result = Vec<DirEntry>;
    const METHOD: &'static str = "workspace/fs/readDirectory";
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadFileParams {
    path: String,
}

struct ReadFileRequest;

impl Request for ReadFileRequest {
    type Params = ReadFileParams;
    type Result = String;
    const METHOD: &'static str = "workspace/fs/readFile";
}
