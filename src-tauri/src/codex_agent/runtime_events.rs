//! Interpret only notifications owned by the acknowledged turn. Diagnostic
//! observations copy no text, arguments, identifiers or model reasoning.

use super::*;
use crate::generation_timing::{RuntimeEventKind as Kind, TokenUsageSnapshot, Trace};

pub(super) fn observe(
    trace: &Trace,
    turn_index: u8,
    method: &str,
    params: &Value,
    thread_id: &str,
    turn_id: Option<&str>,
) -> Result<(), String> {
    let Some(turn_id) = turn_id else {
        return Ok(());
    };
    let event_turn = if matches!(method, "turn/started" | "turn/completed") {
        params.pointer("/turn/id").and_then(Value::as_str)
    } else {
        params.get("turnId").and_then(Value::as_str)
    };
    if params.get("threadId").and_then(Value::as_str) != Some(thread_id)
        || event_turn != Some(turn_id)
    {
        return Ok(());
    }
    let kind = match method {
        "turn/started" => Kind::TurnStarted,
        "turn/completed" => Kind::TurnCompleted,
        "item/started" => Kind::ItemStarted,
        "item/completed" => Kind::ItemCompleted,
        "item/agentMessage/delta" => Kind::AgentMessageDelta,
        "item/reasoning/summaryTextDelta"
        | "item/reasoning/summaryPartAdded"
        | "item/reasoning/textDelta" => Kind::ReasoningActivity,
        "thread/tokenUsage/updated" => Kind::TokenUsageUpdated,
        "error" => {
            let will_retry = params
                .get("willRetry")
                .and_then(Value::as_bool)
                .ok_or(UPGRADE)?;
            trace.runtime_event(
                turn_index,
                if will_retry {
                    Kind::RetryableError
                } else {
                    Kind::TerminalError
                },
            );
            if !will_retry {
                // This is the runtime's terminal event, not a client retry.
                // Do not wait for a possibly lost turn/completed notification.
                return Err(
                    classified_failure(params.get("error").unwrap_or(&Value::Null))
                        .unwrap_or_else(|| rpc_failure("generation", &Value::Null)),
                );
            }
            return Ok(());
        }
        _ => return Ok(()),
    };
    trace.runtime_event(turn_index, kind);
    if method == "thread/tokenUsage/updated" {
        if let Some(usage) = params
            .get("tokenUsage")
            .and_then(TokenUsageSnapshot::from_notification)
        {
            trace.reported_usage(turn_index, usage);
        }
    }
    Ok(())
}

pub(super) fn classified_failure(error: &Value) -> Option<String> {
    let info = error.get("codexErrorInfo")?;
    let message = match info.as_str() {
        Some("usageLimitExceeded" | "rateLimitExceeded" | "sessionBudgetExceeded") => {
            "Your Codex account reached a usage limit. Wait for the reset or manage your plan; no API fallback was attempted."
        }
        Some("unauthorized") => {
            "Codex account access expired or was unavailable. Reconnect your ChatGPT account."
        }
        Some("contextWindowExceeded") => {
            "Codex reached the model's context limit. No final response was accepted; saved sections were preserved."
        }
        _ if [
            "httpConnectionFailed",
            "responseStreamConnectionFailed",
            "responseStreamDisconnected",
            "responseTooManyFailedAttempts",
        ]
        .iter()
        .any(|key| info.get(key).is_some()) =>
        {
            "Codex could not maintain its model connection. The operation stopped; saved sections were preserved."
        }
        _ => return None,
    };
    Some(message.into())
}

#[cfg(test)]
mod tests;
