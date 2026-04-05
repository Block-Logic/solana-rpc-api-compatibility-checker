use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let required_result_attributes = match expectation {
        MethodExpectation::HighestSnapshotSlot {
            required_result_attributes,
        } => required_result_attributes,
        other => anyhow::bail!(
            "getHighestSnapshotSlot expected a highestSnapshotSlot validator, received {other:?}"
        ),
    };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getHighestSnapshotSlot validator",
    )?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    let full = require_u64(result_object, "full")?;
    let incremental = require_optional_u64(result_object, "incremental")?;

    let incremental_summary = match incremental {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    };

    Ok(format!("full={} incremental={}", full, incremental_summary))
}

fn assert_required_result_attributes(
    result_object: &serde_json::Map<String, Value>,
    required_result_attributes: &[String],
) -> Result<()> {
    for field_name in required_result_attributes {
        if !result_object.contains_key(field_name) {
            anyhow::bail!("result object was missing required '{}' field", field_name);
        }
    }

    Ok(())
}

fn require_u64(result_object: &serde_json::Map<String, Value>, field_name: &str) -> Result<u64> {
    result_object
        .get(field_name)
        .with_context(|| format!("result object was missing required '{}' field", field_name))?
        .as_u64()
        .with_context(|| format!("result field '{}' was not an unsigned integer", field_name))
}

fn require_optional_u64(
    result_object: &serde_json::Map<String, Value>,
    field_name: &str,
) -> Result<Option<u64>> {
    let value = result_object
        .get(field_name)
        .with_context(|| format!("result object was missing required '{}' field", field_name))?;

    if value.is_null() {
        return Ok(None);
    }

    value.as_u64().map(Some).with_context(|| {
        format!(
            "result field '{}' was neither null nor an unsigned integer",
            field_name
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::HighestSnapshotSlot {
            required_result_attributes: vec!["full".to_string(), "incremental".to_string()],
        }
    }

    #[test]
    fn validates_highest_snapshot_slot_with_incremental_value() {
        let result = validate(
            &expectation(),
            &json!({
                "full": 100,
                "incremental": 110
            }),
        )
        .expect("expected success");

        assert_eq!(result, "full=100 incremental=110");
    }

    #[test]
    fn validates_highest_snapshot_slot_with_null_incremental() {
        let result = validate(
            &expectation(),
            &json!({
                "full": 100,
                "incremental": null
            }),
        )
        .expect("expected success");

        assert_eq!(result, "full=100 incremental=null");
    }

    #[test]
    fn rejects_missing_required_result_attribute() {
        let error = validate(
            &expectation(),
            &json!({
                "full": 100
            }),
        )
        .expect_err("missing incremental should fail");

        assert!(
            error
                .to_string()
                .contains("result object was missing required 'incremental' field")
        );
    }
}
