use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let expected_value = match expectation {
        MethodExpectation::BlockTime { expected_value } => *expected_value,
        other => anyhow::bail!("getBlockTime expected a blockTime validator, received {other:?}"),
    };

    let value = result
        .as_u64()
        .context("result field was not a u64 as required by the getBlockTime validator")?;

    if value != expected_value {
        anyhow::bail!("result expected {}, received {}", expected_value, value);
    }

    Ok(format!("blockTime={value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_matching_block_time() {
        let result = validate(
            &MethodExpectation::BlockTime {
                expected_value: 1_633_504_705,
            },
            &json!(1_633_504_705u64),
        )
        .expect("expected success");

        assert_eq!(result, "blockTime=1633504705");
    }

    #[test]
    fn rejects_block_time_mismatch() {
        let error = validate(
            &MethodExpectation::BlockTime {
                expected_value: 1_633_504_705,
            },
            &json!(1_633_504_706u64),
        )
        .expect_err("mismatched block time should fail");

        assert!(
            error
                .to_string()
                .contains("result expected 1633504705, received 1633504706")
        );
    }
}
