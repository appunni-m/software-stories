import { useEffect, useState } from 'react';
import { Monitor, Moon, Sun } from 'lucide-react';
export type ThemeMode = 'system' | 'light' | 'dark';
const key = 'code-atlas:theme';
export function ThemeControl() {
  const [mode, setMode] = useState<ThemeMode>(() => {
    try { const value = localStorage.getItem(key); if (value === 'light' || value === 'dark') return value; } catch { /* Optional. */ }
    return 'system';
  });
  useEffect(() => {
    document.documentElement.dataset.theme = mode;
  }, [mode]);
  const chooseMode = (value: ThemeMode) => {
    setMode(value);
    try { localStorage.setItem(key, value); } catch { /* Appearance still applies to this window. */ }
  };
  const Icon = mode === 'light' ? Sun : mode === 'dark' ? Moon : Monitor;
  return <label className="theme-control"><Icon size={15} /><span className="sr-only">Appearance</span><select aria-label="Appearance" value={mode} onChange={(event) => chooseMode(event.target.value as ThemeMode)}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label>;
}
