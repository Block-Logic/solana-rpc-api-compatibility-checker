use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let expected_result = match expectation {
        MethodExpectation::BlocksWithLimitSnapshot { expected_result } => expected_result,
        other => anyhow::bail!(
            "getBlocksWithLimit expected a blocksWithLimitSnapshot validator, received {other:?}"
        ),
    };

    let result_array = result
        .as_array()
        .context("result field was not an array as required by the getBlocksWithLimit validator")?;

    for (index, value) in result_array.iter().enumerate() {
        value
            .as_u64()
            .with_context(|| format!("result[{index}] was not an unsigned integer"))?;
    }

    if result != expected_result {
        anyhow::bail!("result payload did not match the expected blocks-with-limit snapshot");
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
    fn validates_matching_blocks_with_limit_snapshot() {
        let expected_result = json!([2, 3, 4, 5]);

        let result = validate(
            &MethodExpectation::BlocksWithLimitSnapshot {
                expected_result: expected_result.clone(),
            },
            &expected_result,
        )
        .expect("expected success");

        assert_eq!(result, "blocks=4 range=2..5");
    }

    #[test]
    fn rejects_blocks_with_limit_snapshot_mismatch() {
        let error = validate(
            &MethodExpectation::BlocksWithLimitSnapshot {
                expected_result: json!([2, 3, 4, 5]),
            },
            &json!([2, 3, 4, 6]),
        )
        .expect_err("mismatched snapshot should fail");

        assert!(
            error
                .to_string()
                .contains("result payload did not match the expected blocks-with-limit snapshot")
        );
    }
}
