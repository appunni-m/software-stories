# Offline visual-story fixture

This is an explicit synthetic accepted revision for exporter qualification.
It is not an agent-generation benchmark and never changes app history.

From the repository root, with the project's existing Node runtime:

```sh
node --experimental-strip-types scripts/qualify-offline-guide.mjs /private/tmp/code-atlas-offline-guide
```

The command writes standalone HTML with and without cited source excerpts. It
checks scene markup, internal links, inert content, asset hashes and unchanged
input. Open the output in a browser to inspect layout and navigation.

An optional third argument names a directory containing `narration.json` and
`scene-1.wav` / `scene-2.wav` generated from the exact `NARRATION_SCRIPTS` in
`fixture.mjs`. This adds audio/no-audio combinations. The JSON records
`modelId`, `workerSha256`, and two `tracks`, each with `file`, `exactText` and
`sha256`. Audio is validated by the exporter. Fixture fingerprints do not claim
native cache admission; this command never synthesizes speech or installs a
model. Listening quality, browser playback, native cache matching and genuine
accepted-revision export still need their own evidence.
