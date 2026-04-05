use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    match expectation {
        MethodExpectation::MaxShredInsertSlot => {}
        other => anyhow::bail!(
            "getMaxShredInsertSlot expected a maxShredInsertSlot validator, received {other:?}"
        ),
    }

    let value = result
        .as_u64()
        .context("result field was not a u64 as required by the getMaxShredInsertSlot validator")?;

    if value == 0 {
        anyhow::bail!("result must be greater than 0");
    }

    Ok(format!("maxShredInsertSlot={value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_positive_max_shred_insert_slot() {
        let result = validate(&MethodExpectation::MaxShredInsertSlot, &json!(123))
            .expect("expected success");

        assert_eq!(result, "maxShredInsertSlot=123");
    }

    #[test]
    fn rejects_zero_max_shred_insert_slot() {
        let error = validate(&MethodExpectation::MaxShredInsertSlot, &json!(0))
            .expect_err("zero should fail");

        assert!(error.to_string().contains("result must be greater than 0"));
    }
}
