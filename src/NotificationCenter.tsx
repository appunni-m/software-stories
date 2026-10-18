import { useRef, useState } from 'react';
import * as Popover from '@radix-ui/react-popover';
import { Bell, CircleAlert, Info, X } from 'lucide-react';
import { notificationSummary, type NotificationAvailability } from './lib/notificationSummary';
import './NotificationCenter.css';

export interface WorkspaceNotification {
  id: string;
  title: string;
  message: string;
  tone?: 'info' | 'warning' | 'error';
  read?: boolean;
  actions?: { label: string; run: () => void; disabled?: boolean; opensDialog?: boolean; movesFocus?: boolean }[];
}

/** Background work never opens this panel or moves the reader's focus. */
export function NotificationCenter({ items, availability = 'ready', open: controlledOpen, onOpenChange }: {
  items: WorkspaceNotification[]; availability?: NotificationAvailability; open?: boolean; onOpenChange?: (open: boolean) => void;
}) {
  const [localOpen, setLocalOpen] = useState(false);
  const open = controlledOpen ?? localOpen;
  const setOpen = onOpenChange ?? setLocalOpen;
  const actionOpened = useRef(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const summary = notificationSummary(items, availability);
  return <Popover.Root open={open} onOpenChange={setOpen} modal={false}>
    <Popover.Trigger asChild>
      <button ref={trigger} type="button" className="notification-trigger" data-focus-key="notifications" aria-label={summary.label} aria-busy={availability === 'loading'} title="Notifications">
        <Bell size={17} />
        {summary.badge && <span className="notification-count" aria-hidden="true">{summary.badge}</span>}
      </button>
    </Popover.Trigger>
    <span className="notification-announcement" role="status" aria-live="polite" aria-atomic="true">{summary.announcement}</span>
    <Popover.Portal container={trigger.current?.closest<HTMLElement>('.radix-themes') ?? undefined}>
      <Popover.Content className="notification-panel" align="end" sideOffset={8} collisionPadding={12}
        aria-label="Notifications" onOpenAutoFocus={() => { actionOpened.current = false; }}
        onCloseAutoFocus={(event) => { if (actionOpened.current) event.preventDefault(); }}>
        <header className="notification-panel-header"><strong>Notifications</strong><Popover.Close asChild><button type="button" className="notification-close" aria-label="Close notifications"><X size={16} /></button></Popover.Close></header>
        {summary.status && <p className="notification-empty">{summary.status}</p>}
        {items.length === 0 ? availability === 'ready' && <p className="notification-empty">You’re all caught up.</p> : <ul className="notification-list">{items.map((item) => <li key={item.id} className={`notification-item tone-${item.tone ?? 'info'}`}>
          <div className="notification-item-title">{item.tone === 'error' || item.tone === 'warning' ? <CircleAlert size={16} /> : <Info size={16} />}<strong>{item.title}</strong></div>
          <p>{item.message}</p>
          {item.actions?.length ? <div className="notification-item-actions">{item.actions.map((action) => <button type="button" key={action.label} disabled={action.disabled} onClick={() => {
            actionOpened.current = Boolean(action.opensDialog || action.movesFocus);
            setOpen(false);
            action.run();
          }}>{action.label}</button>)}</div> : null}
        </li>)}</ul>}
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>;
}
