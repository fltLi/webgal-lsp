use std::{
    collections::{HashMap, HashSet},
    mem,
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::anyhow;
use path_tree::{Node, join, name_of, parent_of};
use rayon::prelude::*;
use strum::{Display, EnumString};
use tokio::{
    runtime::Handle,
    select,
    sync::{
        Mutex as AsyncMutex, RwLock as AsyncRwLock,
        mpsc::{UnboundedSender, unbounded_channel},
    },
    task::{JoinSet, spawn, spawn_blocking},
    time::interval,
};
use tower_lsp::{Client, LanguageServer, jsonrpc, lsp_types::*};
use tracing::{debug, error, info, warn};
use webgal_language_core::resource::ResourceKind;

use crate::{
    encode::*,
    project::{DirEntry, FileSystem, GetProjectResult, Project, Workspace},
    service::*,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// WebGAL 语言服务器后端
///
/// 支持工作区多项目语言服务提供.
/// 包括: 代码诊断, 语义高亮, 自动补全, 脚本格式化等功能.
///
/// # Requests
/// 此服务器依赖以下自定义协议:
///
/// * `workspace/fs/readDirectory` - 读取目录.
///   * 请求参数
///     ```json
///     { "path": "目录路径" }
///     ```
///   * 成功响应
///     ```json
///     [{ "": "子节点名称", "isDirectory": true / false }]
///     ```
///
/// * `workspace/fs/readFile` - 读取文件.
///   * 请求参数
///     ```json
///     { "path": "文件路径" }
///     ```
///   * 成功响应
///     ```json
///     "文件内容字符串"
///     ```
///
/// * `workspace/fs/exists` - 查询文件或目录是否存在.
///   * 请求参数
///     ```json
///     { "path": "文件或目录路径" }
///     ```
///   * 成功响应
///     ```json
///     { "exists": true / false, "isDirectory": true / false }
///     ```
#[derive(Debug, Clone)]
pub struct Backend {
    client: Client,
    workspace: Arc<AsyncRwLock<Workspace>>,
    open_ducuments: Arc<AsyncRwLock<HashSet<String>>>, // 编辑器活动文档
    diagnose: UnboundedSender<(String, Arc<RwLock<Project>>)>, // 诊断服务通道
    /// 初始化参数
    init_state: Arc<AsyncMutex<InitializationState>>,
}

#[derive(Debug, Default)]
struct InitializationState {
    workspace_folders: Option<Vec<WorkspaceFolder>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let workspace = Arc::new(AsyncRwLock::new(Workspace::new()));
        let diagnose = start_diagnostic_service(client.clone());

        Self {
            client,
            workspace,
            open_ducuments: Default::default(),
            diagnose,
            init_state: Default::default(),
        }
    }

    async fn initialize_workspace(&self, root: &str) -> anyhow::Result<()> {
        debug!(%root, "Scanning workspace root");
        let mut errors = Vec::new();

        let mut stack = vec![root.to_string()];
        while let Some(root) = stack.pop() {
            let children = match self.client.read_dir(&root).await {
                Ok(v) => v,
                Err(error) => {
                    errors.push(anyhow!("遍历目录 `{root}` 出错: {error}"));
                    continue;
                }
            };

            for DirEntry { name, is_directory } in children {
                let path = join(&root, name);
                if is_directory {
                    stack.push(path);
                    continue;
                }

                // 执行更改
                if let Err(error) = self.change_file(&path, FileChangeKind::Create).await {
                    errors.push(error);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!("{errors:?}"))
        }
    }

    async fn initialize_workspace_folders(&self, folders: Vec<WorkspaceFolder>) {
        for WorkspaceFolder { uri, name } in folders {
            let root = uri.to_string();
            info!(%name, %root, "Initializing workspace folder");
            if let Err(error) = self.initialize_workspace(&root).await {
                error!(%name, %root, %error, "Failed to initialize workspace folder");
            }
        }
    }

    async fn change_file(&self, path: &str, kind: FileChangeKind) -> anyhow::Result<()> {
        debug!(%path, %kind, "Changing file");
        let is_config = name_of(path) == "config.txt";

        match kind {
            // 创建项目
            FileChangeKind::Create if is_config => {
                let project_path = parent_of(path);
                info!(project = %project_path, "Loading project");

                let mut errors = Vec::new();
                let project = Project::load(project_path, &self.client, &mut errors).await;

                match self.workspace.write().await.insert(project_path, project) {
                    Ok(project) => self.diagnose_project(project_path, project),
                    Err(error) => errors.push(error),
                };

                if !errors.is_empty() {
                    return Err(anyhow!("创建项目出错: {errors:?}"));
                }
            }

            // 移除项目
            FileChangeKind::Remove if is_config => {
                let project_path = parent_of(path);
                debug!(project = %project_path, "Removing project");
                let project = self
                    .workspace
                    .write()
                    .await
                    .remove(project_path)
                    .ok_or_else(|| anyhow!("找不到配置所在项目"))?;

                // 清理诊断
                clean_project_diagnostics(project_path, project, &self.client).await;
            }

            // 创建 / 修改资源
            FileChangeKind::Create | FileChangeKind::Change => {
                let GetProjectResult {
                    project_path,
                    resource_path,
                    project,
                } = match self.workspace.read().await.get(path) {
                    Some(v) => v,
                    None => {
                        debug!(%path, "File event ignored: not in any project");
                        return Ok(());
                    }
                };
                debug!(project = %project_path, path = %resource_path, "Updating resource");

                let project = spawn_blocking({
                    let fs: &'static Client = unsafe { mem::transmute(&self.client) };
                    let path: &'static str = unsafe { mem::transmute(path) };
                    move || {
                        let result = project.write().unwrap().insert(&resource_path, || {
                            Handle::current().block_on(fs.read_to_string(path))
                        });
                        result.map(|_| project)
                    }
                })
                .await
                .unwrap()?;

                self.diagnose_project(&project_path, project);
            }

            // 移除资源
            FileChangeKind::Remove => {
                let GetProjectResult {
                    project_path,
                    resource_path,
                    project,
                } = match self.workspace.read().await.get(path) {
                    Some(v) => v,
                    None => {
                        debug!(%path, "File event ignored: not in any project");
                        return Ok(());
                    }
                };
                debug!(project = %project_path, path = %resource_path, "Removing resource");

                let project = spawn_blocking(move || {
                    let result = project.write().unwrap().remove(&resource_path);
                    result.map(|_| project)
                })
                .await
                .unwrap()?;

                self.diagnose_project(&project_path, project);
            }
        }

        Ok(())
    }

    fn diagnose_project(&self, path: &str, project: Arc<RwLock<Project>>) {
        if let Err(error) = self.diagnose.send((path.to_string(), project)) {
            warn!(project = %path, %error, "Failed to send diagnostic request");
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        // 记录初始工作区, 等待扫描
        *self.init_state.lock().await = InitializationState {
            workspace_folders: params.workspace_folders,
        };

        // 注册服务器能力和信息
        let capabilities = ServerCapabilities {
            position_encoding: Some(PositionEncodingKind::UTF16),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::FULL),
                    ..Default::default()
                },
            )),
            semantic_tokens_provider: Some(highlight_capability()),
            completion_provider: Some(complete_capability()),
            document_formatting_provider: Some(format_capability()),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        Ok(InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: "webgal-language-server".to_string(),
                version: Some(VERSION.to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        // 扫描并初始化工作区
        {
            let mut state = self.init_state.lock().await;
            if let Some(folders) = state.workspace_folders.take() {
                let server = self.clone();
                spawn(async move { server.initialize_workspace_folders(folders).await });
            }
        }

        // 注册监控所有文件
        let registration = Registration {
            id: "webgal-all".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(
                serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                    watchers: vec![FileSystemWatcher {
                        glob_pattern: GlobPattern::String("**/*".to_string()),
                        kind: Some(WatchKind::all()),
                    }],
                })
                .unwrap(),
            ),
        };
        if let Err(error) = self.client.register_capability(vec![registration]).await {
            error!(%error, "Failed to register file watchers");
        } else {
            info!("File watcher registered for all files");
        }
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let path = params.text_document.uri.to_string();
        let content = params.text_document.text;

        // 注册活动文档
        self.open_ducuments.write().await.insert(path.clone());

        // 查找项目
        let GetProjectResult {
            project_path,
            resource_path,
            project,
        } = match self.workspace.read().await.get(&path) {
            Some(v) => v,
            None => {
                debug!(%path, "Document opened but not in any project");
                return;
            }
        };

        // 更新资源
        let project = match spawn_blocking({
            let path: &'static str = unsafe { mem::transmute(resource_path.as_str()) };
            move || {
                let result = project.write().unwrap().insert(path, || Ok(content));
                result.map(|_| project)
            }
        })
        .await
        .unwrap()
        {
            Ok(v) => v,
            Err(error) => {
                error!(project = %project_path, path = %resource_path, %error, "Failed to update resource on change");
                return;
            }
        };

        debug!(project = %project_path, path = %resource_path, "Updated resource via open");
        self.diagnose_project(&project_path, project);
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        let path = params.text_document.uri.to_string();
        let content = match params.content_changes.pop() {
            Some(TextDocumentContentChangeEvent { text, .. }) => text,
            None => return,
        };

        // 查找项目
        let GetProjectResult {
            project_path,
            resource_path,
            project,
        } = match self.workspace.read().await.get(&path) {
            Some(v) => v,
            None => {
                debug!(%path, "Document changed but not in any project");
                return;
            }
        };

        // 更新资源
        let project = match spawn_blocking({
            let path: &'static str = unsafe { mem::transmute(resource_path.as_str()) };
            move || {
                let result = project.write().unwrap().insert(path, || Ok(content));
                result.map(|_| project)
            }
        })
        .await
        .unwrap()
        {
            Ok(v) => v,
            Err(error) => {
                error!(project = %project_path, path = %resource_path, %error, "Failed to update resource on change");
                return;
            }
        };

        debug!(project = %project_path, path = %resource_path, "Updated resource via change");
        self.diagnose_project(&project_path, project);
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let path = params.text_document.uri.to_string();

        // 签出活动文档
        self.open_ducuments.write().await.remove(&path);

        // 查找项目
        let GetProjectResult {
            project_path,
            resource_path,
            project,
        } = match self.workspace.read().await.get(&path) {
            Some(v) => v,
            None => {
                debug!(%path, "Document closed but not in any project");
                return;
            }
        };

        // 如果文件不存在于磁盘, 就移除映射
        if let Ok(result) = self.client.exists(&path).await
            && !result.exists_file()
        {
            if let Err(error) = spawn_blocking({
                let resource_path: &'static str = unsafe { mem::transmute(resource_path.as_str()) };
                move || project.write().unwrap().remove(resource_path)
            })
            .await
            .unwrap()
            {
                error!(project = %project_path, path = %resource_path, %error, "Failed to remove resource on change");
            } else {
                debug!(project = %project_path, path = %resource_path, "Removed resource via close");
            }
            return;
        }

        // 重置资源为磁盘内容
        let project = match spawn_blocking({
            let fs: &'static Client = unsafe { mem::transmute(&self.client) };
            let resource_path: &'static str = unsafe { mem::transmute(resource_path.as_str()) };
            move || {
                let result = project.write().unwrap().insert(resource_path, || {
                    Handle::current().block_on(fs.read_to_string(&path))
                });
                result.map(|_| project)
            }
        })
        .await
        .unwrap()
        {
            Ok(v) => v,
            Err(error) => {
                error!(project = %project_path, path = %resource_path, %error, "Failed to update resource on change");
                return;
            }
        };

        debug!(project = %project_path, path = %resource_path, "Updated resource via close");
        self.diagnose_project(&project_path, project);
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for FileEvent { uri, typ } in params.changes {
            let path = uri.to_string();
            let kind = match typ.try_into() {
                Ok(v) => v,
                Err(_) => {
                    warn!(%path, "Unknown file event `{typ:?}`");
                    continue;
                }
            };

            // 过滤活动文档
            if self.open_ducuments.read().await.contains(&path) {
                continue;
            }

            if let Err(error) = self.change_file(&path, kind).await {
                error!(%path, %kind, %error, "Failed to update file");
            }
        }
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        let WorkspaceFoldersChangeEvent { added, removed } = params.event;
        let server = self.clone();
        spawn(async move {
            if !added.is_empty() {
                info!(count = added.len(), "Workspace folders added");
                server.initialize_workspace_folders(added).await;
            }
            for WorkspaceFolder { uri, name } in removed {
                let root = uri.to_string();
                info!(%name, %root, "Workspace folder removed");
                server.workspace.write().await.remove_all_under(&root);
            }
        });
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let path = params.text_document.uri.to_string();

        // 查找项目
        let GetProjectResult {
            project_path,
            resource_path,
            project,
        } = match self.workspace.read().await.get(&path) {
            Some(v) => v,
            None => {
                debug!(%path, "Highlighting requested but not in any project");
                return Ok(None);
            }
        };

        let tokens = spawn_blocking(move || {
            // 校验路径
            let (kind, path) = ResourceKind::from_path(&resource_path);
            if kind != ResourceKind::Scene {
                debug!(project = %project_path, path = %resource_path, "Highlighting skipped: not a scene file");
                return None;
            }

            // 查找场景
            let project = project.read().unwrap();
            let scene = match project.resource().scene.get(path) {
                Some(Node::Item(v)) => v,
                _ => {
                    debug!(project = %project_path, %path, "Scene not found for highlighting");
                    return None;
                }
            };

            // 生成补全
            info!(project = %project_path, %path, "Highlight scene");
            let mut tokens = highlight(scene);
            highlights_utf8_to_utf16(scene, &mut tokens);
            Some(tokens)
        })
        .await
        .unwrap();

        Ok(tokens.map(|tokens| {
            SemanticTokensResult::Tokens(SemanticTokens {
                data: tokens,
                ..Default::default()
            })
        }))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let path = params.text_document_position.text_document.uri.to_string();

        // 查找项目
        let GetProjectResult {
            project_path,
            resource_path,
            project,
        } = match self.workspace.read().await.get(&path) {
            Some(v) => v,
            None => {
                debug!(%path, "Completion requested but not in any project");
                return Ok(None);
            }
        };

        let completions = spawn_blocking(move || {
            // 校验路径
            let (kind, path) = ResourceKind::from_path(&resource_path);
            if kind != ResourceKind::Scene {
                debug!(project = %project_path, path = %resource_path, "Completing skipped: not a scene file");
                return None;
            }

            // 查找场景
            let project = project.read().unwrap();
            let scene = match project.resource().scene.get(path) {
                Some(Node::Item(v)) => v,
                _ => {
                    debug!(project = %project_path, %path, "Scene not found for completion");
                    return None;
                }
            };

            // 生成补全
            info!(project = %project_path, %path, "Completing input");
            let position = position_utf16_to_utf8(scene, params.text_document_position.position);
            let mut completions = complete(scene, position, &project);
            completions_utf8_to_utf16(scene, &mut completions);
            Some(completions)
        })
        .await
        .unwrap();

        Ok(completions.map(CompletionResponse::Array))
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let path = params.text_document.uri.to_string();

        // 查找项目
        let GetProjectResult {
            project_path,
            resource_path,
            project,
        } = match self.workspace.read().await.get(&path) {
            Some(v) => v,
            None => {
                debug!(%path, "Formatting requested but not in any project");
                return Ok(None);
            }
        };

        let edits = spawn_blocking(move || {
            // 校验路径
            let (kind, path) = ResourceKind::from_path(&resource_path);
            if kind != ResourceKind::Scene {
                debug!(project = %project_path, path = %resource_path, "Formatting skipped: not a scene file");
                return None;
            }

            // 查找场景
            let project = project.read().unwrap();
            let scene = match project.resource().scene.get(path) {
                Some(Node::Item(v)) => v,
                _ => {
                    debug!(project = %project_path, %path, "Scene not found for formatting");
                    return None;
                }
            };

            // 生成补全
            info!(project = %project_path, %path, "Formatting scene");
            let mut edits = format(scene);
            formatting_utf8_to_utf16(scene, &mut edits);
            Some(edits)
        })
        .await
        .unwrap();

        Ok(edits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, EnumString)]
pub enum FileChangeKind {
    Create,
    Change,
    Remove,
}

impl TryFrom<FileChangeType> for FileChangeKind {
    type Error = ();

    fn try_from(value: FileChangeType) -> Result<Self, Self::Error> {
        match value {
            FileChangeType::CREATED => Ok(Self::Create),
            FileChangeType::CHANGED => Ok(Self::Change),
            FileChangeType::DELETED => Ok(Self::Remove),
            _ => Err(()),
        }
    }
}

/// 启动诊断推送服务
///
/// 通过管道发送推送任务 (项目完整路径 + 项目).
/// 任务自动去重, 每隔 500ms 集中处理一次, 避免大量更新阻塞程序.
fn start_diagnostic_service(client: Client) -> UnboundedSender<(String, Arc<RwLock<Project>>)> {
    let (sender, mut receiver) = unbounded_channel();

    tokio::spawn(async move {
        let client: &'static Client = unsafe { mem::transmute(&client) };
        let mut pending: HashMap<String, _> = HashMap::new();
        let mut interval = interval(Duration::from_millis(500));

        loop {
            select! {
                Some((path, project)) = receiver.recv() => {
                    debug!(project = %path, "Diagnostic request enqueued");
                    pending.insert(path, project);
                }

                _ = interval.tick() => {
                    if pending.is_empty() {
                        continue;
                    }

                    debug!(count = pending.len(), "Processing diagnostic batch");
                    let tasks: JoinSet<_> = pending
                        .drain()
                        .map(|(path, project)| async move {
                            publish_project_diagnostics(&path, project, client).await
                        })
                        .collect();
                    tasks.join_all().await;
                }
            }
        }
    });

    sender
}

async fn clean_project_diagnostics(path: &str, project: Arc<RwLock<Project>>, client: &Client) {
    debug!(project = %path, "Cleaning diagnostics for project");
    let scene_folder_path = join(path, "scene");
    let uris: Vec<_> = spawn_blocking(move || {
        project
            .read()
            .unwrap()
            .resource()
            .scene
            .iter_recursively()
            .map(|(path, _)| join(&scene_folder_path, path).parse().unwrap())
            .collect()
    })
    .await
    .unwrap();
    for uri in uris {
        client.publish_diagnostics(uri, Vec::default(), None).await;
    }
}

/// 生成并推送一个项目的诊断
async fn publish_project_diagnostics(path: &str, project: Arc<RwLock<Project>>, client: &Client) {
    let path = path.to_string();

    info!(project = %path, "Diagnosing project");
    let scene_folder_path = join(&path, "scene");

    // 生成诊断
    let diagnostics: Vec<_> = spawn_blocking(move || {
        diagnose_project(&project.read().unwrap())
            .into_par_iter()
            .map(|(path, scene, mut diagnostics)| {
                let uri = join(&scene_folder_path, path).parse().unwrap();
                diagnostics_utf8_to_utf16(scene, &mut diagnostics);
                (uri, diagnostics)
            })
            .collect()
    })
    .await
    .unwrap();

    // 推送诊断
    for (uri, diagnostics) in diagnostics {
        client.publish_diagnostics(uri, diagnostics, None).await;
    }
}
