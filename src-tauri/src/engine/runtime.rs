use super::data::{
    normalize_credit_snapshot, normalize_rate_limits, now_epoch_seconds, CreditSnapshot,
    CreditStatus, RuntimeStatus, UsageSnapshot,
};
use super::protocol::{discover_binary, read_rate_limits, ProtocolError};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("{0}")]
    Protocol(#[from] super::protocol::ProtocolError),
    #[error("usage normalization failed: {0}")]
    Normalize(#[from] super::data::NormalizeError),
}

#[derive(Debug, Clone)]
pub struct Engine {
    timeout: Duration,
    max_attempts: u8,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(15),
            max_attempts: 2,
        }
    }
}

impl Engine {
    pub fn read_with_recovery(
        &self,
        previous: Option<UsageSnapshot>,
    ) -> Result<UsageSnapshot, EngineError> {
        self.read_usage_and_credits_with_recovery(previous, None)
            .map(|(usage, _credits)| usage)
    }

    pub fn read_usage_and_credits_with_recovery(
        &self,
        previous_usage: Option<UsageSnapshot>,
        previous_credits: Option<CreditSnapshot>,
    ) -> Result<(UsageSnapshot, CreditSnapshot), EngineError> {
        let mut last_error = None;
        for _attempt in 0..self.max_attempts {
            match self.read_once() {
                Ok(snapshot) => return Ok((snapshot.usage, snapshot.credits)),
                Err(error) => last_error = Some(error),
            }
        }
        let error = last_error.expect("at least one bounded attempt");
        let usage = if let Some(mut stale) = previous_usage {
            stale.status = RuntimeStatus::Stale;
            stale.diagnostic_code = Some("bounded_recovery_exhausted".to_string());
            stale
        } else {
            self.failure_snapshot(&error)
        };
        let credits = recover_credit_snapshot(previous_credits, &error);
        Ok((usage, credits))
    }

    fn failure_snapshot(&self, error: &EngineError) -> UsageSnapshot {
        let (status, diagnostic_code) = match error {
            EngineError::Protocol(ProtocolError::BinaryNotFound)
            | EngineError::Protocol(ProtocolError::Unsupported(_))
            | EngineError::Protocol(ProtocolError::Remote(_)) => {
                (RuntimeStatus::Unavailable, diagnostic_code(error))
            }
            _ => (RuntimeStatus::Error, diagnostic_code(error)),
        };
        UsageSnapshot {
            windows: Vec::new(),
            status,
            fetched_at: None,
            last_successful_at: None,
            source: "codex-owned-local-app-server".to_string(),
            capability: "account/rateLimits/read".to_string(),
            diagnostic_code: Some(diagnostic_code),
        }
    }

    fn read_once(&self) -> Result<ReadSnapshot, EngineError> {
        let binary = discover_binary()?;
        let response = read_rate_limits(&binary, self.timeout)?;
        let windows = normalize_rate_limits(&response)?;
        let now = now_epoch_seconds();
        Ok(ReadSnapshot {
            usage: UsageSnapshot {
                status: status_for_windows(windows.len()),
                windows,
                fetched_at: Some(now),
                last_successful_at: Some(now),
                source: "codex-owned-local-app-server".to_string(),
                capability: "account/rateLimits/read".to_string(),
                diagnostic_code: None,
            },
            credits: normalize_credit_snapshot(&response, now),
        })
    }
}

#[derive(Debug)]
struct ReadSnapshot {
    usage: UsageSnapshot,
    credits: CreditSnapshot,
}

fn recover_credit_snapshot(
    previous: Option<CreditSnapshot>,
    error: &EngineError,
) -> CreditSnapshot {
    if let Some(mut stale) = previous.filter(|snapshot| snapshot.last_successful_at.is_some()) {
        stale.status = CreditStatus::Stale;
        stale.diagnostic_code = Some("bounded_recovery_exhausted".to_string());
        stale
    } else {
        credit_failure_snapshot(error)
    }
}

fn credit_failure_snapshot(error: &EngineError) -> CreditSnapshot {
    let status = match error {
        EngineError::Protocol(ProtocolError::BinaryNotFound)
        | EngineError::Protocol(ProtocolError::Unsupported(_))
        | EngineError::Protocol(ProtocolError::Remote(_)) => CreditStatus::Unavailable,
        _ => CreditStatus::Error,
    };
    CreditSnapshot {
        has_credits: false,
        unlimited: false,
        balance: None,
        status,
        fetched_at: None,
        last_successful_at: None,
        diagnostic_code: Some(credit_diagnostic_code(error)),
    }
}

fn status_for_windows(window_count: usize) -> RuntimeStatus {
    if window_count < 2 {
        RuntimeStatus::Partial
    } else {
        RuntimeStatus::Fresh
    }
}

