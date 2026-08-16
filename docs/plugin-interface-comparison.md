# Pocker Studio ↔ DeepSeek Harness 插件接口对比

> 目的：判断 Harness（Cordis）的插件规范哪些能直接复用进 Pocker 的 Rust Seam 模型。
> 两者均为 MIT，vendored 代码与接口模式可自由借鉴。

## 1. 插件生命周期与注册模型

| 维度 | DeepSeek Harness (Cordis) | Pocker (Rust) |
|------|---------------------------|---------------|
| 插件形态 | `class Plugin { static inject = [...]; apply(ctx) {} }` | `trait Plugin { metadata(); mount(ctx); unmount(ctx) }` |
| 依赖注入 | 声明式 `inject` 数组，框架按名字注入 `ctx.xxx` 服务 | 显式 `requires: Vec<String>` 声明 seam 依赖 |
| 卸载 | 隐式：fiber（作用域上下文）销毁时自动回收 Service | 显式：`unmount(ctx)` 反向注销所有 seam 注册 |
| 隔离 | 基于 fiber 的作用域树（isolate） | 基于 `Arc<Ctx>` 共享总线，无作用域隔离 |
| 可逆性 | 隐式、依赖 GC/作用域 | **显式、强制**——`mount` 注册的东西必须在 `unmount` 反注册 |

**判断**：Pocker 的 `mount`/`unmount` 显式可逆模型比 Cordis 的隐式 fiber 回收**更可预测、更易调试**，保留。Cordis 的 `inject` 声明式注入值得借鉴为编译期 seam 依赖校验（Pocker 目前 `requires` 只是元数据，运行时未强制检查）。

## 2. 能力扩展点（Service / Seam）

| | Harness | Pocker |
|---|---------|--------|
| 机制 | `class Service { constructor(ctx, name) }` → `ctx.reflect.provide(name, this)` 挂到 `ctx.<name>` | `SeamRegistry` 按 `SeamId`（如 `ctx.llm`）存 `Arc<dyn Seam>`，`get()` 取第一个 |
| 多实现 | 同名 service 后被覆盖（单值） | 同 seam 允许多 provider（`Vec<SeamEntry>`），`get()` 取默认 |
| 标准扩展点 | `ctx.model` `ctx.tool` `ctx.skills` `ctx.fs` `ctx.terminal` `ctx.http` 等 | `ctx.llm` `ctx.tools` `ctx.skills` `ctx.sandbox` `ctx.fs` `ctx.terminal` `ctx.bus` 等 |

**判断**：命名几乎一一对应（`ctx.model`↔`ctx.llm`，`ctx.tool`↔`ctx.tools`，`ctx.skills`↔`ctx.skills`）。Pocker 的"一个 seam 允许多 provider 择优"比 Harness 的"同名覆盖"更适合插件市场竞争。保留并强化。

## 3. LLM 适配器接口（关键差异 ⚠️）

**Harness `LlmAdapter`（来自 vendored 类型声明）：**
```ts
abstract class LlmAdapter {
  providerInfo(provider: string): LlmProviderInfo;
  providerRetryPolicy(provider: string): ResolvedRetryPolicy | undefined;
  listModels(provider: string): Promise<readonly LlmModelInfo[]>;
  resolveModel(provider: string, model: string, signal?): Promise<LlmResolvedModelInfo>;
  abstract stream(options: GenerateOptions): AsyncIterable<StreamChunk>; // 真·异步流
}
```

**Pocker `LlmAdapter`（crates/pocker-plugin/src/llm.rs）：**
```rust
trait LlmAdapter {
  fn name(&self) -> &str;
  async fn stream(&self, messages, options) -> Result<Vec<Chunk>>;   // ⚠️ 名为 stream，实为一次性收集
  async fn generate(&self, messages, options) -> Result<String>;
  fn models(&self) -> Vec<ModelInfo>;                                // ⚠️ 扁平，无 provider 维度
  fn capabilities(&self) -> LlmCapabilities;
}
```

