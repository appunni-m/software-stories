import { useRef, useState } from 'react';
import * as Popover from '@radix-ui/react-popover';
import { Bell, CircleAlert, Info, X } from 'lucide-react';
import { groupNotifications, notificationSummary, type NotificationAvailability, type NotificationBucket } from './lib/notificationSummary';
import './NotificationCenter.css';

export interface WorkspaceNotification {
  id: string;
  title: string;
  message: string;
  tone?: 'info' | 'warning' | 'error';
  read?: boolean;
  bucket?: NotificationBucket;
  groupKey?: string;
  countLabel?: string;
  detail?: string;
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
  const groups = groupNotifications(items);
  const attentionGroups = groups.filter((group) => group.bucket === 'attention');
  const updateGroups = groups.filter((group) => group.bucket === 'updates');
  const renderActions = (item: WorkspaceNotification) => item.actions?.length ? <div className="notification-item-actions">{item.actions.map((action) => <button type="button" key={action.label} disabled={action.disabled} onClick={() => {
    actionOpened.current = Boolean(action.opensDialog || action.movesFocus);
    setOpen(false);
    action.run();
  }}>{action.label}</button>)}</div> : null;
  const renderAttempt = (item: WorkspaceNotification) => <li key={item.id} className={`notification-attempt tone-${item.tone ?? 'info'}`}>
    <div className="notification-item-title"><strong>{item.title}</strong>{item.read !== undefined && <span className={`notification-read ${item.read ? 'is-read' : 'is-new'}`}>{item.read ? 'Read' : 'New'}</span>}</div>
    <p>{item.message}</p>
    {item.detail && <small className="notification-attempt-detail">{item.detail}</small>}
    {renderActions(item)}
  </li>;
  const renderGroup = (group: typeof groups[number]) => {
    const [latest, ...earlier] = group.items;
    if (!latest) return null;
    return <li key={group.id} className={`notification-item tone-${latest.tone ?? 'info'}`}>
      <div className="notification-item-title">
        {latest.tone === 'error' || latest.tone === 'warning' ? <CircleAlert size={16} /> : <Info size={16} />}
        <strong>{latest.title}</strong>
        {group.items.length > 1 && <span className="notification-group-count">{group.items.length} {group.countLabel}</span>}
        {group.unreadCount > 0 && <span className="notification-read is-new">{group.unreadCount} new</span>}
        {group.items.length === 1 && latest.read !== undefined && <span className={`notification-read ${latest.read ? 'is-read' : 'is-new'}`}>{latest.read ? 'Read' : 'New'}</span>}
      </div>
      <p>{latest.message}</p>
      {latest.detail && <small className="notification-attempt-detail">{latest.detail}</small>}
      {renderActions(latest)}
      {earlier.length > 0 && <details className="notification-earlier">
        <summary>Show {earlier.length} earlier {group.countLabel}</summary>
        <ul className="notification-attempt-list">{earlier.map(renderAttempt)}</ul>
      </details>}
    </li>;
  };
  const renderSection = (title: string, section: typeof groups) => section.length > 0 ? <>
    <li className="notification-section-heading"><h3>{title}</h3><span>{section.length}</span></li>
    {section.map(renderGroup)}
  </> : null;
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
        {groups.length === 0 ? availability === 'ready' && <p className="notification-empty">You’re all caught up.</p> : <ul className="notification-list">
          {renderSection('Needs attention', attentionGroups)}
          {renderSection('Recent updates', updateGroups)}
        </ul>}
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>;
}
