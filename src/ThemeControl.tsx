import { useEffect, useState } from 'react';
import { Monitor, Moon, Sun } from 'lucide-react';
import { AppSelect } from './design/AppSelect';
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
  return <div className="theme-control"><Icon size={15} aria-hidden="true" /><AppSelect ariaLabel="Appearance" className="theme-control-trigger" size="1" variant="ghost" value={mode} options={[{value:'system',label:'System'},{value:'light',label:'Light'},{value:'dark',label:'Dark'}]} onValueChange={(value) => {
    if (value === 'system' || value === 'light' || value === 'dark') chooseMode(value);
  }} /></div>;
}
