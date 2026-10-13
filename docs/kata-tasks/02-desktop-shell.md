---
kind: task
key: t02
parent: m0
title: Build the shared Mac shell and typed application boundary
priority: 1
labels: phase-0, foundation, design
blocked_by: t01
---

## User outcome
The app opens to a calm, coherent workspace and lets the user move between the three core views without needing a repository or account.

## Contract
Build a Tauri 2 macOS window with React/TypeScript UI and a Rust command layer. The shell contains project/snapshot selection, primary Atlas / Code Lens / Walkthrough navigation, and entry points to Guide Plan and Chapter Studio. It starts in a designed no-project state. Tauri commands are explicit and typed; the WebView has no broad filesystem, shell, Git, or network capability.

## Acceptance criteria
- Every primary view opens and renders its defined empty state with a stable selected-project/snapshot state model.
- Navigation preserves view selection and supports back/forward without duplicate or lost history.
- On a fresh no-project launch, keyboard focus lands on the primary Open repository action; workspace navigation and return navigation focus their intended destination or restored control.
- Opening the shell causes no repository scan, API-key lookup, Gemini request, or TTS request.
- Window resize, app relaunch, and command failure produce defined UI states rather than a blank screen.
- Keyboard navigation can reach primary navigation and return focus to the originating control.

## Excludes
Repository parsing, generation, AI setup, audio, and release signing.

## Done evidence
Fresh-install run-through shows all five workflow surfaces, no-project and failure states, keyboard navigation, and zero provider/network calls before user action.
