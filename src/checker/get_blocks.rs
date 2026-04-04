use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let expected_result = match expectation {
        MethodExpectation::BlocksSnapshot { expected_result } => expected_result,
        other => anyhow::bail!("getBlocks expected a blocksSnapshot validator, received {other:?}"),
    };

    let result_array = result
        .as_array()
        .context("result field was not an array as required by the getBlocks validator")?;

    for (index, value) in result_array.iter().enumerate() {
        value
            .as_u64()
            .with_context(|| format!("result[{index}] was not an unsigned integer"))?;
    }

    if result != expected_result {
        anyhow::bail!("result payload did not match the expected blocks snapshot");
    }

    let first_slot = result_array
        .first()
        .and_then(Value::as_u64)
        .context("result array was unexpectedly empty")?;
    let last_slot = result_array
        .last()
        .and_then(Value::as_u64)
        .context("result array was unexpectedly empty")?;

    Ok(format!(
        "blocks={} range={}..{}",
        result_array.len(),
        first_slot,
        last_slot
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_matching_blocks_snapshot() {
        let expected_result = json!([2, 3, 4]);

        let result = validate(
            &MethodExpectation::BlocksSnapshot {
                expected_result: expected_result.clone(),
            },
            &expected_result,
        )
        .expect("expected success");

        assert_eq!(result, "blocks=3 range=2..4");
    }

    #[test]
    fn rejects_blocks_snapshot_mismatch() {
        let error = validate(
            &MethodExpectation::BlocksSnapshot {
                expected_result: json!([2, 3, 4]),
            },
            &json!([2, 3, 5]),
        )
        .expect_err("mismatched snapshot should fail");

        assert!(
            error
                .to_string()
                .contains("result payload did not match the expected blocks snapshot")
        );
    }
}
