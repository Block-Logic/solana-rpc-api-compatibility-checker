use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, required_context_attributes, expected_value) =
        match expectation {
            MethodExpectation::Balance {
                required_result_attributes,
                required_context_attributes,
                expected_value,
            } => (
                required_result_attributes,
                required_context_attributes,
                *expected_value,
            ),
            other => anyhow::bail!("getBalance expected a balance validator, received {other:?}"),
        };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getBalance validator")?;
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

    context_object
        .get("slot")
        .and_then(Value::as_u64)
        .context("result.context.slot was not a u64")?;
    context_object
        .get("apiVersion")
        .and_then(Value::as_str)
        .context("result.context.apiVersion was not a string")?;

    let value = result_object
        .get("value")
        .and_then(Value::as_u64)
        .context("result.value was not a u64")?;

    if let Some(expected_value) = expected_value {
        if value != expected_value {
            anyhow::bail!(
                "result.value expected {}, received {}",
                expected_value,
                value
            );
        }
    } else if value == 0 {
        anyhow::bail!("result.value must be greater than 0");
    }

    Ok(format!("balance={}", value))
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

    #[test]
    fn validates_positive_balance() {
        let result = validate(
            &MethodExpectation::Balance {
                required_result_attributes: vec!["context".to_string(), "value".to_string()],
                required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
                expected_value: None,
            },
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 1
                },
                "value": 123
            }),
        )
        .expect("expected success");

        assert_eq!(result, "balance=123");
    }

    #[test]
    fn rejects_zero_balance() {
        let error = validate(
            &MethodExpectation::Balance {
                required_result_attributes: vec!["context".to_string(), "value".to_string()],
                required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
                expected_value: None,
            },
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 1
                },
                "value": 0
            }),
        )
        .expect_err("zero balance should fail");

        assert!(
            error
                .to_string()
                .contains("result.value must be greater than 0")
        );
    }

    #[test]
    fn validates_exact_zero_balance_when_expected() {
        let result = validate(
            &MethodExpectation::Balance {
                required_result_attributes: vec!["context".to_string(), "value".to_string()],
                required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
                expected_value: Some(0),
            },
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 1
                },
                "value": 0
            }),
        )
        .expect("expected success");

        assert_eq!(result, "balance=0");
    }
}
