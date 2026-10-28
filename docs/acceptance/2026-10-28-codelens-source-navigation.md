# Packaged Code Lens source navigation — 28 October 2026

## Environment

- App: Code Atlas 0.1.0, bundle ID `com.codeatlas.desktop`.
- Installed executable SHA-256: `b5bba1e6d760ce8a3bf1ebbead2553b697efdabf91526948c11954dc8eb56623`.
- The installed bundle does not identify its source commit. This observation is
  not attributed to the current checkout or to commit `2a58384`.
- Workspace: Fontdone, partial captured snapshot `main@5249b72371`.
- No explanation, provider request, narration synthesis, or source edit was
  started.

## Procedure and result

1. From the saved Code Lens answer for “Explain what `glyf.rs:564–583` does and
   how it works,” selected the visual's **Open `src/tt/glyf.rs`, lines 564
   through 582** action. The Explorer selected `src/tt/glyf.rs`; the source
   pane showed its captured code at Rust lines 564–582. Guide remained selected.
2. Selected `src/tt/gasp.rs` in Explorer. The source pane changed to that
   captured file while the same Guide view and saved Code Lens answer remained
   visible. The app stayed open and no error appeared.

Both actions were local source navigation. I did not click **Explain this
code**, so no source was sent to an agent.

## Limits

This qualifies one saved citation and one Explorer file selection on the
installed 0.1.0 app. The original reported click control and exception remain
unknown. It does not reproduce the selected-code generation action, verify a
request review, or identify the installed binary's source commit. No screenshot
was saved. Keep Kata `mhqx` open for the original click error and remaining
source-navigation cases.

## Selected-code review boundary — 28 October 2026

The currently installed Code Atlas app was identified by its running executable
path `/Users/lazytrot/Applications/Code Atlas.app/Contents/MacOS/code-atlas-desktop`,
version `0.1.0`, and executable SHA-256
`adec1e64e5d1783b5b531256af0c3005743af472e30c167fd4313cde8597af83`. The app
metadata does not identify its source revision. In Fontdone partial snapshot
`main@5249b72371`, Explorer opened `src/tt/glyf.rs` without an error. Opening the
saved citation selected captured lines 554–574. **Explain this code** opened a
local review for that exact path/range. The dialog stated “Nothing is sent until
you choose Ask about this code,” showed the selected source preview, and warned
that account quota applies if the request is sent. I canceled; the app returned
to the same selected source and Guide, stayed open, and the question count
remained 27. No **Ask about this code** action, provider request, source change,
or narration occurred. The original user-reported click control and exception
remain unknown; keep `mhqx` open.
