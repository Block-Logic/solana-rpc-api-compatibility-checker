use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (
        required_result_attributes,
        required_context_attributes,
        required_value_attributes,
        required_range_attributes,
        expected_identity,
    ) = match expectation {
        MethodExpectation::BlockProduction {
            required_result_attributes,
            required_context_attributes,
            required_value_attributes,
            required_range_attributes,
            expected_identity,
        } => (
            required_result_attributes,
            required_context_attributes,
            required_value_attributes,
            required_range_attributes,
            expected_identity,
        ),
        other => anyhow::bail!(
            "getBlockProduction expected a blockProduction validator, received {other:?}"
        ),
    };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getBlockProduction validator",
    )?;
    assert_required_attributes(result_object, required_result_attributes, "result")?;

    let context_object = result_object
        .get("context")
        .and_then(Value::as_object)
        .context("result.context was not an object")?;
    assert_required_attributes(
        context_object,
        required_context_attributes,
        "result.context",
    )?;
    context_object
        .get("slot")
        .and_then(Value::as_u64)
        .context("result.context.slot was not a u64")?;
    context_object
        .get("apiVersion")
        .and_then(Value::as_str)
        .context("result.context.apiVersion was not a string")?;

    let value_object = result_object
        .get("value")
        .and_then(Value::as_object)
        .context("result.value was not an object")?;
    assert_required_attributes(value_object, required_value_attributes, "result.value")?;

    let by_identity_object = value_object
        .get("byIdentity")
        .and_then(Value::as_object)
        .context("result.value.byIdentity was not an object")?;
    let identity_entry = by_identity_object.get(expected_identity).with_context(|| {
        format!(
            "result.value.byIdentity was missing expected identity '{}'",
            expected_identity
        )
    })?;
    let identity_counts = identity_entry
        .as_array()
        .context("result.value.byIdentity.<identity> was not an array")?;
    if identity_counts.len() != 2 {
        anyhow::bail!(
            "result.value.byIdentity.<identity> must contain exactly 2 elements, received {}",
            identity_counts.len()
        );
    }
    for (index, value) in identity_counts.iter().enumerate() {
        value.as_u64().with_context(|| {
            format!("result.value.byIdentity.<identity>[{index}] was not a u64")
        })?;
    }

    let range_object = value_object
        .get("range")
        .and_then(Value::as_object)
        .context("result.value.range was not an object")?;
    assert_required_attributes(
        range_object,
        required_range_attributes,
        "result.value.range",
    )?;
    let first_slot = range_object
        .get("firstSlot")
        .and_then(Value::as_u64)
        .context("result.value.range.firstSlot was not a u64")?;
    let last_slot = range_object
        .get("lastSlot")
        .and_then(Value::as_u64)
        .context("result.value.range.lastSlot was not a u64")?;

    Ok(format!(
        "identity={} range={}..{}",
        expected_identity, first_slot, last_slot
    ))
}

fn assert_required_attributes(
    object: &serde_json::Map<String, Value>,
    required_attributes: &[String],
    location: &str,
) -> Result<()> {
    for field_name in required_attributes {
        if !object.contains_key(field_name) {
            anyhow::bail!("{location} was missing required '{field_name}' field");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::BlockProduction {
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
            required_value_attributes: vec!["byIdentity".to_string(), "range".to_string()],
            required_range_attributes: vec!["firstSlot".to_string(), "lastSlot".to_string()],
            expected_identity: "validator-1".to_string(),
        }
    }

    #[test]
    fn validates_block_production_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 123
                },
                "value": {
                    "byIdentity": {
                        "validator-1": [10, 9]
                    },
                    "range": {
                        "firstSlot": 100,
                        "lastSlot": 123
                    }
                }
            }),
        )
        .expect("expected success");

        assert_eq!(result, "identity=validator-1 range=100..123");
    }

    #[test]
    fn rejects_missing_identity_entry() {
        let error = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 123
                },
                "value": {
                    "byIdentity": {},
                    "range": {
                        "firstSlot": 100,
                        "lastSlot": 123
                    }
                }
            }),
        )
        .expect_err("missing identity should fail");

        assert!(
            error
                .to_string()
                .contains("result.value.byIdentity was missing expected identity")
        );
    }
}
