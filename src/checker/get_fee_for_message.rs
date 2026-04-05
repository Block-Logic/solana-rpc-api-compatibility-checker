use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, required_context_attributes) = match expectation {
        MethodExpectation::FeeForMessage {
            required_result_attributes,
            required_context_attributes,
        } => (required_result_attributes, required_context_attributes),
        other => {
            anyhow::bail!("getFeeForMessage expected a feeForMessage validator, received {other:?}")
        }
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getFeeForMessage validator")?;
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
        .context("result was missing required 'value' field")?;

    if !value.is_null() && value.as_u64().is_none() {
        anyhow::bail!("result.value was neither null nor a u64");
    }

    let value_summary = if let Some(value) = value.as_u64() {
        value.to_string()
    } else {
        "null".to_string()
    };

    Ok(format!("fee={value_summary}"))
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

    fn null_expectation() -> MethodExpectation {
        MethodExpectation::FeeForMessage {
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
        }
    }

    #[test]
    fn validates_null_fee_with_context_shape() {
        let result = validate(
            &null_expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411090718
                },
                "value": null
            }),
        )
        .expect("expected success");

        assert_eq!(result, "fee=null");
    }

    #[test]
    fn validates_exact_numeric_fee_when_expected() {
        let result = validate(
            &MethodExpectation::FeeForMessage {
                required_result_attributes: vec!["context".to_string(), "value".to_string()],
                required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
            },
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 5068
                },
                "value": 5000
            }),
        )
        .expect("expected success");

        assert_eq!(result, "fee=5000");
    }

    #[test]
    fn rejects_non_numeric_non_null_fee_value() {
        let error = validate(
            &null_expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411090718
                },
                "value": "5000"
            }),
        )
        .expect_err("string fee value should fail");

        assert!(
            error
                .to_string()
                .contains("result.value was neither null nor a u64")
        );
    }
}
