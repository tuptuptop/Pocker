# PockerStudio

> Pocker Studio — Plugin as a Service Web GUI
>
> 基于 [Hermes-Studio](https://github.com/JPeetz/Hermes-Studio) 改造，适配 Pocker 引擎。

## 技术栈

- **前端**: React 19 + TanStack Router/Start + TailwindCSS 4 + Zustand
- **后端**: Node.js (TanStack Start SSR) + Rust (Pocker Engine)
- **桌面**: Electron (打包为 exe / dmg)
- **终端**: xterm.js
- **编辑器**: Monaco Editor
- **图表**: Recharts

## 功能

- 💬 **Chat** — 与 LLM 对话，支持流式输出、工具调用
- 🔌 **Plugins** — 管理已安装的 Pocker 插件
- ⚡ **Skills** — 查看和运行 Pocker Skills
- 📁 **Files** — 文件浏览器
- 🖥️ **Terminal** — 内嵌终端 (xterm.js)
- 📊 **Dashboard** — 系统概览
- 🧠 **Memory** — 记忆管理
- ⚙️ **Settings** — 配置 LLM、Profile 等

## 开发

```bash
# 安装依赖
cd PockerStudio
pnpm install

# 开发模式 (需要 Pocker 引擎在 127.0.0.1:3080 运行)
pnpm dev

# 构建
pnpm build

# 启动生产服务器
pnpm start
```

## 桌面应用打包

```bash
# Windows (exe)
pnpm electron:build:win

# macOS (dmg)
pnpm electron:build:mac

# Linux (AppImage)
pnpm electron:build:linux
```

## 环境变量

参见 `.env.example`:

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `POCKER_API_URL` | `http://127.0.0.1:3080` | Pocker 引擎 API 地址 |
| `PORT` | `3000` | Web 服务器端口 |
| `OPENAI_API_KEY` | - | OpenAI API 密钥 |
| `OLLAMA_HOST` | `http://127.0.0.1:11434` | Ollama 地址 |

## 许可

MIT
