export function DashboardScreen() {
  return (
    <div className="p-6">
      <h1 className="mb-4 text-2xl font-bold text-neutral-100">Dashboard</h1>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        <div className="rounded-xl border border-neutral-800 bg-neutral-900 p-4">
          <h3 className="text-sm font-medium text-neutral-400">Active Profile</h3>
          <p className="mt-1 text-xl font-bold text-blue-400">web</p>
        </div>
        <div className="rounded-xl border border-neutral-800 bg-neutral-900 p-4">
          <h3 className="text-sm font-medium text-neutral-400">Plugins</h3>
          <p className="mt-1 text-xl font-bold text-green-400">0</p>
        </div>
        <div className="rounded-xl border border-neutral-800 bg-neutral-900 p-4">
          <h3 className="text-sm font-medium text-neutral-400">Seams</h3>
          <p className="mt-1 text-xl font-bold text-purple-400">0</p>
        </div>
      </div>

      <div className="mt-6 rounded-xl border border-neutral-800 bg-neutral-900 p-4">
        <h2 className="mb-2 text-lg font-semibold text-neutral-200">Welcome to Pocker Studio</h2>
        <p className="text-sm text-neutral-400">
          Pocker is a Plugin as a Service system where everything is a plugin — LLM adapters,
          tools, skills, sandboxes, and UI components. Use the sidebar to navigate.
        </p>
      </div>
    </div>
  );
}
