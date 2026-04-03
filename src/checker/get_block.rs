use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, expected_result) = match expectation {
        MethodExpectation::BlockSnapshot {
            required_result_attributes,
            expected_result,
        } => (required_result_attributes, expected_result),
        other => anyhow::bail!("getBlock expected a blockSnapshot validator, received {other:?}"),
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getBlock validator")?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    if result != expected_result {
        anyhow::bail!("result payload did not match the expected block snapshot");
    }

    let parent_slot = result_object
        .get("parentSlot")
        .and_then(Value::as_u64)
        .context("result field 'parentSlot' was not an unsigned integer")?;
    let transaction_count = result_object
        .get("transactions")
        .and_then(Value::as_array)
        .map(|transactions| transactions.len())
        .context("result field 'transactions' was not an array")?;

    Ok(format!(
        "parentSlot={} transactions={}",
        parent_slot, transaction_count
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_matching_block_snapshot() {
        let expected_result = json!({
            "blockHeight": null,
            "blockTime": null,
            "blockhash": "abc",
            "parentSlot": 0,
            "previousBlockhash": "def",
            "rewards": [],
            "transactions": []
        });

        let result = validate(
            &MethodExpectation::BlockSnapshot {
                required_result_attributes: vec![
                    "blockHeight".to_string(),
                    "blockTime".to_string(),
                    "blockhash".to_string(),
                    "parentSlot".to_string(),
                    "previousBlockhash".to_string(),
                    "rewards".to_string(),
                    "transactions".to_string(),
                ],
                expected_result: expected_result.clone(),
            },
            &expected_result,
        )
        .expect("expected success");

        assert!(result.contains("transactions=0"));
    }

    #[test]
    fn rejects_block_snapshot_mismatch() {
        let error = validate(
            &MethodExpectation::BlockSnapshot {
                required_result_attributes: vec![
                    "blockHeight".to_string(),
                    "blockTime".to_string(),
                    "blockhash".to_string(),
                    "parentSlot".to_string(),
                    "previousBlockhash".to_string(),
                    "rewards".to_string(),
                    "transactions".to_string(),
                ],
                expected_result: json!({
                    "blockHeight": null,
                    "blockTime": null,
                    "blockhash": "abc",
                    "parentSlot": 0,
                    "previousBlockhash": "def",
                    "rewards": [],
                    "transactions": []
                }),
            },
            &json!({
                "blockHeight": null,
                "blockTime": null,
                "blockhash": "xyz",
                "parentSlot": 0,
                "previousBlockhash": "def",
                "rewards": [],
                "transactions": []
            }),
        )
        .expect_err("mismatched snapshot should fail");

        assert!(
            error
                .to_string()
                .contains("result payload did not match the expected block snapshot")
        );
    }
}
