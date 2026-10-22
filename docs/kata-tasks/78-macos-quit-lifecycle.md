---
kind: task
key: macos-quit-lifecycle
title: Ensure macOS Quit exits the packaged app process
priority: 2
labels: macos, lifecycle
---

## User outcome

Quitting Code Atlas closes the app process and its WebKit children. Relaunching the installed app opens the current bundle and restores the last saved repository and exploration.

## Required behavior

- Wire both the native **Code Atlas > Quit Code Atlas** menu item and Command-Q to the supported app exit path.
- Persist already-saved workspace state and stop app-owned work using the existing cancellation and recovery behavior.
- Keep ordinary window close behavior distinct when the product intends to keep the app alive.
- Launch only one active app process/window when reopening the installed bundle.

## Completion evidence recorded in Kata

Kata `cc7g` closed on 22 October 2026 after packaged macOS acceptance. The installed app was exercised on Fontdone `main@5249b72371`; the rebuilt bundle executable SHA-256 was `9940e7ff66832c4aa051ca3a81e84e1929dc77847a94a9af16290815964bc156`. The old process that ignored both quit paths was terminated while idle after preserving its app-support directory. The rebuilt bundle restored the saved repository, 33-question exploration, source selection, Flow position and Codex connection. Three subsequent launch/quit cycles passed: native menu Quit once and Command-Q twice terminated the process, and a later process scan found no Code Atlas or WebKit WebContent process. No generation was run.

This qualifies the packaged quit/relaunch lifecycle only; it does not establish visual, provider, generation, or other release gates.
