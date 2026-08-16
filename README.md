# Pocker

<p align="center">
  <img src="assets/logo.png" width="160" alt="Pocker" />
</p>

> **The Rust-native, plugin-first AI agent runtime.**
> Everything is a Plugin — LLMs, tools, skills, sandbox, and UI are all swappable plugins mounted on typed capability seams.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![AI Agent](https://img.shields.io/badge/AI%20Agent-Plugin--First-9cf)](https://github.com/pocker/pocker)
[![Cross-platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-2ea)](https://github.com/pocker/pocker)

**Pocker** is a blazing-fast, extensible runtime for building and running AI agents. It is built in Rust (tokio async runtime) and adopts the **"Everything is a Plugin"** architecture: there is no privileged core. Your LLM provider, tool registry, skill library, sandbox executor, and even the CLI/TUI/Web UI are plugins that mount into a shared context through named **seams** (`ctx.llm`, `ctx.tools`, `ctx.skills`, `ctx.sandbox`, …). Swap any layer by configuration — no forking, no recompiling the core.

Pocker is designed for the way developers actually build agents in 2026: **local-first, self-hosted, terminal-native, and skill-driven** — with an open, MCP-friendly extension model so your plugins can talk to the wider agent ecosystem.

---

## Table of Contents

- [Why Pocker](#why-pocker)
- [Features](#features)
- [DeepSeek Harness: A Supercharged AI Coding Experience](#deepseek-harness-a-supercharged-ai-coding-experience)
- [Architecture: Everything is a Plugin](#architecture-everything-is-a-plugin)
- [Why "Pocker": Docker Meets Plugin](#why-pocker-docker-meets-plugin)
- [Quick Start](#quick-start)
- [CLI Reference](#cli-reference)
- [Build a Plugin](#build-a-plugin)
- [Pocker Studio (Web UI)](#pocker-studio-web-ui)
- [Roadmap](#roadmap)
- [Recommended GitHub Topics & SEO Keywords](#recommended-github-topics--seo-keywords)
- [Contributing](#contributing)
- [License](#license)

---

## Why Pocker

The agent ecosystem has pivoted from "calling a model" to "composing systems." Three trends shape Pocker's design:

1. **Rust is eating AI infrastructure.** Agent runtimes like Warp, OpenAI Codex CLI, and browser-use are going Rust for speed and safety. Pocker is Rust from the engine up — single static binary, no runtime dependency, sandboxed execution.
2. **Skills are the new unit of reuse.** The explosion of *Claude Skills* / agent-skills frameworks shows developers want composable, versioned, shareable capabilities — not 2,000-word prompts. Pocker makes every skill a typed plugin (`ctx.skills`, `ctx.prompt`).
3. **Agents need governance, not just capability.** Tool calling, approval flows, and sandbox isolation are now table stakes. Pocker treats the sandbox and approval policy as first-class, replaceable seams.

Pocker gives you a **plugin marketplace model** (à la Docker Hub) for agent capabilities: publish an LLM adapter, a tool set, or a skill; discover and compose them per-profile.

---

## Features

- **Plugin-first architecture** — no privileged core. LLM, tools, skills, sandbox, session, UI are all plugins.
- **Typed capability seams** — register services on `ctx.llm`, `ctx.tools`, `ctx.skills`, `ctx.sandbox`, `ctx.session`, `ctx.approval`, `ctx.fs`, `ctx.bus`, `ctx.credentials`, and more.
- **Multi-provider LLM adapters** — OpenAI, Anthropic, Ollama, DeepSeek (and your own) as drop-in plugins. Real **token streaming** via async streams.
- **Tool calling & function calling** — register tools as plugins; the agent loop invokes them through `ctx.tools`.
- **Skill framework** — package reusable, schema-validated capabilities (`ctx.skills`) and Markdown instruction skills (`ctx.prompt`) the way the community ships "agent skills."
- **Profiles & Bundles** — compose plugins into named, layered profiles (`web`, `cli`, `headless`, `tui`) with override-able patches.
- **Sandboxed execution** — cross-platform process isolation via the `ctx.sandbox` seam; approval policies for shell, filesystem, and network actions.
- **Rust speed, terminal-native UX** — a zero-dependency CLI and a `ratatui` TUI, plus a React/TanStack web UI (Pocker Studio).
- **Self-hosted & local-first** — run fully offline; bring your own models via Ollama; own your data and credentials.
- **Open, MCP-friendly seams** — capability seams are protocol-shaped, so bridging to the Model Context Protocol (MCP) tool/resource model is a thin adapter, not a rewrite.

---

## DeepSeek Harness: A Supercharged AI Coding Experience

Pocker is designed to ride the **deepseek-harness** family — a pair of open projects that lift AI-assisted coding from "smart autocomplete" to an **autonomous workshop**. They are the reference architecture for Pocker's agent design, and an official `deepseek-harness` plugin is tracked on the [Roadmap](#roadmap).

### `deepseek-harness` — an elevated developer experience

Mounted on Pocker's `ctx.llm` seam, `deepseek-harness` turns the model into a **reasoning-aware coding partner** rather than a token predictor:

- **Repository-grounded context** — awareness that spans your whole codebase, so suggestions respect the real architecture instead of a narrow 200-line window.
- **Agentic task loops** — plan → edit → test → self-review cycles that execute *inside* the harness and surface through `ctx.tools` and `ctx.skills`.
- **Streaming, inspectable runs** — every step is a typed event on `ctx.bus`, so the CLI, TUI, and Studio render the exact same live trace.

### `deepseek-harness-desktop` — native, cross-platform integration

`deepseek-harness-desktop` wraps the harness in a **first-class desktop shell** that behaves identically on Linux, macOS, and Windows — the natural host for Pocker's CLI / TUI / Studio trio:

- **One harness, three surfaces** — the same agent backend powers your terminal, your TUI, and your web Studio with zero duplication.
- **Local-first by default** — models run on your machine via Ollama; nothing leaves the box unless you opt in.
- **OS-native glue** — file watchers, global hotkeys, and system notifications fold the agent into your daily flow.

### The net effect on your coding workflow

Together they collapse the distance between *thinking* and *shipping*:

> A task arrives as a sentence, becomes a plan on `ctx.skills`, is executed by `deepseek-harness` through `ctx.tools`, is sandboxed on `ctx.sandbox`, and lands back in your editor — all on one typed, plugin-driven pipeline you can audit, replay, and swap.

That is the Pocker promise: **the model does the work; the plugin model keeps you in control.**

#### Reference projects

```text
deepseek-harness          Reasoning-aware coding harness (LLM / agent layer)
deepseek-harness-desktop  Cross-platform desktop shell for the harness
```

*These are upstream reference projects for Pocker's agent design. An official
`deepseek-harness` plugin (mounted on `ctx.llm` / `ctx.agent`) is tracked on the
[Roadmap](#roadmap).*

---

## Architecture: Everything is a Plugin

Pocker has **no privileged core**. The engine is just a runtime that loads plugins into a shared `Ctx`. Each plugin contributes services to one or more **seams** — named extension points — and reverses its registrations on unmount.

```
                        User Interface Plugins (also plugins!)
                 ┌──────────────┬───────────────┬──────────────┐
                 │   CLI (clap) │  TUI (ratatui)│ Web (Studio) │
                 └──────┬───────┴───────┬───────┴──────┬───────┘
                        └───────────────┼───────────────┘
                                       ▼
                         ┌──────────────────────────────┐
                         │     Shared Context  (Ctx)    │
                         │  ┌────────────────────────┐  │
                         │  │  ctx.llm      adapters │  │  ← LLM plugins
                         │  │  ctx.tools    registry │  │  ← tool plugins
                         │  │  ctx.skills   registry │  │  ← skill plugins
                         │  │  ctx.prompt   library  │  │  ← instruction skills
                         │  │  ctx.sandbox  executor │  │  ← sandbox plugins
                         │  │  ctx.session  log      │  │
                         │  │  ctx.approval policy   │  │
                         │  │  ctx.fs / ctx.bus / …  │  │
                         │  └────────────────────────┘  │
                         └───────────────┬──────────────┘
                                         ▼
                  ┌────────────────────────────────────────┐
                  │  Pocker Engine (plugin loader + events) │
                  │  + Pocker Hub (plugin registry/distro)  │
                  └────────────────────────────────────────┘
```

Standard seams you can extend:

| Seam | Key | What mounts here |
|------|-----|------------------|
| LLM adapter | `ctx.llm` | OpenAI / Anthropic / Ollama / DeepSeek / custom |
| Tool registry | `ctx.tools` | shell / fs / http / git / MCP tools |
| Skills | `ctx.skills` | schema-validated, reusable capabilities |
| Instruction skills | `ctx.prompt` | Harness-style Markdown skills discovered by the LLM |
| Sandbox | `ctx.sandbox` | process isolation / container / WASM |
| Session | `ctx.session` | SQLite / PostgreSQL / file-backed memory |
| Approval | `ctx.approval` | manual / allowlist / policy engine |
| Filesystem | `ctx.fs` | local / remote / virtual |
| Event bus | `ctx.bus` | in-process / Redis / NATS |
| Credentials | `ctx.credentials` | file / OS keychain / Vault |

---

## Why "Pocker": Docker Meets Plugin

The name says it all: **Pocker = Portable + Container.** Pocker borrows Docker's core design idea — *package any capability as an immutable, portable, composable unit* — and adapts it to AI agents by fusing it with the **plugin** concept from `deepseek-harness`.

**One decisive reframing:** in Pocker's model the minimal, content-addressable unit is the **plugin**, not the container. A container is a *runtime instance* — mutable, ephemeral, identified by a process/runtime handle. A plugin is *content*: a self-contained, versioned, immutable bundle whose **content hash is its identity**. You address, cache, and reproduce plugins by hash; you never address a container.

| Docker (apps) | Pocker (agents) |
|---|---|
| Image / Container | **Plugin** (the minimal hashable unit) |
| Layer digest (content address) | **Plugin digest** (content address) |
| `docker-compose` stack | Profile (layered plugin composition) |
| Container runtime | Engine + typed **seams** |
| Docker Hub | Pocker Hub (plugin registry) |
| `Containerfile` / lockfile | `--dump-config` plugin tree (reproducible) |
| Isolate processes | Isolate capabilities (mount / unmount on seams) |

### Why the plugin — not the container — is the minimal hash unit

- **Indivisible by design.** A plugin is the smallest artifact you can build, sign, publish, and address on its own. It carries its own manifest, capabilities, and metadata; you cannot partially hash "half a plugin." A container is just a live instantiation of a plugin graph and mutates as it runs.
- **Identity = content hash.** A plugin's digest is computed from its bytes — same bytes, same hash, same plugin, anywhere. A container's identity is a runtime handle that is meaningless once it stops.
- **Cache key = plugin digest.** Builds and runs are cached by plugin hash. Two agents sharing a plugin reuse the exact same cached bits — no rebuild, no re-download. Container-layer caching is coarser and tied to image build order.
- **Content addressing is the source of truth.** `ctx` resolves a plugin by its digest; the registry, the Hub, and the lockfile all agree on that digest. There is no separate mutable `latest` pointer that can silently drift.

### How the plugin-as-atom model reshapes the mechanics

- **Image build → plugin assembly.** "Building" an agent means assembling a directed graph of plugins. Each node is content-addressed, so builds are deterministic and incremental.
- **Layer reuse → plugin cache reuse.** Because the unit is the plugin, reuse is granular: swap one skill plugin and only that digest changes; everything else stays cached and reproducible.
- **Dependency management → locked plugin trees.** Dependencies are declared as a plugin graph and resolved to a locked set of digests (`--dump-config`). Same inputs → same resolved tree on every machine, exactly like a pinned image — except the atom is the plugin, not the container.

> **Implemented today.** The plugin-as-atom model is live in the code: every `PluginMetadata` carries a deterministic `digest()` (SHA-256 over name, version, type, `provides`, `requires`, and an optional `code_hash`); each seam registry entry records the owning plugin's `PluginDigest`; `Ctx::register_plugin` records mounted plugins into a content-addressed ledger; and `pocker --dump-config` emits `seam_providers` (seam → `[(provider, digest)]`) and `plugin_digests` (digest → `{ name, version }`) — a reproducible lockfile proving exactly which plugin "layers" are active.

**What the fusion buys you:**

- **Isolation without containers.** A plugin can never corrupt the core — it mounts a service on a typed seam and reverses every registration on unmount. Swap LLM, tools, or UI with zero risk to the runtime.
- **Composability as a first principle.** Profiles compose plugins the way `docker-compose` stacks services: declarative, layered, override-able. `web`, `cli`, `tui`, and `headless` are simply different stacks.
- **True portability.** A plugin authored on Linux runs unchanged on macOS and Windows — the `deepseek-harness-desktop` shell supplies the OS-native substrate.
- **A marketplace, not a monastery.** Pocker Hub mirrors Docker Hub: publish, version, rate, and pull plugins. Capabilities become shareable currency.
- **Reproducible by construction.** A locked plugin tree (`--dump-config`) is the agent-world equivalent of a pinned image — same inputs, same behavior, anywhere.

Pocker is, in one line, **the container runtime for AI agents — with the plugin as its true atomic unit**: Docker gave applications portability and an ecosystem; Pocker gives agent *capabilities* the same — now plugin-shaped, harness-powered, hash-addressed, and yours to compose.

---

## Quick Start

### Prerequisites

- **Rust** 1.80+ (edition 2021) — `cargo` from [rustup](https://rustup.rs/)
- **Node.js** 20+ (only for the Pocker Studio web frontend)
- An LLM API key exported in the environment (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, or a running [Ollama](https://ollama.com/) for local models)

### Build from source

```bash
# Clone the repository
git clone https://github.com/pocker/pocker.git
cd pocker

# Build all Rust crates (engine, CLI, TUI, Hub, Studio, SDK)
cargo build --release

# Run the test suite (50+ tests across core crates)
cargo test

# Launch Pocker Studio (Web UI) — defaults to http://127.0.0.1:3080
cargo run -p pocker-cli -- web

# Or start the headless API server (Pocker Hub)
cargo run -p pocker-cli -- headless --port 3080

# Inspect the resolved plugin tree
cargo run -p pocker-cli -- --dump-config
cargo run -p pocker-cli -- system info
```

### Pocker Studio frontend (optional)

```bash
cd studio-web
npm install
npm run dev      # dev server at http://localhost:3000
npm run build    # production build to dist/
```

> **Status:** The core engine, plugin system, seams, CLI, Hub server skeleton, Studio backend, and SDK are implemented and covered by tests. The TUI, full Hub marketplace, skill execution, and MCP bridging are on the [Roadmap](#roadmap).

---

## CLI Reference

```text
pocker [--profile <name>] [--dump-config] <command>

Commands:
  web                 Start the Web UI (Pocker Studio) at :3080
  tui                 Start the terminal UI (ratatui)        [planned]
  headless [--port]   Start the headless API server (Hub)   [default 3080]
  plugin list         List installed plugins
  plugin info <name>  Show plugin metadata
  profile list        List all profiles
  profile create <n>  Create a new profile
  profile switch <n>  Load and switch to a profile
  run <skill> [--input <json>]   Run a skill directly
  hub search <query>  Search the plugin Hub
  hub info <name>     Show plugin info from the Hub
  system info         Show engine / plugin / seam counts
```

Examples:

```bash
# Default profile boots automatically; list what's mounted
cargo run -p pocker-cli -- system info

# Switch to a different composition of plugins
cargo run -p pocker-cli -- profile switch cli

# Inspect the full plugin tree as YAML
cargo run -p pocker-cli -- --dump-config
```

---

## Build a Plugin

A plugin implements the `Plugin` trait: `mount` registers services on seams, `unmount` reverses them. The SDK (`pocker-plugin`) ships ready-made traits for the common seams.

### 1. An LLM adapter plugin

```rust
use async_trait::async_trait;
use pocker_core::{seam::SeamId, Ctx, error::Result, plugin::{Plugin, PluginMetadata}};
use pocker_plugin::{LlmAdapter, LlmSeam};
use std::sync::Arc;

struct MyAdapter;

#[async_trait]
impl LlmAdapter for MyAdapter {
    fn name(&self) -> &str { "my-model" }
    fn stream(&self, _messages: Vec<pocker_core::types::Message>,
              _opts: pocker_core::types::LlmOptions)
        -> Result<futures::stream::BoxStream<'static, Result<pocker_core::types::Chunk>>> {
        unimplemented!("yield Chunk::Text deltas here")
    }
    fn list_models(&self, _provider: &str) -> Result<Vec<pocker_core::types::ModelInfo>> {
        Ok(vec![])
    }
    fn capabilities(&self) -> pocker_core::types::LlmCapabilities {
        pocker_core::types::LlmCapabilities { function_calling: true, vision: false, streaming: true }
    }
}

struct MyPlugin { meta: PluginMetadata }

#[async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> &PluginMetadata { &self.meta }
    async fn mount(&self, ctx: &Arc<Ctx>) -> Result<()> {
        ctx.register_seam(
            SeamId::llm(),
            "my-model".into(),
            self.meta.digest(),
            Arc::new(LlmSeam { adapter: Arc::new(MyAdapter) }),
        );
        Ok(())
    }
    async fn unmount(&self, ctx: &Arc<Ctx>) -> Result<()> {
        ctx.unregister_seam(&SeamId::llm(), "my-model");
        Ok(())
    }
}
```

### 2. A skill plugin

```rust
use async_trait::async_trait;
use pocker_core::error::Result;
use pocker_core::types::{SkillType, SkillDefinition};
use pocker_plugin::{Skill, SkillResult};
use std::sync::Arc;

struct CountSkill;

#[async_trait]
impl Skill for CountSkill {
    fn name(&self) -> &str { "count-items" }
    fn version(&self) -> &str { "1.0.0" }
    fn description(&self) -> &str { "Count elements in a JSON array" }
    fn skill_type(&self) -> SkillType { SkillType::Tool }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object",
            "properties": { "items": { "type": "array" } },
            "required": ["items"] })
    }
    fn output_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object",
            "properties": { "count": { "type": "integer" } } })
    }
    fn requires(&self) -> Vec<String> { vec![] }
    async fn execute(&self, input: serde_json::Value) -> Result<SkillResult> {
        let n = input["items"].as_array().map_or(0, |a| a.len());
        Ok(SkillResult::ok(serde_json::json!({ "count": n })))
    }
}
```

Register skills on the `ctx.skills` seam the same way — `skill_registry(ctx)?.register_skill(Arc::new(CountSkill))`.

---

## Pocker Studio (Web UI)

Pocker Studio is the official web control plane, built with **React 19 + TanStack (Router/Query) + Zustand + TailwindCSS 4 + xterm**, served by the Rust `axum` backend.

Planned surfaces (mapped to seams):

- **Chat** — streamed agent turns with live tool-call rendering
- **Plugins** — install / enable / disable plugins
- **Skills** — browse and run the skill library
- **Sandbox** — execute code in an isolated environment
- **Approvals** — review and allow/deny shell, filesystem, and network actions
- **Sessions** — browse and replay conversation history
- **Hub** — search, rate, and publish plugins
- **Profiles** — switch and edit plugin compositions

Run it with `cargo run -p pocker-cli -- web`, then open `http://127.0.0.1:3080`.

---

## Roadmap

Pocker is early but real. Near-term priorities, aligned with where the agent ecosystem is heading:

- [ ] **MCP bridging** — expose `ctx.tools` / `ctx.skills` as Model Context Protocol servers and consume external MCP servers as plugins.
- [ ] **Pocker Hub marketplace** — publish / pull / search / version / rate plugins; signature & security scanning.
- [ ] **TUI** — ship the `ratatui` terminal UI as a first-class profile.
- [ ] **Skill execution & multi-agent orchestration** — run skills end-to-end and coordinate agent crews.
- [ ] **Sandbox hardening** — Linux (namespaces + cgroups + seccomp), Windows (Job Object + AppContainer), macOS (seatbelt); optional WASM/container isolation.
- [ ] **Agent memory** — persistent, graph-based session memory on `ctx.session`.
- [ ] **Built-in library** — 10+ LLM adapters and 20+ starter skills out of the box.

---

## Recommended GitHub Topics & SEO Keywords

Copy these into **Repository settings → Topics** to maximize discovery:

```text
rust, ai-agent, ai-agent-framework, agent-framework, plugin-architecture,
plugin-system, extensible, llm, llm-framework, mcp, model-context-protocol,
agent-skills, skills, tool-calling, function-calling, self-hosted, local-first,
cli, tui, terminal, chatbot, rag, sandbox, multi-agent, async, tokio,
openai, anthropic, ollama, deepseek, cross-platform,
deepseek-harness, ai-coding-agent, coding-assistant, coding-workflow,
desktop, cross-platform-desktop, devtools
```

SEO keyword string (for descriptions, READMEs, and docs site metadata):

> Rust AI agent framework, plugin architecture, everything is a plugin, Model Context Protocol, MCP, agent skills, tool calling, function calling, self-hosted AI, local-first LLM, terminal AI agent, async Rust agent runtime, multi-agent orchestration, LLM adapter, sandbox execution, OpenAI Anthropic Ollama DeepSeek. DeepSeek Harness, AI coding agent, coding assistant, cross-platform desktop, developer workflow, Docker-for-agents.

---

## Contributing

Plugins are the whole point — contributions are welcome at every seam.

1. Fork and clone the repo.
2. `cargo test` must pass for the crates you touch.
3. Add a plugin (or extend a seam) following the patterns in `crates/pocker-plugin`.
4. Open a PR describing which seam you extend and why.

See [`docs/规划架构.md`](docs/规划架构.md) for the full architecture and design decisions.

---

## License

[MIT](LICENSE) © Pocker Contributors