**问题清单：**
1. **Pocker `stream` 返回 `Vec<Chunk>` 是伪流式**——把整段流收集完才返回。Harness 用 `AsyncIterable<StreamChunk>`（拉取式异步迭代）。Pocker 应改为流式输出（`tokio_stream` / `futures::Stream<Chunk>`），否则长文本会阻塞且无法逐字渲染。
2. **Provider 维度缺失**——Harness 每个方法都带 `provider` 参数（多供应商感知）；Pocker 的 `models()` 是扁平 `Vec<ModelInfo>`，无法区分 openai/ollama/deepseek 归属。建议模型信息带 `provider` 字段。
3. **缺重试策略**——Harness 有 `providerRetryPolicy`，Pocker 无。建议 seam 元数据补充。

## 4. Tool 定义接口

**Harness `defineTool`（`packages/core/tools/src/schema.ts`）：** 强类型 `parameters` schema + `output.schema` + **输出 render/presentation** + `timeoutMs` + `isConcurrencySafe` + `presentCall`/`presentResult`。注册前自动做 JSON Schema 校验。

**Pocker `Tool` trait：** `name` / `description` / `input_schema` / `execute(args) -> ToolResult`。`ToolResult { success, output, error }`。

**判断：**
- Pocker 的 `Tool` 是 lean 版，够用。
- **建议吸收**：① `output` 输出 schema（Pocker 只有输入 schema）；② `timeoutMs`（超时预算）；③ 可选的 present 元数据（用于 UI 卡片）。这些作为 seam 元数据可选实现即可，不破坏现有 trait。

## 5. Skill 模型（最大语义分歧 ⚠️）

| | Harness `Skill` | Pocker `Skill` |
|---|-----------------|----------------|
| 本质 | **Markdown 指令文档**：`SkillDefinition { name, description, whenToUse, content }` | **带类型的可执行能力**：`trait Skill { input_schema, output_schema, requires, execute() -> SkillResult }` |
| 发现 | LLM 路由发现 → 注入上下文，模型"照做" | 注册为可执行单元，被 agent loop / tools 调用 |
| 执行 | 模型遵循 instruction 文本（无强类型边界） | 强类型 `execute`，有输入/输出 schema 校验 |

**判断**：这是两种范式，不是谁优谁劣：
- Harness 的 skill 是**提示词/指令路由**（prompt library），靠 LLM 理解执行。
- Pocker 的 skill 是**类型化函数**（类似高阶 tool）。

**建议**：Pocker 保留强类型 `Skill`（更适合可信执行与结果校验）；同时**新增一个轻量 `ctx.prompt`/instruction seam**，用来承载 Harness 式的"指令型 skill 库"。两者并存，不互相替代。不要在 `Skill` trait 里塞 Markdown 正文——那会稀释类型边界。

## 6. 可执行建议（按 ROI 排序）

1. **[高/必修] 修 Pocker `LlmAdapter::stream` 伪流式** → 改成 `Stream<Chunk>`，否则 Studio 无法逐字渲染、长任务阻塞。这是当前最实在的缺陷。
2. **[高] 模型信息加 `provider` 维度** → 对齐 Harness 多供应商模型解析。
3. **[中] `requires` seam 做运行时强制校验** → 借鉴 Cordis `inject`，`mount` 时检查依赖 seam 是否存在，缺失直接报错而非静默。
4. **[中] Tool 补 `output_schema` + `timeout_ms`** → 复用 Harness `defineTool` 的 schema 编译思路（MIT，可直接搬校验逻辑）。
5. **[低/可选] 新增 instruction/prompt seam** → 承接 Harness 式文档型 skill，不污染现有 typed Skill。
6. **[保留] `mount`/`unmount` 显式可逆模型** → 比 Cordis 隐式 fiber 回收更稳，不要为了"像 Harness"而改掉。

## 结论

Harness 的 Cordis 规范与 Pocker Seam 模型**同构**（命名几乎一一对应），可低成本对齐。但有三处 Pocker 当前偏弱或被 Harness 反超：**真流式 LLM、provider 维度的模型解析、Tool 输出 schema/超时**。最大分歧在 Skill 范式（指令文档 vs 类型函数），建议并存而非融合。

真正该持续同步的不是 `deepseek-harness-desktop`（只是 Electron 壳），而是 `deepseek-ai/deepseek-harness` 本体的 `vendor/cordis` 与 `packages/core` —— 它们才是接口规范的源头。
