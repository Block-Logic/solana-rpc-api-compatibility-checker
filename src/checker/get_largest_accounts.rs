use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (
        minimum_result_count,
        required_result_attributes,
        required_context_attributes,
        required_value_attributes,
    ) = match expectation {
        MethodExpectation::LargestAccounts {
            minimum_result_count,
            required_result_attributes,
            required_context_attributes,
            required_value_attributes,
        } => (
            *minimum_result_count,
            required_result_attributes,
            required_context_attributes,
            required_value_attributes,
        ),
        other => anyhow::bail!(
            "getLargestAccounts expected a largestAccounts validator, received {other:?}"
        ),
    };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getLargestAccounts validator",
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
    context_object
        .get("slot")
        .and_then(Value::as_u64)
        .context("result.context.slot was not a u64")?;

    let values = result_object
        .get("value")
        .and_then(Value::as_array)
        .context("result.value was not an array")?;

    if values.len() < minimum_result_count {
        anyhow::bail!(
            "result.value length {} was smaller than the required minimum {}",
            values.len(),
            minimum_result_count
        );
    }

    for (index, value) in values.iter().enumerate() {
        let value_object = value
            .as_object()
            .with_context(|| format!("result.value[{index}] was not an object"))?;
        assert_required_attributes(
            value_object,
            required_value_attributes,
            &format!("result.value[{index}]"),
        )?;
        value_object
            .get("address")
            .and_then(Value::as_str)
            .with_context(|| format!("result.value[{index}].address was not a string"))?;
        value_object
            .get("lamports")
            .and_then(Value::as_u64)
            .with_context(|| format!("result.value[{index}].lamports was not a u64"))?;
    }

    Ok(format!("accounts={}", values.len()))
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
        MethodExpectation::LargestAccounts {
            minimum_result_count: 1,
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["slot".to_string()],
            required_value_attributes: vec!["address".to_string(), "lamports".to_string()],
        }
    }

    #[test]
    fn validates_largest_accounts_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "context": {
                    "slot": 54
                },
                "value": [
                    {
                        "address": "99P8ZgtJYe1buSK8JXkvpLh8xPsCFuLYhz9hQFNw93WJ",
                        "lamports": 999974
                    }
                ]
            }),
        )
        .expect("expected success");

        assert_eq!(result, "accounts=1");
    }

    #[test]
    fn rejects_missing_value_field() {
        let error = validate(
            &expectation(),
            &json!({
                "context": {
                    "slot": 54
                },
                "value": [
                    {
                        "address": "99P8ZgtJYe1buSK8JXkvpLh8xPsCFuLYhz9hQFNw93WJ"
                    }
                ]
            }),
        )
        .expect_err("missing lamports should fail");

        assert!(
            error
                .to_string()
                .contains("result.value[0] was missing required 'lamports' field")
        );
    }
}
