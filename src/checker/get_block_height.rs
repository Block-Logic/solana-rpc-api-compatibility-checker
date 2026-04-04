use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    match expectation {
        MethodExpectation::BlockHeight => {}
        other => {
            anyhow::bail!("getBlockHeight expected a block height validator, received {other:?}")
        }
    }

    let value = result
        .as_u64()
        .context("result field was not a u64 as required by the getBlockHeight validator")?;

    if value == 0 {
        anyhow::bail!("result must be greater than 0");
    }

    Ok(format!("blockHeight={value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_positive_block_height() {
        let result =
            validate(&MethodExpectation::BlockHeight, &json!(123)).expect("expected success");

        assert_eq!(result, "blockHeight=123");
    }

    #[test]
    fn rejects_zero_block_height() {
        let error =
            validate(&MethodExpectation::BlockHeight, &json!(0)).expect_err("zero should fail");

        assert!(error.to_string().contains("result must be greater than 0"));
    }
}
