# Take a visual guide offline

Export creates one HTML file that opens without Code Atlas, an account or a
server. It includes selected **accepted** chapter revisions. Drafts and rejected
revisions stay in the app.

## Export a guide

1. Open a chapter's player and choose **Offline export**.
2. Choose the accepted revisions to include.
3. Review chapter names, source paths and cited excerpts. Turn off **Bundle
   cited source excerpts** to omit both cited quotes and source-backed values
   inside visual scenes. Scene labels, explanations, illustrations and citation
   paths remain.
4. Optionally include audio already saved on this Mac. The preview shows each
   scene's cached-audio status and matching reviewed transcript. Missing scenes
   remain readable; the exporter never synthesizes speech or calls a model.
5. Review the text and audio selections, then choose **Download HTML**.
6. Open the downloaded file in your browser. Follow **Next scene** and
   **Continue** at your own pace. Source links open the bundled citation; when
   quotes were omitted, the citation retains its path and line range.

Visual compositions preserve their groups, comparisons, cards, code, tokens,
metrics, byte strips, matrices and tables. Illustration labels remain visible.
Graphs currently use a source outline, rather than the app's interactive graph
canvas. Chapter text and narration transcripts are collapsed under their reading
controls. No executable scripts are included: audio has browser-native controls,
scene navigation is manual, and playback does not advance scenes or pause other
tracks automatically.

## What travels with the file

The package includes a schema-4 manifest with revision/snapshot identities,
asset sizes and SHA-256 hashes. A scene audio asset identifies its section,
scene and cache fingerprint; its transcript is independently hashed. Custom
whole-section scripts remain whole, and a whole-section recording is never
silently substituted for a missing scene clip. Existing schema-3 exports remain
independent HTML files and are not modified by this change.

Only the selected chapters and their cited source material are bundled. Hidden
or unsafe paths are excluded. Common credentials and local paths are filtered
from text, including visual titles and values. This cannot detect every secret,
and text filters do not change audio already recorded. Listen before sharing.
Provider configuration, the repository checkout and a restorable project backup
are not part of the export.

The HTML has a restrictive content policy and no external resource dependency.
The total file is limited to 25 MB; individual PCM WAV tracks are limited to
12 MB. Reduce the chapter selection or omit audio/excerpts if it exceeds the
limit. No source project or guide revision is changed by export.

## Qualification

Run the [synthetic fixture](../benchmarks/offline-guide/README.md) for reproducible
layout/link/hash checks. A fixture pass does not qualify browser playback,
native cache matching, human speech quality or every real accepted chapter.
See the [dated verification](acceptance/2026-10-17-offline-visual-scenes.md) and
[remaining Kata requirements](kata-tasks/22-offline-export.md).
