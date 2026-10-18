//! Content-free observations of the owned app-server turn. Counts describe
//! received notifications, not provider processing time or evidence quality.

use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RuntimeEventKind {
    TurnStarted,
    ItemStarted,
    ItemCompleted,
    AgentMessageDelta,
    ReasoningActivity,
    TokenUsageUpdated,
    RetryableError,
    TerminalError,
    TurnCompleted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Observation {
    kind: RuntimeEventKind,
    count: u32,
    first_ms: u64,
    last_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TokenUsageBreakdown {
    pub(crate) input_tokens: u64,
    pub(crate) cached_input_tokens: u64,
    pub(crate) cache_write_input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) reasoning_output_tokens: u64,
    pub(crate) total_tokens: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TokenUsageSnapshot {
    pub(crate) last: TokenUsageBreakdown,
    pub(crate) total: TokenUsageBreakdown,
    pub(crate) model_context_window: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum UsageScope {
    AppServerThreadCumulative,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReportedUsage {
    scope: UsageScope,
    turn_index: u8,
    observed_ms: u64,
    snapshot: TokenUsageSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RuntimeTurn {
    index: u8,
    requested_ms: u64,
    observations: Vec<Observation>,
    token_usage: Option<ReportedUsage>,
}

impl TokenUsageSnapshot {
    /// Read only numeric fields from the installed v2 protocol. New upstream
    /// metadata is ignored rather than copied into diagnostics.
    pub(crate) fn from_notification(value: &serde_json::Value) -> Option<Self> {
        fn count(value: &serde_json::Value, key: &str) -> Option<u64> {
            let number = value.get(key)?.as_i64()?;
            u64::try_from(number).ok()
        }
        fn breakdown(value: &serde_json::Value) -> Option<TokenUsageBreakdown> {
            Some(TokenUsageBreakdown {
                input_tokens: count(value, "inputTokens")?,
                cached_input_tokens: count(value, "cachedInputTokens")?,
                cache_write_input_tokens: if value.get("cacheWriteInputTokens").is_none() {
                    0 // The installed protocol declares a zero default.
                } else {
                    count(value, "cacheWriteInputTokens")?
                },
                output_tokens: count(value, "outputTokens")?,
                reasoning_output_tokens: count(value, "reasoningOutputTokens")?,
                total_tokens: count(value, "totalTokens")?,
            })
        }
        let window = match value.get("modelContextWindow") {
            None | Some(serde_json::Value::Null) => None,
            Some(_) => Some(count(value, "modelContextWindow")?),
        };
        Some(Self {
            last: breakdown(value.get("last")?)?,
            total: breakdown(value.get("total")?)?,
            model_context_window: window,
        })
    }

    fn valid(&self) -> bool {
        let valid = |v: &TokenUsageBreakdown| {
            [
                v.input_tokens,
                v.cached_input_tokens,
                v.cache_write_input_tokens,
                v.output_tokens,
                v.reasoning_output_tokens,
                v.total_tokens,
            ]
            .into_iter()
            .all(|n| n <= i64::MAX as u64)
        };
        valid(&self.last)
            && valid(&self.total)
            && self
                .model_context_window
                .is_none_or(|n| n <= i64::MAX as u64)
    }
}

impl Trace {
    pub(crate) fn turn_requested(&self, index: u8) {
        self.update(|record| {
            if index as usize != record.runtime_turns.len() + 1 || index > 2 {
                return false;
            }
            record.runtime_turns.push(RuntimeTurn {
                index,
                requested_ms: elapsed(self.started),
                observations: vec![],
                token_usage: None,
            });
            true
        });
    }

    pub(crate) fn runtime_event(&self, index: u8, kind: RuntimeEventKind) {
        self.update_with_flush(true, |record| {
            let Some(turn) = record
                .runtime_turns
                .iter_mut()
                .find(|turn| turn.index == index)
            else {
                return false;
            };
            let now = elapsed(self.started);
            if let Some(event) = turn
                .observations
                .iter_mut()
                .find(|event| event.kind == kind)
            {
                if event.count == u32::MAX {
                    record.truncated = true;
                } else {
                    event.count += 1;
                }
                event.last_ms = now;
            } else {
                turn.observations.push(Observation {
                    kind,
                    count: 1,
                    first_ms: now,
                    last_ms: now,
                });
            }
            true
        });
    }

    pub(crate) fn reported_usage(&self, index: u8, snapshot: TokenUsageSnapshot) {
        if !snapshot.valid() {
            return;
        }
        self.update_with_flush(true, |record| {
            let Some(turn) = record
                .runtime_turns
                .iter_mut()
                .find(|turn| turn.index == index)
            else {
                return false;
            };
            let usage = ReportedUsage {
                scope: UsageScope::AppServerThreadCumulative,
                turn_index: index,
                observed_ms: elapsed(self.started),
                snapshot,
            };
            turn.token_usage = Some(usage.clone());
            record.token_usage = Some(usage);
            true
        });
    }
}

pub(super) fn validate(record: &Record) -> Result<(), String> {
    let invalid = || "Timing runtime observations are invalid.".to_owned();
    if record.runtime_turns.len() > 2 {
        return Err(invalid());
    }
    let end = record
        .terminal
        .as_ref()
        .map_or(MAX_ELAPSED, |end| end.elapsed_ms);
    let mut prior = record.events.first().map_or(0, |event| event.elapsed_ms);
    let mut latest_usage = None;
    for (index, turn) in record.runtime_turns.iter().enumerate() {
        if turn.index as usize != index + 1
            || turn.requested_ms < prior
            || turn.requested_ms > end
            || turn.observations.len() > 9
        {
            return Err(invalid());
        }
        prior = turn.requested_ms;
        let mut first = turn.requested_ms;
        for (i, event) in turn.observations.iter().enumerate() {
            if event.count == 0
                || event.first_ms < first
                || event.last_ms < event.first_ms
                || event.last_ms > end
                || turn.observations[..i]
                    .iter()
                    .any(|other| other.kind == event.kind)
            {
                return Err(invalid());
            }
            first = event.first_ms;
            prior = prior.max(event.last_ms);
        }
        if let Some(usage) = &turn.token_usage {
            if usage.turn_index != turn.index
                || usage.observed_ms < turn.requested_ms
                || usage.observed_ms > end
                || !usage.snapshot.valid()
                || !turn
                    .observations
                    .iter()
                    .any(|event| event.kind == RuntimeEventKind::TokenUsageUpdated)
            {
                return Err(invalid());
            }
            prior = prior.max(usage.observed_ms);
            latest_usage = Some(usage);
        }
    }
    if record.token_usage.as_ref() != latest_usage {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
