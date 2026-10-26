import { useEffect, useState } from 'react';
import { Select } from '@radix-ui/themes';
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
  return <div className="theme-control"><Icon size={15} aria-hidden="true" /><Select.Root size="1" value={mode} onValueChange={(value) => {
    if (value === 'system' || value === 'light' || value === 'dark') chooseMode(value);
  }}>
    <Select.Trigger className="theme-control-trigger" variant="ghost" color="gray" aria-label="Appearance" />
    <Select.Content position="popper">
      <Select.Item value="system">System</Select.Item>
      <Select.Item value="light">Light</Select.Item>
      <Select.Item value="dark">Dark</Select.Item>
    </Select.Content>
  </Select.Root></div>;
}
