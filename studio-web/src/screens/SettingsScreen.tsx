export function SettingsScreen() {
  return (
    <div className="p-6">
      <h1 className="mb-4 text-2xl font-bold text-neutral-100">Settings</h1>
      <div className="space-y-4">
        <div className="rounded-xl border border-neutral-800 bg-neutral-900 p-4">
          <h2 className="mb-3 text-lg font-semibold text-neutral-200">Profile</h2>
          <select className="w-full rounded-lg border border-neutral-700 bg-neutral-800 px-3 py-2 text-sm text-neutral-200">
            <option value="web">web</option>
            <option value="cli">cli</option>
            <option value="tui">tui</option>
            <option value="headless">headless</option>
          </select>
        </div>
        <div className="rounded-xl border border-neutral-800 bg-neutral-900 p-4">
          <h2 className="mb-3 text-lg font-semibold text-neutral-200">LLM Configuration</h2>
          <p className="text-sm text-neutral-500">
            Configure LLM adapters in your profile YAML. Example:
          </p>
          <pre className="mt-2 rounded-lg bg-neutral-950 p-3 text-xs text-green-400">
{`config:
  - id: llm-openai
    plugin: @pocker/llm-openai
    provides: ctx.llm
    config:
      api_key: \${env:OPENAI_API_KEY}
      default_model: gpt-4o`}
          </pre>
        </div>
      </div>
    </div>
  );
}
