use std::io::stderr;

use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;
use webgal_ls::server::Backend;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(stderr)
        .with_ansi(true)
        .init();

    // 启动服务器
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin(), stdout(), socket).serve(service).await;
    Ok(())
}
