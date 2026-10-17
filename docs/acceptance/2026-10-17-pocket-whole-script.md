# Pocket ownership across a reviewed script — 17 October 2026

## Confirmed source defect

Local synthesis previously invoked a finite Python process for every reviewed
4,000-byte segment. Each process loaded the model again, and the exclusive
native lease was released between segments. Another window or setup operation
could therefore enter the runtime before the same reviewed script finished.
This contradicts the whole-section ownership requirement in `q6g3`.

A deterministic worker regression failed before the correction: the old worker
interpreted the new packet as one plain-text utterance rather than two exact
ordered segments. The retained before log has 3 tests / 11 failures, including
invalid packet admission. That log is a protocol regression, not an observed
native concurrency failure: `/private/tmp/code-atlas-pocket-section-before.log`.

## Correction

- One finite offline worker receives one strict version-1 JSON packet over
  stdin and owns the complete reviewed script. Bounds are 1–64 nonblank UTF-8
  segments, 4,000 bytes per segment and 16,000 total text bytes. JSON transport
  is independently bounded to 97,024 bytes. Duplicate/unknown fields, future
  versions, booleans in place of the version and invalid text are rejected.
- One process-wide owner and at most four waiting operations cover setup and
  generation. Queued cancellation or a dropped waiter starts no process.
  Cancellation of active work holds ownership until the child is stopped and
  reaped; exit closes the queue and stops the owned process group.
- Model and built-in voice state load once per script. Segments generate in
  original order with Pocket's explicit `copy_state=True`; a 200 ms PCM pause
  separates them. Warm reuse is within one script; there is no persistent
  model daemon or new listening port.
- Every segment must produce nonempty finite audio. Both audio and join pauses
  count toward 12 MiB. Any later failure deletes the whole temporary WAV.
  Rust independently validates complete mono PCM16 / 24 kHz WAV output before
  the narration layer can cache or play it. Previous cached files are retained.
- Nonblocking input writes now obey cancellation and the same five-minute
  deadline as generation. A worker that never consumes its pipe cannot strand
  the native owner. Partial output and dependency errors remain private.
- The embedded worker's existing runtime fingerprint changes, invalidating
  new local reviews/cache lookup without rewriting old files. Model/voice pins
  and legacy Google cache identity are unchanged. No engine controls were
  added to the answer surface; Voice remains in Connections.

## Qualification and limits

The deterministic Python contract suite passes **six tests** using the installed
NumPy/PyTorch with model loading replaced by finite synthetic chunks. It checks
one load, exact order/options, pause bytes, UTF-8/escape expansion and packet
bounds, invalid packet rejection before load, nonfinite/empty later output and
whole-output cleanup when the pause or PCM exceeds the limit. No model download
or subjective speech judgment is involved.

Native focused checks cover the packet, bounded owner/queue, queued cancellation,
dropped waiters, shutdown and child reaping after a blocked-input timeout or
cancellation. The added `fcntl` boundary preserves existing descriptor flags;
the owned live pipe descriptor cannot be closed/reused during either call.
Only integer arguments are supplied, with no foreign pointer or shared mutable
state. Synthetic child tests confirm reaping via `waitpid` / `ECHILD`.

The real pinned worker separately produced a **13.08-second**, 627,884-byte valid
WAV from two explicitly synthetic reviewed segments in **12.679 seconds**.
The output/configuration are isolated; existing interpreter/assets were reused
read-only. Offline environment and local paths were enforced, but network
syscalls were not traced. This is not clean-account setup, native cache, player,
export or human listening acceptance. [Exact worker/audio identity and scripts](artifacts/2026-10-17-pocket-whole-script-worker.json).
Audio remains at `/private/tmp/code-atlas-pocket-whole-script-qualification/active.wav`.

Run deterministic worker tests with the app-managed Python:

```sh
"$POCKET_RUNTIME/venv/bin/python" -I -m unittest discover -s src-tauri/tests -p test_pocket_tts_worker.py
```

Set `POCKET_RUNTIME` to the installed `pocket-tts/runtime-3.3.0-v1` directory in
app data; this command requires its installed NumPy/PyTorch, not model inference.
The runtime and player/cache Kata tasks remain open for clean-account packaging,
setup failure/repair, installed cross-window and exit behavior, cache/export
invalidation and real-code listening. Mac lock still prevents installed checks.

The final native suite passes **388 tests** (zero failed, 13 explicit account
probes ignored) in 35.71 seconds; **184 frontend tests** and production
TypeScript/Vite plus unsigned debug app packaging pass. Existing unused-code
and large-chunk warnings remain. Embedded Mach-O hash verification passes with
resources excluded. Strict executable verification reports a missing resource
seal; this is not Developer ID signing, bundle sealing or notarization evidence.
The replacement bundle is not installed.

Exact native/frontend/bundle identities and logs are recorded in the
[mechanical artifact](artifacts/2026-10-17-pocket-whole-script-mechanical.json).
