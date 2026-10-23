//! Diff two Catalog snapshots.
//!
//! Any difference sets the v0.1 badge; severity classification is deferred
//! (ADR-0002). Changes are reported in model-id order (SYNC-06).

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{CatalogSnapshot, ModelEntry};

/// Numeric price fields compared per model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceField {
    Input,
    Output,
    CacheRead,
}

/// Boolean capability fields compared per model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityFlag {
    ToolCall,
    Attachment,
    Reasoning,
}

/// One difference between two snapshots.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogChange {
    Added {
        id: String,
    },
    Removed {
        id: String,
    },
    PriceChanged {
        id: String,
        field: PriceField,
        old: f64,
        new: f64,
    },
    CapabilityChanged {
        id: String,
        flag: CapabilityFlag,
        old: bool,
        new: bool,
    },
}

/// List additions, removals, price changes and capability changes (SYNC-06).
pub fn detect_changes(previous: &CatalogSnapshot, current: &CatalogSnapshot) -> Vec<CatalogChange> {
    let previous: BTreeMap<&str, &ModelEntry> = previous
        .models
        .iter()
        .map(|model| (model.id.as_str(), model))
        .collect();
    let current: BTreeMap<&str, &ModelEntry> = current
        .models
        .iter()
        .map(|model| (model.id.as_str(), model))
        .collect();

    let ids: BTreeSet<&str> = previous.keys().chain(current.keys()).copied().collect();

    let mut changes = Vec::new();

    for id in ids {
        match (previous.get(id), current.get(id)) {
            (None, Some(_)) => changes.push(CatalogChange::Added { id: id.to_string() }),
            (Some(_), None) => changes.push(CatalogChange::Removed { id: id.to_string() }),
            (Some(before), Some(after)) => {
                push_price_changes(&mut changes, id, before, after);
                push_capability_changes(&mut changes, id, before, after);
            }
            (None, None) => unreachable!("id came from one of the snapshots"),
        }
    }

    changes
}

fn push_price_changes(
    changes: &mut Vec<CatalogChange>,
    id: &str,
    before: &ModelEntry,
    after: &ModelEntry,
) {
    let pairs = [
        (PriceField::Input, before.price.input, after.price.input),
        (PriceField::Output, before.price.output, after.price.output),
        (PriceField::CacheRead, before.cache_read, after.cache_read),
    ];

    for (field, old, new) in pairs {
        if old != new {
            changes.push(CatalogChange::PriceChanged {
                id: id.to_string(),
                field,
                old,
                new,
            });
        }
    }
}

fn push_capability_changes(
    changes: &mut Vec<CatalogChange>,
    id: &str,
    before: &ModelEntry,
    after: &ModelEntry,
) {
    let pairs = [
        (CapabilityFlag::ToolCall, before.tool_call, after.tool_call),
        (
            CapabilityFlag::Attachment,
            before.attachment,
            after.attachment,
        ),
        (CapabilityFlag::Reasoning, before.reasoning, after.reasoning),
    ];

    for (flag, old, new) in pairs {
        if old != new {
            changes.push(CatalogChange::CapabilityChanged {
                id: id.to_string(),
                flag,
                old,
                new,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Price;

    fn model(id: &str) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            provider_id: "opencode-go".to_string(),
            name: id.to_string(),
            price: Price {
                input: 0.15,
                output: 0.6,
            },
            cache_read: 0.003,
            context: 1_048_576,
            tool_call: true,
            attachment: true,
            reasoning: true,
            release_date: None,
        }
    }

    fn snapshot(models: Vec<ModelEntry>) -> CatalogSnapshot {
        CatalogSnapshot {
            schema: 1,
            source: "https://models.opencode.ai/api.json".to_string(),
            etag: None,
            fetched_at: "2026-10-23T00:00:00Z".to_string(),
            models,
        }
    }

    #[test]
    fn detects_added_models() {
        let previous = snapshot(vec![model("model-a")]);
        let current = snapshot(vec![model("model-a"), model("model-b")]);

        assert_eq!(
            detect_changes(&previous, &current),
            vec![CatalogChange::Added {
                id: "model-b".to_string()
            }]
        );
    }

    #[test]
    fn detects_removed_models() {
        let previous = snapshot(vec![model("model-a"), model("model-b")]);
        let current = snapshot(vec![model("model-a")]);

        assert_eq!(
            detect_changes(&previous, &current),
            vec![CatalogChange::Removed {
                id: "model-b".to_string()
            }]
        );
    }

    #[test]
    fn detects_price_change_per_field() {
        let previous = snapshot(vec![model("model-a")]);
        let mut updated = model("model-a");
        updated.price.input = 0.2;
        updated.price.output = 0.9;
        updated.cache_read = 0.004;
        let current = snapshot(vec![updated]);

        assert_eq!(
            detect_changes(&previous, &current),
            vec![
                CatalogChange::PriceChanged {
                    id: "model-a".to_string(),
                    field: PriceField::Input,
                    old: 0.15,
                    new: 0.2,
                },
                CatalogChange::PriceChanged {
                    id: "model-a".to_string(),
                    field: PriceField::Output,
                    old: 0.6,
                    new: 0.9,
                },
                CatalogChange::PriceChanged {
                    id: "model-a".to_string(),
                    field: PriceField::CacheRead,
                    old: 0.003,
                    new: 0.004,
                },
            ]
        );
    }

    #[test]
    fn detects_capability_flag_changes() {
        let previous = snapshot(vec![model("model-a")]);
        let mut updated = model("model-a");
        updated.tool_call = false;
        updated.attachment = false;
        updated.reasoning = false;
        let current = snapshot(vec![updated]);

        assert_eq!(
            detect_changes(&previous, &current),
            vec![
                CatalogChange::CapabilityChanged {
                    id: "model-a".to_string(),
                    flag: CapabilityFlag::ToolCall,
                    old: true,
                    new: false,
                },
                CatalogChange::CapabilityChanged {
                    id: "model-a".to_string(),
                    flag: CapabilityFlag::Attachment,
                    old: true,
                    new: false,
                },
                CatalogChange::CapabilityChanged {
                    id: "model-a".to_string(),
                    flag: CapabilityFlag::Reasoning,
                    old: true,
                    new: false,
                },
            ]
        );
    }

    #[test]
    fn identical_snapshots_produce_no_changes() {
        let previous = snapshot(vec![model("model-a"), model("model-b")]);

        assert!(detect_changes(&previous, &previous).is_empty());
    }
}
