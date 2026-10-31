# Changes default comparison acceptance

**Source:** `b2da701` (`Highlight resolved comparison target in commit history`)
**Installed executable:** `/Users/lazytrot/Applications/Code Atlas.app/Contents/MacOS/code-atlas-desktop`
**SHA-256:** `44a87560468c7e6a8f6e4ad9bb8dcd783a16b8383776ae80f0006f4612973e2d`
**Build:** unsigned macOS Apple Silicon app and DMG; package metadata and DMG checksum verified on 31 October 2026.

## Results

- In Fontdone `main@5249b72371`, Changes opened with the saved comparison `v2.14.3 → main` and its diff already visible.
- Clearing the route's comparison fields without changing the saved preference caused the current `HEAD` comparison to appear automatically: first parent `7c48b706482b8800e97b35cccc18671893c34e9f → 5249b723719445813b8771a602ea5b96891d4450`. The comparison showed two changed files.
- The default comparison marked `5249b723719445813b8771a602ea5b96891d4450` as **Comparison target**. The commit row was visually highlighted and exposed as pressed/selected in the accessibility tree.
- The saved symbolic range `v2.14.3 → main` also marked the resolved HEAD commit as **Comparison target**. Selecting `7c48b706482b8800e97b35cccc18671893c34e9f` moved the marker to that row, compared its first parent `f3096b4e4c91fa1b8c9ff2e2c48b2dce548f2046` to the selected commit, and showed four changed files.
- Restoring `v2.14.3 → main` returned to its two-file diff. Navigating to Explorer restored the original selected `src/api.rs:710–728`, Guide, and scene 2 of 3.
- The Fontdone comparison labeled `README.md` unsupported and `doc/FILE_RETENTION_INVENTORY.tsv` oversized. It disclosed partial static relationship coverage and did not claim runtime impact.
- The empty-repository fixture displayed: “No commits yet. Create the first commit in your Git tool, then select Refresh here.” The app did not start a comparison.
- The initial-commit fixture displayed the commit as disabled and explained: “This is the initial commit. Create another commit in your Git tool, then select Refresh to compare changes.” The app did not start a comparison.

## Method and boundaries

The app was built from `b2da701` with `npm run bundle:macos`. License inventory, TypeScript/Vite production build, Rust release build, metadata verification and DMG checksum passed. `node --test tests/gitCommitNavigation.test.mjs` passed (4/4); no broad test suite was run.

Packaged behavior was inspected in the installed native app with the saved Fontdone workspace and two pre-existing temporary fixture paths: `/private/tmp/code-atlas-0vd8-empty` and `/private/tmp/code-atlas-0vd8-fixture`. The fixture repositories were initialized locally for this check; the second contains one synthetic initial commit. No Fontdone source or Git history was changed, no provider request or narration was started, and the original Fontdone project, snapshot, selected source file, Guide scene and saved comparison preference were restored afterward.

This qualifies the five acceptance cases in Kata `0vd8` on this macOS build, including target-row identification for full commit IDs and a symbolic branch ref. It does not qualify other platforms, signed/notarized release packaging, or the broader Git comparison and accessibility matrix tracked by separate issues.
