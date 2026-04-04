use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, expected_result) = match expectation {
        MethodExpectation::BlockCommitment {
            required_result_attributes,
            expected_result,
        } => (required_result_attributes, expected_result),
        other => anyhow::bail!(
            "getBlockCommitment expected a blockCommitment validator, received {other:?}"
        ),
    };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getBlockCommitment validator",
    )?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    if result != expected_result {
        anyhow::bail!("result payload did not match the expected block commitment snapshot");
    }

    let commitment_summary = match result_object.get("commitment") {
        Some(Value::Array(values)) => values.len().to_string(),
        Some(Value::Null) => "null".to_string(),
        _ => anyhow::bail!("result field 'commitment' was neither null nor an array"),
    };
    let total_stake = result_object
        .get("totalStake")
        .and_then(Value::as_u64)
        .context("result field 'totalStake' was not an unsigned integer")?;

    Ok(format!(
        "commitment={} totalStake={}",
        commitment_summary, total_stake
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
    fn validates_matching_block_commitment_snapshot() {
        let expected_result = json!({
            "commitment": null,
            "totalStake": 42
        });

        let result = validate(
            &MethodExpectation::BlockCommitment {
                required_result_attributes: vec![
                    "commitment".to_string(),
                    "totalStake".to_string(),
                ],
                expected_result: expected_result.clone(),
            },
            &expected_result,
        )
        .expect("expected success");

        assert_eq!(result, "commitment=null totalStake=42");
    }

    #[test]
    fn rejects_block_commitment_snapshot_mismatch() {
        let error = validate(
            &MethodExpectation::BlockCommitment {
                required_result_attributes: vec![
                    "commitment".to_string(),
                    "totalStake".to_string(),
                ],
                expected_result: json!({
                    "commitment": null,
                    "totalStake": 42
                }),
            },
            &json!({
                "commitment": [1, 2, 3],
                "totalStake": 42
            }),
        )
        .expect_err("mismatched snapshot should fail");

        assert!(
            error
                .to_string()
                .contains("result payload did not match the expected block commitment snapshot")
        );
    }
}
