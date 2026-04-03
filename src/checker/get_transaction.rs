use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, expected_result) = match expectation {
        MethodExpectation::TransactionSnapshot {
            required_result_attributes,
            expected_result,
        } => (required_result_attributes, expected_result),
        other => anyhow::bail!(
            "getTransaction expected a transactionSnapshot validator, received {other:?}"
        ),
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getTransaction validator")?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    if result != expected_result {
        anyhow::bail!("result payload did not match the expected transaction snapshot");
    }

    let slot = result_object
        .get("slot")
        .and_then(Value::as_u64)
        .context("result field 'slot' was not an unsigned integer")?;

    let transaction_summary = match result_object.get("transaction") {
        Some(Value::Object(_)) => "transaction=object",
        Some(Value::Array(values)) if values.len() == 2 => {
            let encoding = values
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("unknown-encoding");
            return Ok(format!(
                "slot={} transaction=array encoding={}",
                slot, encoding
            ));
        }
        Some(other) => {
            return Ok(format!(
                "slot={} transaction={}",
                slot,
                describe_value_kind(other)
            ));
        }
        None => "transaction=missing",
    };

    Ok(format!("slot={} {}", slot, transaction_summary))
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

fn describe_value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_matching_transaction_snapshot() {
        let expected_result = json!({
            "blockTime": null,
            "meta": null,
            "slot": 1,
            "transaction": ["abc", "base64"]
        });

        let result = validate(
            &MethodExpectation::TransactionSnapshot {
                required_result_attributes: vec![
                    "blockTime".to_string(),
                    "meta".to_string(),
                    "slot".to_string(),
                    "transaction".to_string(),
                ],
                expected_result: expected_result.clone(),
            },
            &expected_result,
        )
        .expect("expected success");

        assert!(result.contains("slot=1"));
    }

    #[test]
    fn rejects_snapshot_mismatch() {
        let error = validate(
            &MethodExpectation::TransactionSnapshot {
                required_result_attributes: vec![
                    "blockTime".to_string(),
                    "meta".to_string(),
                    "slot".to_string(),
                    "transaction".to_string(),
                ],
                expected_result: json!({
                    "blockTime": null,
                    "meta": null,
                    "slot": 1,
                    "transaction": ["abc", "base64"]
                }),
            },
            &json!({
                "blockTime": null,
                "meta": null,
                "slot": 2,
                "transaction": ["abc", "base64"]
            }),
        )
        .expect_err("mismatched snapshot should fail");

        assert!(
            error
                .to_string()
                .contains("result payload did not match the expected transaction snapshot")
        );
    }
}
