---
kind: task
key: t10
parent: m2
title: Add optional Gemini with exact-context consent and billing disclosure
priority: 0
labels: phase-2, provider, privacy
blocked_by: t02, t05
---

## User outcome
The user can choose whether to send one reviewed chapter request to Google and can make that choice with the exact content and current terms visible.

## Contract
The user supplies their own Gemini API key, stored only in macOS Keychain. A versioned allowlist/disclosure record identifies the requested model, current free/paid pricing page, terms URL, and verification date. The review screen shows purpose, target path/group, selected paths, exact excerpts, byte/token estimate, fingerprint, privacy terms for unpaid service, and the fact that the app cannot know whether the Cloud project has billing enabled. Consent is bound to this exact fingerprint and disclosure version.

## Acceptance criteria
- Opening settings, entering a key, building a manifest, or viewing a request review sends no model request.
- A send occurs only after separate user confirmation following exact-context review; any manifest/model/purpose/terms change invalidates confirmation.
- Current free-tier availability is presented per model, with a clear statement that Code Atlas cannot verify the user's account tier, active billing, or remaining quota.
- Unpaid-service data use and possible human review are disclosed before every send; regions/age requirements follow current terms.
- Provider error, quota, billing, network, and unavailable model states never trigger a paid fallback or automatic model switch.
- Keys and source context do not appear in logs, metadata, crash reports, or app analytics.

## Excludes
Hosted proxy, model autodiscovery, billing-account inspection, paid fallback, and sending repository contents to improve the product.

## Done evidence
Use a mocked provider transport to assert zero calls before consent and exactly one call for the reviewed fingerprint; change each fingerprint input and verify consent resets; inspect storage/logs for secrets and context.
