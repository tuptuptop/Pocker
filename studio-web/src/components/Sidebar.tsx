import clsx from 'clsx';
import type { Screen } from '../App';

const NAV_ITEMS: { id: Screen; label: string; icon: string }[] = [
  { id: 'dashboard', label: 'Dashboard', icon: '◉' },
  { id: 'chat', label: 'Chat', icon: '💬' },
  { id: 'plugins', label: 'Plugins', icon: '🔌' },
  { id: 'skills', label: 'Skills', icon: '⚡' },
  { id: 'settings', label: 'Settings', icon: '⚙' },
];

export function Sidebar({
  current,
  onNavigate,
}: {
  current: Screen;
  onNavigate: (screen: Screen) => void;
}) {
  return (
    <aside className="flex w-60 flex-col border-r border-neutral-800 bg-neutral-950">
      <div className="flex items-center gap-2 px-4 py-4">
        <span className="text-xl font-bold text-blue-500">P</span>
        <span className="text-lg font-semibold text-neutral-200">Pocker Studio</span>
      </div>
      <nav className="flex-1 px-2 py-2">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            onClick={() => onNavigate(item.id)}
            className={clsx(
              'mb-1 flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors',
              current === item.id
                ? 'bg-blue-600/20 text-blue-400'
                : 'text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200'
            )}
          >
            <span className="text-base">{item.icon}</span>
            <span>{item.label}</span>
          </button>
        ))}
      </nav>
      <div className="border-t border-neutral-800 px-4 py-3 text-xs text-neutral-600">
        Pocker v0.1.0
      </div>
    </aside>
  );
}
