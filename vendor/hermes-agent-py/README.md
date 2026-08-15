# Hermes Agent (Python) — Pocker Sidecar 集成

把 [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent) 的 Python 版作为
**独立 sidecar 子进程**集成进 Pocker，通过 **ACP (Agent Client Protocol)** 的 stdio JSON-RPC 与主程序通信。

这是 Pocker 的 agent 运行时候选之一（仓库里 `vendor/hermes-studio` 是 JS/TS 版 Web UI，本目录是
Python 版 agent 本体，二者通过此方式组合）。

## 已验证（2026-08-15）

- ✅ 隔离 venv 安装 `hermes-agent[acp]`（`hermes-acp` 0.19.0）
- ✅ `hermes-acp --check` 依赖就绪
- ✅ Node 客户端 spawn `hermes-acp` 走 stdio ACP，`initialize` 握手成功
  - server 返回 `agentInfo: hermes-agent 0.19.0` + `agentCapabilities`（session/prompt 能力）
- ⏳ 模型调用（prompt → reply）**未验证**：当前环境无 LLM provider key

## 目录结构

```
vendor/hermes-agent-py/
├── .venv/            # 隔离 Python venv（gitignore，pip 重建）
├── acp-client.mjs    # 参考 ACP stdio 客户端（Node，零依赖）—— Pocker 对接基准
├── install.log       # pip 安装日志（gitignore）
└── README.md         # 本文件
```

## 重建环境

```powershell
# 用系统 Python 3.12 建 venv
py -3.12 -m venv vendor/hermes-agent-py/.venv
vendor/hermes-agent-py/.venv/Scripts/python.exe -m pip install -U pip
vendor/hermes-agent-py/.venv/Scripts/python.exe -m pip install "hermes-agent[acp]"
```

## 验证握手

```powershell
cd vendor/hermes-agent-py
C:/Users/User/.workbuddy/binaries/node/versions/22.22.2/node.exe acp-client.mjs
# 期望输出: INIT_OK {... "agentInfo":{"name":"hermes-agent","version":"0.19.0"} ...}
```

## 跑通端到端（需模型 key）

`hermes-acp` 启动时读取 `HERMES_HOME/.env`（Hermes Desktop 下默认
`C:/Users/User/AppData/Local/hermes/data/.env`）。填入任一 provider 的 key 即可：

```env
# OpenRouter（官方首选，零配置，200+ 模型，有免费档）
OPENROUTER_API_KEY=sk-or-xxxx
# 或 OpenAI 兼容
OPENAI_API_KEY=sk-xxxx
```

然后：

```powershell
# 设好 HERMES_HOME 指向含 .env 的目录，或直接把 .env 放当前目录
$env:HERMES_HOME="C:/path/to/hermes/data"
PROMPT="用一句话介绍你自己" node acp-client.mjs
```

## ACP 协议要点（已实测）

- 传输：**stdio**，stdout 专供 JSON-RPC，`hermes-acp` 把日志写到 stderr。
- 启动：`hermes-acp`（无端口参数，纯 stdio server）。可选 `--check` / `--setup` / `--setup-browser`。
- 入口内部：`asyncio.run(acp.run_agent(agent, use_unstable_protocol=True))`。
- `initialize` 的 `protocolVersion` 必须是**整数**（实测 `1` 被接受；字符串 `"2025-03-26"` 报 -32602）。
- 初次启动会 lazy-install 缺省的 provider 依赖（如 `boto3`），首次 `initialize` 可能耗时 >30s，客户端需放宽超时。
- 支持的 method（来自 `agentCapabilities`）：`prompt`、`session`（fork/list/resume）、图片输入等。

## 接入 Pocker 主程序（Rust / Electron）的方案

1. **Rust sidecar**：在 `crates/*` 已有的 sidecar 机制里，新增一个 Hermes Python 后端，
   用 `std::process::Command` spawn `hermes-acp.exe`，通过子进程 stdin/stdout 收发 ACP JSON-RPC。
   （复用你 trace-auto cua driver 的 sidecar 模式即可。）
2. **Electron 侧**：可在主进程内用 `child_process.spawn` 起 `hermes-acp`，或复用本目录的
   `acp-client.mjs` 思路封装成 IPC 桥。
3. **配置**：Pocker 的 profile/settings 里加 `hermes` 一节，负责把 provider key 注入
   `HERMES_HOME/.env` 或环境变量，再启动 sidecar。
4. **生命周期**：监听 sidecar 退出码，异常退出自动重启；Pocker 退出时 SIGTERM 回收子进程。

## 注意事项

- **不要**把 `.venv/` 或 `*.env` 提交进仓库（已在根 `.gitignore` 忽略）。
- `hermes-agent` 体积大（822 个 .py / 36 万行），通过 pip 安装而非 vendoring 源码。
- 若想要可复现构建，可在 CI 之外单独用 `uv` 锁版本；当前 Pocker 的 GitHub Actions 只构建
  Rust + PockerStudio（Electron/Web），不含 Python 侧。
