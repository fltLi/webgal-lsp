//! 文件访问 (提供基于自定义语言服务扩展协议的默认实现)

use anyhow::Result;
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};
use tower_lsp::{Client, lsp_types::request::Request};

/// 支持文件访问的类型
#[async_trait::async_trait]
pub trait FileSystem {
    /// 列出目录子节点 (非递归)
    ///
    /// # Behavior
    /// * 路径不合法, 目录不存在等情况返回错误, 而非空列表.
    async fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>>;

    /// 读取文件
    async fn read_to_string(&self, path: &str) -> Result<String>;
}

/// 路径条目, 包含节点名称及类型
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub is_file: bool,
}

impl DirEntry {
    pub fn as_file(&self) -> Option<&str> {
        self.is_file.then_some(&self.name)
    }

    pub fn into_file(self) -> Option<String> {
        self.is_file.then_some(self.name)
    }

    pub fn is_folder(&self) -> bool {
        !self.is_file
    }

    pub fn as_folder(&self) -> Option<&str> {
        (!self.is_file).then_some(&self.name)
    }

    pub fn into_folder(self) -> Option<String> {
        (!self.is_file).then_some(self.name)
    }
}

// -------- client --------

#[async_trait::async_trait]
impl FileSystem for Client {
    /// 列出目录子节点 (非递归)
    ///
    /// # Requests
    /// 该方法通过自定义请求 `workspace/fs/readDir` 与客户端通信.
    /// * 请求参数
    ///   ```json
    ///   { "path": "目录相对于工作区根目录的路径" }
    ///   ```
    /// * 成功响应
    ///   ```json
    ///   [{ "": "文件名 (即文件相对于传入目录的路径)", "isFile": true / false}]
    ///   ```
    async fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let params = ReadDirParams {
            path: path.to_string(),
        };
        let entries = self.send_request::<ReadDirRequest>(params).await?;
        Ok(entries)
    }

    /// 读取文件
    ///
    /// # Requests
    /// 该方法通过自定义请求 `workspace/fs/readToString` 与客户端通信.
    /// * 请求参数
    ///   ```json
    ///   { "path": "文件相对于工作区根目录的路径" }
    ///   ```
    /// * 成功响应
    ///   ```json
    ///   "文件内容字符串"
    ///   ```
    async fn read_to_string(&self, path: &str) -> Result<String> {
        let params = ReadToStringParams {
            path: path.to_string(),
        };
        let content = self.send_request::<ReadToStringRequest>(params).await?;
        Ok(content)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadDirParams {
    path: String,
}

enum ReadDirRequest {}

impl Request for ReadDirRequest {
    type Params = ReadDirParams;
    type Result = Vec<DirEntry>;
    const METHOD: &'static str = "workspace/fs/readDir";
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadToStringParams {
    path: String,
}

enum ReadToStringRequest {}

impl Request for ReadToStringRequest {
    type Params = ReadToStringParams;
    type Result = String;
    const METHOD: &'static str = "workspace/fs/readToString";
}
