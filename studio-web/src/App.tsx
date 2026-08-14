import { useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { ChatScreen } from './screens/ChatScreen';
import { PluginsScreen } from './screens/PluginsScreen';
import { SkillsScreen } from './screens/SkillsScreen';
import { SettingsScreen } from './screens/SettingsScreen';
import { DashboardScreen } from './screens/DashboardScreen';

export type Screen = 'dashboard' | 'chat' | 'plugins' | 'skills' | 'settings';

export default function App() {
  const [screen, setScreen] = useState<Screen>('dashboard');

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-neutral-950 text-neutral-200">
      <Sidebar current={screen} onNavigate={setScreen} />
      <main className="flex-1 overflow-auto">
        {screen === 'dashboard' && <DashboardScreen />}
        {screen === 'chat' && <ChatScreen />}
        {screen === 'plugins' && <PluginsScreen />}
        {screen === 'skills' && <SkillsScreen />}
        {screen === 'settings' && <SettingsScreen />}
      </main>
    </div>
  );
}
