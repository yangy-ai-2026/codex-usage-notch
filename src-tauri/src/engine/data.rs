use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    Loading,
    Fresh,
    Stale,
    Partial,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreditStatus {
    Loading,
    Available,
    Unlimited,
    Unavailable,
    Stale,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreditSnapshot {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
    pub status: CreditStatus,
    pub fetched_at: Option<u64>,
    pub last_successful_at: Option<u64>,
    pub diagnostic_code: Option<String>,
}

impl CreditSnapshot {
    pub fn loading() -> Self {
        Self {
            has_credits: false,
            unlimited: false,
            balance: None,
            status: CreditStatus::Loading,
            fetched_at: None,
            last_successful_at: None,
            diagnostic_code: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub key: String,
    pub limit_id: String,
    pub window_duration_mins: u64,
    pub used_percent: u8,
    pub remaining_percent: u8,
    pub resets_at: Option<String>,
    pub source_slot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub windows: Vec<UsageWindow>,
    pub status: RuntimeStatus,
    pub fetched_at: Option<u64>,
    pub last_successful_at: Option<u64>,
    pub source: String,
    pub capability: String,
    pub diagnostic_code: Option<String>,
}

pub fn normalize_credit_snapshot(response: &Value, fetched_at: u64) -> CreditSnapshot {
    let credits = response
        .get("result")
        .and_then(|result| result.get("rateLimits"))
        .or_else(|| response.get("rateLimits"))
        .and_then(|rate_limits| rate_limits.get("credits"))
        .filter(|value| !value.is_null());

    let has_credits = credits
        .and_then(|value| value.get("hasCredits"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let unlimited = credits
        .and_then(|value| value.get("unlimited"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let balance = credits
        .and_then(|value| value.get("balance"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let status = if unlimited {
        CreditStatus::Unlimited
    } else if has_credits && balance.is_some() {
        CreditStatus::Available
    } else {
        CreditStatus::Unavailable
    };
    let last_successful_at = match status {
        CreditStatus::Available | CreditStatus::Unlimited => Some(fetched_at),
        _ => None,
    };

    CreditSnapshot {
        has_credits,
        unlimited,
        balance,
        status,
        fetched_at: Some(fetched_at),
        last_successful_at,
        diagnostic_code: None,
    }
}

#[derive(Debug, Error)]
pub enum NormalizeError {
    #[error("rate limit response did not contain a rateLimits object")]
    MissingRateLimits,
    #[error("rate limit window {slot} is missing limitId")]
    MissingLimitId { slot: String },
    #[error("rate limit window {slot} is missing windowDurationMins")]
    MissingDuration { slot: String },
    #[error("rate limit window {slot} is missing usedPercent")]
    MissingUsedPercent { slot: String },
    #[error("rate limit window {slot} has invalid usedPercent")]
    InvalidUsedPercent { slot: String },
}

pub fn normalize_rate_limits(response: &Value) -> Result<Vec<UsageWindow>, NormalizeError> {
    let limits = response
        .get("result")
        .and_then(|result| result.get("rateLimits"))
        .or_else(|| response.get("rateLimits"))
        .ok_or(NormalizeError::MissingRateLimits)?;
    let limit_id = limits
        .get("limitId")
        .and_then(Value::as_str)
        .unwrap_or("codex")
        .to_string();
    let mut windows = BTreeMap::new();

    for slot in ["primary", "secondary"] {
        if let Some(window) = limits.get(slot).filter(|value| !value.is_null()) {
            let normalized = normalize_window(window, &limit_id, slot)?;
            windows.insert(normalized.key.clone(), normalized);
        }
    }

    if windows.is_empty() {
        if let Some(by_id) = response
            .get("result")
            .and_then(|result| result.get("rateLimitsByLimitId"))
            .or_else(|| response.get("rateLimitsByLimitId"))
        {
            if let Some(object) = by_id.as_object() {
                for (id, value) in object {
                    for slot in ["primary", "secondary"] {
                        if let Some(window) = value.get(slot).filter(|item| !item.is_null()) {
                            let normalized = normalize_window(window, id, slot)?;
                            windows.insert(normalized.key.clone(), normalized);
                        }
                    }
                }
            }
        }
    }

    Ok(windows.into_values().collect())
}

fn normalize_window(
    window: &Value,
    limit_id: &str,
    slot: &str,
) -> Result<UsageWindow, NormalizeError> {
    let duration = window
        .get("windowDurationMins")
        .and_then(Value::as_u64)
        .ok_or_else(|| NormalizeError::MissingDuration {
            slot: slot.to_string(),
        })?;
    let used = window
        .get("usedPercent")
        .and_then(Value::as_f64)
        .ok_or_else(|| NormalizeError::MissingUsedPercent {
            slot: slot.to_string(),
        })?;
    if !used.is_finite() {
        return Err(NormalizeError::InvalidUsedPercent {
            slot: slot.to_string(),
        });
    }
    let used_percent = used.round().clamp(0.0, 100.0) as u8;
    let reset = window
        .get("resetsAt")
        .and_then(Value::as_i64)
        .map(|seconds| seconds.to_string());
    Ok(UsageWindow {
        key: format!("{limit_id}:{duration}"),
        limit_id: limit_id.to_string(),
        window_duration_mins: duration,
        used_percent,
        remaining_percent: 100 - used_percent,
        resets_at: reset,
        source_slot: slot.to_string(),
    })
}

pub fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn credits_response(credits: Value) -> Value {
        json!({"rateLimits": {"credits": credits}})
    }

    #[test]
    fn normalizes_single_seven_day_window_without_inventing_secondary() {
        let response = json!({"rateLimits": {"limitId": "codex", "primary": {"usedPercent": 18, "windowDurationMins": 10080, "resetsAt": 1787198350}, "secondary": null}});
        let windows = normalize_rate_limits(&response).expect("valid response");
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].key, "codex:10080");
        assert_eq!(windows[0].remaining_percent, 82);
        assert_eq!(windows[0].source_slot, "primary");
    }

    #[test]
    fn normalizes_multiple_protocol_slots_by_duration() {
        let response = json!({"rateLimits": {"limitId": "codex", "primary": {"usedPercent": 101, "windowDurationMins": 300}, "secondary": {"usedPercent": -4, "windowDurationMins": 10080}}});
        let windows = normalize_rate_limits(&response).expect("valid response");
        assert_eq!(windows.len(), 2);
        let by_duration = windows
            .into_iter()
            .map(|window| (window.window_duration_mins, window.remaining_percent))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(by_duration.get(&300), Some(&0));
        assert_eq!(by_duration.get(&10080), Some(&100));
    }

    #[test]
    fn preserves_missing_reset_and_ignores_unknown_fields() {
        let response = json!({"rateLimits": {"limitId": "codex", "primary": {"usedPercent": 50, "windowDurationMins": 60, "futureField": "ignored"}}});
        let windows = normalize_rate_limits(&response).expect("valid response");
        assert_eq!(windows[0].resets_at, None);
    }

    #[test]
    fn normalizes_available_credit_balance_without_parsing_it() {
        let snapshot = normalize_credit_snapshot(
            &credits_response(json!({
                "hasCredits": true,
                "unlimited": false,
                "balance": "841.00"
            })),
            42,
        );
        assert_eq!(snapshot.status, CreditStatus::Available);
        assert_eq!(snapshot.balance.as_deref(), Some("841.00"));
        assert_eq!(snapshot.last_successful_at, Some(42));
    }

    #[test]
    fn normalizes_unlimited_credits_as_a_distinct_state() {
        let snapshot = normalize_credit_snapshot(
            &credits_response(json!({
                "hasCredits": true,
                "unlimited": true,
                "balance": null
            })),
            42,
        );
        assert_eq!(snapshot.status, CreditStatus::Unlimited);
        assert!(snapshot.unlimited);
        assert_eq!(snapshot.balance, None);
    }

    #[test]
    fn missing_or_null_credit_balance_is_unavailable_not_zero() {
        let missing = normalize_credit_snapshot(&json!({"rateLimits": {}}), 42);
        let null_balance = normalize_credit_snapshot(
            &credits_response(json!({
                "hasCredits": true,
                "unlimited": false,
                "balance": null
            })),
            42,
        );
        assert_eq!(missing.status, CreditStatus::Unavailable);
        assert_eq!(missing.balance, None);
        assert_eq!(null_balance.status, CreditStatus::Unavailable);
        assert_eq!(null_balance.balance, None);
    }

    #[test]
    fn purchased_balance_is_separate_from_rate_limit_reset_credits() {
        let response = json!({
            "rateLimits": {
                "credits": {"hasCredits": true, "unlimited": false, "balance": "841"}
            },
            "rateLimitResetCredits": {"availableCount": 3}
        });
        let snapshot = normalize_credit_snapshot(&response, 42);
        assert_eq!(snapshot.balance.as_deref(), Some("841"));
        assert_ne!(snapshot.balance.as_deref(), Some("3"));
    }
}
