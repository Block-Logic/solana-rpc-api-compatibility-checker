use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let expected_value = match expectation {
        MethodExpectation::FirstAvailableBlock { expected_value } => *expected_value,
        other => anyhow::bail!(
            "getFirstAvailableBlock expected a firstAvailableBlock validator, received {other:?}"
        ),
    };

    let value = result.as_u64().context(
        "result field was not a u64 as required by the getFirstAvailableBlock validator",
    )?;

    if value != expected_value {
        anyhow::bail!("result expected {}, received {}", expected_value, value);
    }

    Ok(format!("firstAvailableBlock={value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_matching_first_available_block() {
        let result = validate(
            &MethodExpectation::FirstAvailableBlock { expected_value: 0 },
            &json!(0u64),
        )
        .expect("expected success");

        assert_eq!(result, "firstAvailableBlock=0");
    }

    #[test]
    fn rejects_first_available_block_mismatch() {
        let error = validate(
            &MethodExpectation::FirstAvailableBlock { expected_value: 0 },
            &json!(1u64),
        )
        .expect_err("mismatched first available block should fail");

        assert!(error.to_string().contains("result expected 0, received 1"));
    }
}
