//! Catalog domain types and parsing of the OpenCode mirror payload.
//!
//! The mirror is a JSON object keyed by provider. Only the `opencode-go`
//! provider contributes to the Roster ([`ROSTER_PROVIDER_ID`]).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Provider key of the models that make up the Roster.
pub const ROSTER_PROVIDER_ID: &str = "opencode-go";

/// A model price pair in USD per million tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Price {
    pub input: f64,
    pub output: f64,
}

/// One Roster model, normalized to the snapshot schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub price: Price,
    pub cache_read: f64,
    pub context: u64,
    pub tool_call: bool,
    pub attachment: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
}

/// Persisted catalog snapshot, both bundled and synced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogSnapshot {
    pub schema: u32,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    pub fetched_at: String,
    pub models: Vec<ModelEntry>,
}

/// Result of parsing a mirror payload.
#[derive(Debug, Clone, PartialEq)]
pub struct MirrorParse {
    /// Models that passed validation, in model-id order.
    pub models: Vec<ModelEntry>,
    /// Entries dropped because required fields were missing.
    pub skipped: Vec<SkippedEntry>,
}

/// A mirror entry dropped during parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedEntry {
    /// Model id, or the mirror key when the entry has no usable `id`.
    pub id: String,
    /// Required field names that were absent or had the wrong type.
    pub missing: Vec<String>,
}

impl SkippedEntry {
    /// Human-readable warning text for sync reporting.
    pub fn reason(&self) -> String {
        format!("missing required fields: {}", self.missing.join(", "))
    }
}

/// Catalog parsing errors.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("invalid catalog JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid catalog payload: {0}")]
    InvalidPayload(String),
}

/// Extract the `opencode-go` Roster from a mirror payload.
///
/// Entries marked `status = "deprecated"` are excluded. Entries missing any
/// required field are reported in [`MirrorParse::skipped`] and do not abort the
/// parse. A payload without the `opencode-go` provider yields an empty roster;
/// the sync layer treats an empty roster as an invalid payload.
pub fn parse_mirror(json: &str) -> Result<MirrorParse, CatalogError> {
    let root: Value = serde_json::from_str(json)?;
    let Value::Object(providers) = root else {
        return Err(CatalogError::InvalidPayload(
            "mirror root must be a JSON object".to_string(),
        ));
    };

    let mut models = Vec::new();
    let mut skipped = Vec::new();

    let Some(entries) = providers
        .get(ROSTER_PROVIDER_ID)
        .and_then(|provider| provider.get("models"))
        .and_then(Value::as_object)
    else {
        return Ok(MirrorParse { models, skipped });
    };

    for (key, entry) in entries {
        if entry.get("status").and_then(Value::as_str) == Some("deprecated") {
            continue;
        }

        let mut missing = Vec::new();

        let id = required(entry, "/id", &mut missing, |value| {
            value
                .as_str()
                .filter(|id| !id.trim().is_empty())
                .map(str::to_owned)
        });
        let name = required(entry, "/name", &mut missing, |value| {
            value
                .as_str()
                .filter(|name| !name.trim().is_empty())
                .map(str::to_owned)
        });
        let input = required(entry, "/cost/input", &mut missing, Value::as_f64);
        let output = required(entry, "/cost/output", &mut missing, Value::as_f64);
        let cache_read = required(entry, "/cost/cache_read", &mut missing, Value::as_f64);
        let context = required(entry, "/limit/context", &mut missing, Value::as_u64);
        let tool_call = required(entry, "/tool_call", &mut missing, Value::as_bool);
        let attachment = required(entry, "/attachment", &mut missing, Value::as_bool);

        if !missing.is_empty() {
            skipped.push(SkippedEntry {
                id: id.unwrap_or_else(|| key.clone()),
                missing,
            });
            continue;
        }

        models.push(ModelEntry {
            id: id.expect("checked above"),
            provider_id: ROSTER_PROVIDER_ID.to_string(),
            name: name.expect("checked above"),
            price: Price {
                input: input.expect("checked above"),
                output: output.expect("checked above"),
            },
            cache_read: cache_read.expect("checked above"),
            context: context.expect("checked above"),
            tool_call: tool_call.expect("checked above"),
            attachment: attachment.expect("checked above"),
            reasoning: entry
                .get("reasoning")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            release_date: entry
                .get("release_date")
                .and_then(Value::as_str)
                .map(str::to_owned),
        });
    }

    Ok(MirrorParse { models, skipped })
}

/// Deserialize a stored or bundled snapshot.
pub fn parse_snapshot(json: &str) -> Result<CatalogSnapshot, CatalogError> {
    Ok(serde_json::from_str(json)?)
}

