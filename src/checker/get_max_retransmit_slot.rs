use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    match expectation {
        MethodExpectation::MaxRetransmitSlot => {}
        other => anyhow::bail!(
            "getMaxRetransmitSlot expected a maxRetransmitSlot validator, received {other:?}"
        ),
    }

    let value = result
        .as_u64()
        .context("result field was not a u64 as required by the getMaxRetransmitSlot validator")?;

    if value == 0 {
        anyhow::bail!("result must be greater than 0");
    }

    Ok(format!("maxRetransmitSlot={value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_positive_max_retransmit_slot() {
        let result =
            validate(&MethodExpectation::MaxRetransmitSlot, &json!(123)).expect("expected success");

        assert_eq!(result, "maxRetransmitSlot=123");
    }

    #[test]
    fn rejects_zero_max_retransmit_slot() {
        let error = validate(&MethodExpectation::MaxRetransmitSlot, &json!(0))
            .expect_err("zero should fail");

        assert!(error.to_string().contains("result must be greater than 0"));
    }
}