fn diagnostic_code(error: &EngineError) -> String {
    match error {
        EngineError::Protocol(ProtocolError::BinaryNotFound) => {
            "codex_binary_not_found".to_string()
        }
        EngineError::Protocol(ProtocolError::Unsupported(_)) => {
            "rate_limits_capability_unsupported".to_string()
        }
        EngineError::Protocol(ProtocolError::Remote(code)) => code.clone(),
        EngineError::Protocol(ProtocolError::Timeout(_)) => "app_server_timeout".to_string(),
        EngineError::Protocol(ProtocolError::ProcessExited) => {
            "app_server_process_exited".to_string()
        }
        EngineError::Protocol(ProtocolError::Spawn(_)) => "app_server_spawn_failed".to_string(),
        EngineError::Protocol(ProtocolError::MalformedJson) => {
            "app_server_malformed_json".to_string()
        }
        EngineError::Normalize(_) => "usage_response_invalid".to_string(),
    }
}

fn credit_diagnostic_code(error: &EngineError) -> String {
    match error {
        EngineError::Protocol(ProtocolError::BinaryNotFound) => {
            "codex_binary_not_found".to_string()
        }
        EngineError::Protocol(ProtocolError::Unsupported(_)) => {
            "rate_limits_capability_unsupported".to_string()
        }
        EngineError::Protocol(ProtocolError::Remote(code)) => code.clone(),
        EngineError::Protocol(ProtocolError::Timeout(_)) => "app_server_timeout".to_string(),
        EngineError::Protocol(ProtocolError::ProcessExited) => {
            "app_server_process_exited".to_string()
        }
        EngineError::Protocol(ProtocolError::Spawn(_)) => "app_server_spawn_failed".to_string(),
        EngineError::Protocol(ProtocolError::MalformedJson) => {
            "app_server_malformed_json".to_string()
        }
        EngineError::Normalize(_) => "usage_response_invalid".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::data::UsageWindow;

    fn prior_snapshot() -> UsageSnapshot {
        UsageSnapshot {
            windows: vec![UsageWindow {
                key: "codex:10080".into(),
                limit_id: "codex".into(),
                window_duration_mins: 10080,
                used_percent: 18,
                remaining_percent: 82,
                resets_at: Some("1787198350".into()),
                source_slot: "primary".into(),
            }],
            status: RuntimeStatus::Fresh,
            fetched_at: Some(1),
            last_successful_at: Some(1),
            source: "test".into(),
            capability: "account/rateLimits/read".into(),
            diagnostic_code: None,
        }
    }

    #[test]
    fn bounded_failure_marks_last_good_data_stale() {
        let engine = Engine {
            timeout: Duration::from_millis(1),
            max_attempts: 1,
        };
        let result = engine
            .read_with_recovery(Some(prior_snapshot()))
            .expect("stale fallback");
        assert_eq!(result.status, RuntimeStatus::Stale);
        assert_eq!(
            result.diagnostic_code.as_deref(),
            Some("bounded_recovery_exhausted")
        );
    }

    #[test]
    fn valid_window_count_distinguishes_fresh_and_partial() {
        assert_eq!(status_for_windows(1), RuntimeStatus::Partial);
        assert_eq!(status_for_windows(2), RuntimeStatus::Fresh);
    }

    #[test]
    fn unsupported_capability_is_unavailable() {
        let snapshot = Engine::default().failure_snapshot(&EngineError::Protocol(
            ProtocolError::Unsupported("account/rateLimits/read".into()),
        ));
        assert_eq!(snapshot.status, RuntimeStatus::Unavailable);
        assert_eq!(
            snapshot.diagnostic_code.as_deref(),
            Some("rate_limits_capability_unsupported")
        );
    }

    #[test]
    fn available_credit_snapshot_becomes_stale_after_bounded_failure() {
        let previous = CreditSnapshot {
            has_credits: true,
            unlimited: false,
            balance: Some("841.00".into()),
            status: CreditStatus::Available,
            fetched_at: Some(1),
            last_successful_at: Some(1),
            diagnostic_code: None,
        };
        let result = recover_credit_snapshot(
            Some(previous),
            &EngineError::Protocol(ProtocolError::Timeout(1)),
        );
        assert_eq!(result.status, CreditStatus::Stale);
        assert_eq!(result.balance.as_deref(), Some("841.00"));
        assert_eq!(
            result.diagnostic_code.as_deref(),
            Some("bounded_recovery_exhausted")
        );
    }

    #[test]
    fn first_credit_failure_without_prior_value_is_error_or_unavailable() {
        let result =
            recover_credit_snapshot(None, &EngineError::Protocol(ProtocolError::Timeout(1)));
        assert_eq!(result.status, CreditStatus::Error);
        assert_eq!(result.balance, None);
    }

    #[test]
    fn timeout_is_error_and_states_have_distinct_wire_values() {
        let snapshot =
            Engine::default().failure_snapshot(&EngineError::Protocol(ProtocolError::Timeout(1)));
        assert_eq!(snapshot.status, RuntimeStatus::Error);
        let values = [
            RuntimeStatus::Loading,
            RuntimeStatus::Fresh,
            RuntimeStatus::Stale,
            RuntimeStatus::Partial,
            RuntimeStatus::Unavailable,
            RuntimeStatus::Error,
        ]
        .into_iter()
        .map(|status| serde_json::to_string(&status).expect("status serialization"))
        .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(values.len(), 6);
    }
}
