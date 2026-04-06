use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, required_context_attributes) = match expectation {
        MethodExpectation::StakeMinimumDelegation {
            required_result_attributes,
            required_context_attributes,
        } => (required_result_attributes, required_context_attributes),
        other => anyhow::bail!(
            "getStakeMinimumDelegation expected a stakeMinimumDelegation validator, received {other:?}"
        ),
    };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getStakeMinimumDelegation validator",
    )?;
    assert_required_attributes(result_object, required_result_attributes, "result")?;

    let context_object = result_object
        .get("context")
        .and_then(Value::as_object)
        .context("result.context was not an object")?;
    assert_required_attributes(
        context_object,
        required_context_attributes,
        "result.context",
    )?;

    let api_version = context_object
        .get("apiVersion")
        .and_then(Value::as_str)
        .context("result.context.apiVersion was not a string")?;
    let slot = context_object
        .get("slot")
        .and_then(Value::as_u64)
        .context("result.context.slot was not a u64")?;

    let value = result_object
        .get("value")
        .and_then(Value::as_u64)
        .context("result.value was not a u64")?;
    if value == 0 {
        anyhow::bail!("result.value must be greater than 0");
    }

    Ok(format!(
        "slot={} apiVersion={} stakeMinimumDelegation={}",
        slot, api_version, value
    ))
}

fn assert_required_attributes(
    object: &serde_json::Map<String, Value>,
    required_attributes: &[String],
    location: &str,
) -> Result<()> {
    for field_name in required_attributes {
        if !object.contains_key(field_name) {
            anyhow::bail!("{location} was missing required '{field_name}' field");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::StakeMinimumDelegation {
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
        }
    }

    #[test]
    fn validates_stake_minimum_delegation_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411325179
                },
                "value": 1
            }),
        )
        .expect("expected success");

        assert_eq!(
            result,
            "slot=411325179 apiVersion=3.1.11 stakeMinimumDelegation=1"
        );
    }

    #[test]
    fn rejects_zero_stake_minimum_delegation() {
        let error = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411325179
                },
                "value": 0
            }),
        )
        .expect_err("zero value should fail");

        assert!(
            error
                .to_string()
                .contains("result.value must be greater than 0")
        );
    }
}
