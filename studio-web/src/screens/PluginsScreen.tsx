export function PluginsScreen() {
  return (
    <div className="p-6">
      <h1 className="mb-4 text-2xl font-bold text-neutral-100">Plugins</h1>
      <div className="rounded-xl border border-neutral-800 bg-neutral-900 p-8 text-center text-neutral-500">
        No plugins loaded. Install plugins using:
        <code className="mx-2 rounded bg-neutral-800 px-2 py-1 text-sm text-blue-400">
          pocker plugin install &lt;name&gt;
        </code>
      </div>
    </div>
  );
}
