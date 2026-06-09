use tokio::io;
use tower_lsp::LspService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .init();
    // 启动服务器
    let (service, socket) = LspService::new(webgal_ls::server::Backend::new);
    tower_lsp::Server::new(io::stdin(), io::stdout(), socket)
        .serve(service)
        .await;
    Ok(())
}
