import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { AlertDialog, Button, Dialog, Flex, Text } from '@radix-ui/themes';
import { SavedAttempts, type SavedAttemptEntry } from './lib/savedAttempts';
import { listSavedAttempts, readSavedAttempt, trashSavedAttempt } from './lib/commands';
import { appCloseCoordinator } from './lib/closeCoordinator';
import './SavedAttemptsDialog.css';

function AttemptText({ text }: { text: string }) {
  const [expanded, setExpanded] = useState(false);
  return <><pre className="saved-attempt-text">{expanded ? text : text.slice(0, 12000)}</pre>
    {!expanded && text.length > 12000 && <Button size="1" variant="soft" onClick={() => setExpanded(true)}>Show full text</Button>}</>;
}

/** Mounted on explicit user action, so listing private output adds no startup work. */
export function SavedAttemptsDialog({ onClose, projectNames }: { onClose: () => void; projectNames: Record<string, string> }) {
  const [controller] = useState(() => new SavedAttempts({ list: listSavedAttempts, read: readSavedAttempt, trash: trashSavedAttempt }));
  const state = useSyncExternalStore(controller.subscribe, controller.getSnapshot, controller.getSnapshot);
  const [confirm, setConfirm] = useState<SavedAttemptEntry | null>(null);
  const dialog = useRef<HTMLDivElement>(null);
  const trashButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const unregister = appCloseCoordinator.register({ freeze: () => controller.beginClose(), flush: () => controller.flush() });
    void controller.refresh(); return () => { unregister(); controller.dispose(); };
  }, [controller]);
  const selected = state.inventory?.entries.find(entry => entry.id === state.selectedId);
  const label = (entry: SavedAttemptEntry) => entry.available ? projectNames[entry.projectId ?? ''] ?? 'Saved repository' : 'Unreadable saved attempt';
  return <Dialog.Root open onOpenChange={open => { if (!open && !state.removing) onClose(); }}>
    <Dialog.Content ref={dialog} maxWidth="760px" className="saved-attempts-dialog" onCloseAutoFocus={event => {
      event.preventDefault(); document.querySelector<HTMLElement>('[data-focus-key="notifications"]')?.focus();
    }}>
      <Dialog.Title>Saved attempts</Dialog.Title>
      <Dialog.Description size="2" mb="4">Review saved output from earlier generation attempts. Removing it frees space; your saved explanations stay intact.</Dialog.Description>
      <Flex justify="between" align="center" gap="3" wrap="wrap" mb="3">
        <Text size="2" color="gray">{state.inventory ? `${state.inventory.entries.length} saved items · ${state.inventory.operationLimit} attempts maximum` : 'Saved on this Mac'}</Text>
        <Button size="2" variant="soft" disabled={state.loading || state.removing} onClick={() => void controller.refresh()}>Refresh</Button>
      </Flex>
      {state.loading && <p role="status">Loading saved attempts…</p>}
      {state.error && <p role="alert" className="saved-attempts-error">{state.error}</p>}
      {state.message && <p role="status">{state.message}</p>}
      {state.inventory?.busy && <p>Finish or stop the current explanation before removing an attempt.</p>}
      {state.inventory?.limited && <p>Showing the first 32 saved items. Removing items can reveal more.</p>}
      {state.inventory?.entries.length === 0 && <p>No saved attempts.</p>}
      <ul className="saved-attempts-list" aria-label="Saved attempts">{state.inventory?.entries.map(entry => <li key={entry.id}>
        <button type="button" aria-pressed={entry.id === state.selectedId} className="saved-attempt-row"
          disabled={state.loading || state.removing || state.requiresRefresh || !entry.reviewToken} onClick={() => void controller.select(entry)}>
          <strong className="saved-attempt-question">{entry.question ?? label(entry)}</strong>
          <span>{entry.question ? `${label(entry)} · ` : ''}{entry.startedAtUnixMs ? new Date(entry.startedAtUnixMs).toLocaleString() : 'Date unavailable'} · {Math.ceil(entry.bytes / 1024)} KB{entry.available ? ` · ${entry.candidateCount} ${entry.candidateCount === 1 ? 'version' : 'versions'}` : ''}</span>
        </button>
      </li>)}</ul>
      {selected && <section className="saved-attempt-review" aria-label="Selected saved attempt" aria-busy={state.reviewing}>
        <Flex justify="between" align="center" gap="3" wrap="wrap"><Text as="p" weight="bold">{label(selected)}</Text>
          <Button ref={trashButton} color="red" variant="soft" size="2" disabled={!controller.canRemove(selected)} onClick={() => setConfirm(selected)}>Move to Trash</Button>
        </Flex>
        {!selected.available && <p>This saved output could not be verified. You can move this file to Trash to free space.</p>}
        {selected.question ? <details><summary>Original question</summary><p>{selected.question}</p></details> : <p>The original question isn’t available in saved conversation history.</p>}
        {state.reviewing && <p role="status">Opening saved text…</p>}
        {state.review?.attempts.map(attempt => <details key={`${state.review!.entry.reviewToken}:${attempt.ordinal}`}>
          <summary>Version {attempt.ordinal} · saved text</summary>
          {attempt.rejection && <p>{attempt.rejection}</p>}
          <AttemptText text={attempt.responseText} />
          {attempt.findings.length > 0 && <details><summary>Validation notes</summary><pre className="saved-attempt-text">{JSON.stringify(attempt.findings, null, 2)}</pre></details>}
        </details>)}
      </section>}
      <Flex justify="end" mt="4"><Button disabled={state.removing} onClick={onClose}>Done</Button></Flex>
      <AlertDialog.Root open={confirm !== null} onOpenChange={open => { if (!open) setConfirm(null); }}>
        <AlertDialog.Content maxWidth="440px" onCloseAutoFocus={event => {
          event.preventDefault();
          (controller.getSnapshot().removing ? dialog.current : trashButton.current)?.focus();
        }}>
          <AlertDialog.Title>Move this attempt to Trash?</AlertDialog.Title>
          <AlertDialog.Description>You can recover the file from Trash in Finder. This won’t remove a saved explanation or retry generation.</AlertDialog.Description>
          <Flex gap="3" justify="end" mt="4"><AlertDialog.Cancel><Button variant="soft" color="gray">Cancel</Button></AlertDialog.Cancel>
            <AlertDialog.Action><Button color="red" onClick={() => { const entry = confirm; setConfirm(null); if (entry) void controller.remove(entry); }}>Move to Trash</Button></AlertDialog.Action></Flex>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </Dialog.Content>
  </Dialog.Root>;
}
