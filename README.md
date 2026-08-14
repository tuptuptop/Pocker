# Pocker

> **Plugin as a Service** — Everything is a Plugin.
>
> 融合 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) 的 "Everything is a Plugin" 架构、
> [Docker](https://docs.docker.com/) 的插件分发理念、
> [Hermes-Studio](https://github.com/JPeetz/Hermes-Studio) 的 UI 技术栈。

## 技术栈

| 组件 | 语言 | 技术选型 |
|------|------|----------|
| **Pocker Engine** | Rust | tokio, serde, tracing |
| **Pocker CLI** | Rust | clap, indicatif |
| **Pocker TUI** | Rust | ratatui, crossterm |
| **Pocker Hub** | Rust | axum, tower-http |
| **Pocker Studio** | Rust + TypeScript | axum (后端) + React/TanStack/TailwindCSS (前端) |
| **Pocker SDK** | Rust | 插件开发工具包 |

## 项目结构

```
Pocker/
├── Cargo.toml                 # Rust workspace 根配置
├── crates/
│   ├── pocker-core/           # 核心类型: Ctx, Seam, Plugin, Event
│   ├── pocker-engine/         # 引擎: PluginLoader, ProfileManager, Engine
│   ├── pocker-plugin/         # 插件 SDK: LlmAdapter, Tool, Skill traits
│   ├── pocker-sandbox/        # 跨平台沙箱: 进程隔离
│   ├── pocker-cli/            # CLI: pocker 命令行工具
│   ├── pocker-tui/            # TUI: 终端界面 (ratatui)
│   ├── pocker-hub/            # Hub: 插件注册和分发平台
│   ├── pocker-studio/         # Studio: Web GUI 后端 (axum)
│   └── pocker-sdk/            # SDK: 插件开发者工具包
├── studio-web/                 # Studio 前端 (TypeScript/React)
├── vendor/                     # 上游参考项目 (gitignore)
│   ├── deepseek-harness/      # DeepSeek Harness 源码
│   └── hermes-studio/         # Hermes-Studio 源码
└── docs/
    └── 规划架构.md             # 架构设计文档
```

## 快速开始

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/pocker/pocker.git
cd pocker

# 编译所有 Rust crate
cargo build

# 运行测试 (50+ 测试)
cargo test

# 运行 CLI
cargo run -p pocker-cli -- --help
cargo run -p pocker-cli -- system info
cargo run -p pocker-cli -- --dump-config

# 运行 Hub 服务器
cargo run -p pocker-hub

# 运行 Studio 后端
cargo run -p pocker-studio
```

### Studio 前端开发

```bash
cd studio-web
npm install
npm run dev    # 开发服务器: http://localhost:3000
npm run build  # 构建到 dist/
```

## 核心理念

**Everything is a Plugin** — 没有特权核心，一切皆可替换：

- **LLM 模型适配器**是 plugin (OpenAI / Anthropic / Ollama / DeepSeek) → 注册到 `ctx.llm`
- **工具注册表**是 plugin (shell / fs / http / git) → 注册到 `ctx.tools`
- **沙箱执行**是 plugin (进程隔离 / 容器 / WASM) → 注册到 `ctx.sandbox`
- **Skills**是 plugin (code-review / doc-gen / data-analysis) → 注册到 `ctx.skills`
- **UI**是 plugin (CLI / TUI / Web GUI / API) → 通过 Profile 切换

### Seam 设计

Seam 是共享上下文中的命名扩展点。插件将自己的服务实现注册到 Seam 上：

```rust
// 注册一个 LLM 适配器
ctx.register_seam(SeamId::llm(), "openai-plugin".into(), llm_impl);

// 获取 LLM 适配器
let llm = ctx.get_seam(&SeamId::llm());
```

标准 Seam：
- `ctx.llm` — LLM 模型适配器
- `ctx.tools` — 工具注册表
- `ctx.skills` — Skills 注册表
- `ctx.sandbox` — 沙箱执行
- `ctx.session` — 会话日志
- `ctx.approval` — 审批系统
- `ctx.fs` — 文件系统
- `ctx.terminal` — 终端
- `ctx.bus` — 事件总线
- `ctx.credentials` — 凭证管理

### Profile 系统

Profile 定义了一组插件的组合方式，类似于 DSH 的 Profile + Bundle 机制：

```yaml
# ~/.pocker/profiles/web/profile.yaml
name: web
description: Web UI profile
bundles:
  - @pocker/core-bundle
  - @pocker/llm-bundle
plugins:
  - @pocker/llm-openai@1.0.0
  - @pocker/tool-shell@1.0.0
```

## 开发

### 创建一个插件

```rust
use pocker_core::plugin::{Plugin, PluginMetadata};
use pocker_core::context::Ctx;
use pocker_core::error::Result;
use std::sync::Arc;

struct MyPlugin {
    meta: PluginMetadata,
}

#[async_trait::async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }

    async fn mount(&self, ctx: &Arc<Ctx>) -> Result<()> {
        // 注册服务到 Seam
        // ctx.register_seam(...)
        Ok(())
    }

    async fn unmount(&self, ctx: &Arc<Ctx>) -> Result<()> {
        // 反注册 (自动展开)
        // ctx.unregister_seam(...)
        Ok(())
    }
}
```

## 测试

```bash
# 运行所有测试
cargo test

# 运行特定 crate 的测试
cargo test -p pocker-core
cargo test -p pocker-engine
cargo test -p pocker-sandbox
cargo test -p pocker-hub
cargo test -p pocker-studio
cargo test -p pocker-sdk
```

当前测试覆盖：
- `pocker-core`: 24 个测试 (错误类型、上下文、事件、Seam、Plugin、类型)
- `pocker-engine`: 12 个测试 (加载器、Profile 管理、引擎)
- `pocker-sandbox`: 4 个测试 (进程执行、超时、退出码)
- `pocker-hub`: 7 个测试 (API 路由、插件存储)
- `pocker-studio`: 2 个测试 (API 路由)
- `pocker-sdk`: 1 个测试 (测试工具)

## 许可

[MIT](LICENSE)