fn required<T>(
    entry: &Value,
    pointer: &str,
    missing: &mut Vec<String>,
    parse: impl Fn(&Value) -> Option<T>,
) -> Option<T> {
    let field_name = pointer.trim_start_matches('/').replace('/', ".");
    let parsed = entry
        .pointer(pointer)
        .filter(|value| !value.is_null())
        .and_then(parse);
    if parsed.is_none() {
        missing.push(field_name);
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MIRROR: &str = r#"
    {
      "opencode-go": {
        "id": "opencode-go",
        "models": {
          "deepseek-v4.1-flash": {
            "id": "deepseek-v4.1-flash",
            "name": "DeepSeek V4.1 Flash",
            "cost": { "input": 0.15, "output": 0.6, "cache_read": 0.003 },
            "limit": { "context": 1000000 },
            "tool_call": true,
            "attachment": true,
            "reasoning": true,
            "release_date": "2026-09-10"
          }
        }
      },
      "other-provider": {
        "models": {
          "not-in-roster": {
            "id": "not-in-roster",
            "name": "Not In Roster",
            "cost": { "input": 9.0, "output": 9.0, "cache_read": 9.0 },
            "limit": { "context": 1 },
            "tool_call": true,
            "attachment": true
          }
        }
      }
    }
    "#;

    #[test]
    fn parses_roster_models_only() {
        let parsed = parse_mirror(VALID_MIRROR).expect("valid payload");

        assert_eq!(parsed.models.len(), 1);
        assert!(parsed.skipped.is_empty());

        let model = &parsed.models[0];
        assert_eq!(model.id, "deepseek-v4.1-flash");
        assert_eq!(model.provider_id, "opencode-go");
        assert_eq!(model.name, "DeepSeek V4.1 Flash");
        assert_eq!(model.price.input, 0.15);
        assert_eq!(model.price.output, 0.6);
        assert_eq!(model.cache_read, 0.003);
        assert_eq!(model.context, 1_000_000);
        assert!(model.tool_call);
        assert!(model.attachment);
        assert!(model.reasoning);
        assert_eq!(model.release_date.as_deref(), Some("2026-09-10"));
    }

    #[test]
    fn skips_entries_missing_required_fields() {
        let json = r#"
        {
          "opencode-go": {
            "models": {
              "broken": {
                "id": "broken",
                "cost": { "output": 1.2 },
                "attachment": false
              },
              "healthy": {
                "id": "healthy",
                "name": "Healthy",
                "cost": { "input": 0.1, "output": 0.2, "cache_read": 0.01 },
                "limit": { "context": 128000 },
                "tool_call": true,
                "attachment": false
              }
            }
          }
        }
        "#;

        let parsed = parse_mirror(json).expect("valid payload");

        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.models[0].id, "healthy");

        assert_eq!(parsed.skipped.len(), 1);
        let skipped = &parsed.skipped[0];
        assert_eq!(skipped.id, "broken");
        assert_eq!(
            skipped.missing,
            vec![
                "name".to_string(),
                "cost.input".to_string(),
                "cost.cache_read".to_string(),
                "limit.context".to_string(),
                "tool_call".to_string(),
            ]
        );
        assert_eq!(
            skipped.reason(),
            "missing required fields: name, cost.input, cost.cache_read, limit.context, tool_call"
        );
    }

    #[test]
    fn excludes_deprecated_entries() {
        let json = r#"
        {
          "opencode-go": {
            "models": {
              "active": {
                "id": "active",
                "name": "Active",
                "cost": { "input": 0.1, "output": 0.2, "cache_read": 0.01 },
                "limit": { "context": 128000 },
                "tool_call": true,
                "attachment": false
              },
              "retired": {
                "id": "retired",
                "name": "Retired",
                "status": "deprecated",
                "cost": { "input": 0.1, "output": 0.2, "cache_read": 0.01 },
                "limit": { "context": 128000 },
                "tool_call": true,
                "attachment": false
              }
            }
          }
        }
        "#;

        let parsed = parse_mirror(json).expect("valid payload");

        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.models[0].id, "active");
        assert!(parsed.skipped.is_empty());
    }

    #[test]
    fn round_trips_snapshot_json() {
        let snapshot = CatalogSnapshot {
            schema: 1,
            source: "https://models.opencode.ai/api.json".to_string(),
            etag: Some("\"abc123\"".to_string()),
            fetched_at: "2026-10-23T22:38:03Z".to_string(),
            models: vec![
                ModelEntry {
                    id: "deepseek-v4.1-flash".to_string(),
                    provider_id: "opencode-go".to_string(),
                    name: "DeepSeek V4.1 Flash".to_string(),
                    price: Price {
                        input: 0.15,
                        output: 0.6,
                    },
                    cache_read: 0.003,
                    context: 1_000_000,
                    tool_call: true,
                    attachment: true,
                    reasoning: true,
                    release_date: Some("2026-09-10".to_string()),
                },
                ModelEntry {
                    id: "no-release-date".to_string(),
                    provider_id: "opencode-go".to_string(),
                    name: "No Release Date".to_string(),
                    price: Price {
                        input: 0.0,
                        output: 0.0,
                    },
                    cache_read: 0.0,
                    context: 1_000,
                    tool_call: false,
                    attachment: false,
                    reasoning: false,
                    release_date: None,
                },
            ],
        };

        let json = serde_json::to_string(&snapshot).expect("serialize snapshot");
        assert!(json.contains("\"provider_id\":\"opencode-go\""));
        assert!(json.contains("\"price\":{\"input\":0.15,\"output\":0.6}"));

        let parsed = parse_snapshot(&json).expect("parse snapshot");
        assert_eq!(parsed, snapshot);
    }
}
