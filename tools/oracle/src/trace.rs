use crate::schema::{SCHEMA_VERSION, Scenario};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Reference,
    Mantle,
}

impl Backend {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "reference" => Ok(Self::Reference),
            "mantle" => Ok(Self::Mantle),
            _ => Err(format!(
                "unknown backend {value:?}; expected reference or mantle"
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawRecord {
    pub action_id: String,
    pub kind: String,
    #[serde(default)]
    pub data: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    pub schema_version: u32,
    pub scenario: String,
    pub backend: Backend,
    pub records: Vec<TraceRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TraceRecord {
    pub sequence: usize,
    pub action_id: String,
    pub kind: String,
    pub data: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Comparison {
    pub schema_version: u32,
    pub scenario: String,
    pub equal_records: usize,
    pub differences: Vec<Difference>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Difference {
    pub sequence: usize,
    pub action_id: String,
    pub reference: Option<TraceRecord>,
    pub mantle: Option<TraceRecord>,
}

pub fn normalize(
    scenario: &Scenario,
    backend: Backend,
    raw_path: &Path,
) -> Result<Trace, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(raw_path)?;
    let mut records = Vec::new();
    for (line_index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut record: RawRecord = serde_json::from_str(line).map_err(|error| {
            format!(
                "{}:{}: invalid raw trace record: {error}",
                raw_path.display(),
                line_index + 1
            )
        })?;
        if !scenario
            .actions
            .iter()
            .any(|action| action.id() == record.action_id)
        {
            return Err(format!(
                "raw trace references unknown action id {:?}",
                record.action_id
            )
            .into());
        }
        for volatile in ["thread", "wall_time", "elapsed_ns", "stack_trace"] {
            record.data.remove(volatile);
        }
        records.push(TraceRecord {
            sequence: records.len(),
            action_id: record.action_id,
            kind: record.kind,
            data: record.data,
        });
    }

    let observed = records
        .iter()
        .map(|record| record.action_id.as_str())
        .collect::<Vec<_>>();
    let expected = scenario
        .actions
        .iter()
        .map(super::schema::Action::id)
        .collect::<Vec<_>>();
    if observed != expected {
        return Err(format!(
            "raw trace action order mismatch: expected {expected:?}, observed {observed:?}"
        )
        .into());
    }

    Ok(Trace {
        schema_version: SCHEMA_VERSION,
        scenario: scenario.name.clone(),
        backend,
        records,
    })
}

pub fn compare(reference: &Trace, mantle: &Trace) -> Result<Comparison, String> {
    if reference.schema_version != mantle.schema_version || reference.scenario != mantle.scenario {
        return Err("trace schema/scenario mismatch".into());
    }
    if reference.backend != Backend::Reference || mantle.backend != Backend::Mantle {
        return Err("comparison requires reference then Mantle traces".into());
    }

    let length = reference.records.len().max(mantle.records.len());
    let mut equal_records = 0;
    let mut differences = Vec::new();
    for sequence in 0..length {
        let reference_record = reference.records.get(sequence).cloned();
        let mantle_record = mantle.records.get(sequence).cloned();
        if reference_record == mantle_record {
            equal_records += 1;
        } else {
            let action_id = reference_record
                .as_ref()
                .or(mantle_record.as_ref())
                .map_or_else(|| "<missing>".into(), |record| record.action_id.clone());
            differences.push(Difference {
                sequence,
                action_id,
                reference: reference_record,
                mantle: mantle_record,
            });
        }
    }

    Ok(Comparison {
        schema_version: SCHEMA_VERSION,
        scenario: reference.scenario.clone(),
        equal_records,
        differences,
    })
}

pub fn assert_difference_actions(
    comparison: &Comparison,
    allowed: &BTreeSet<String>,
) -> Result<(), String> {
    let actual = comparison
        .differences
        .iter()
        .map(|difference| difference.action_id.clone())
        .collect::<BTreeSet<_>>();
    if actual == *allowed {
        Ok(())
    } else {
        Err(format!(
            "differential action set mismatch: expected {allowed:?}, observed {actual:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{Backend, RawRecord, Trace, TraceRecord, assert_difference_actions, compare};
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};

    fn record(backend_value: i64) -> TraceRecord {
        TraceRecord {
            sequence: 0,
            action_id: "a".into(),
            kind: "state".into(),
            data: BTreeMap::from([("value".into(), json!(backend_value))]),
        }
    }

    #[test]
    fn comparison_preserves_evidence_from_both_backends() {
        let reference = Trace {
            schema_version: 1,
            scenario: "s".into(),
            backend: Backend::Reference,
            records: vec![record(1)],
        };
        let mantle = Trace {
            schema_version: 1,
            scenario: "s".into(),
            backend: Backend::Mantle,
            records: vec![record(2)],
        };
        let comparison = compare(&reference, &mantle).unwrap();
        assert_eq!(comparison.equal_records, 0);
        assert_eq!(comparison.differences.len(), 1);
        assert!(assert_difference_actions(&comparison, &BTreeSet::from(["a".into()])).is_ok());
        assert!(assert_difference_actions(&comparison, &BTreeSet::new()).is_err());
    }

    #[test]
    fn raw_record_rejects_unknown_fields() {
        let result = serde_json::from_value::<RawRecord>(json!({
            "action_id": "a",
            "kind": "state",
            "extra": true
        }));
        assert!(result.is_err());
    }
}
