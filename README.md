# WebGAL LSP

WebGAL 语言基础设施。

> :construction: 项目仍在开发中，欢迎试用和反馈。

## :sparkles: 功能

- **语言解析**：提供 WebGAL 语法解析与数据结构支持
- **自动补全**：语句、参数、资源路径、标识符等智能提示 -> [详细说明](./docs/complete.md)
- **代码诊断**：语法错误、资源缺失等实时检查 -> [详细说明](./docs/diagnose.md)
- **语义高亮**：语句类型、参数、注释等色彩渲染

## :rocket: 快速开始

> [!IMPORTANT]
> 此服务器在标准 LSP 的基础上进行了扩展（详见文档注释），第三方客户端需要提供支持。

### 1. 编译语言服务器

```bash
cargo build -p webgal-ls
```

### 2. 构建并运行 VS Code 扩展

```bash
cd vscode-extension
npm install && npm run compile
```

按 `F5` 启动调试窗口。

## :page_facing_up: 许可证

Code: MPL-2.0, 2026, fltLi
