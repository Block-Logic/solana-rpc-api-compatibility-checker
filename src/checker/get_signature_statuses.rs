use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (
        required_result_attributes,
        required_context_attributes,
        expected_value,
        expected_api_version,
    ) = match expectation {
        MethodExpectation::SignatureStatuses {
            required_result_attributes,
            required_context_attributes,
            expected_value,
            expected_api_version,
        } => (
            required_result_attributes,
            required_context_attributes,
            expected_value,
            expected_api_version,
        ),
        other => anyhow::bail!(
            "getSignatureStatuses expected a signatureStatuses validator, received {other:?}"
        ),
    };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getSignatureStatuses validator",
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

    let actual_api_version = context_object
        .get("apiVersion")
        .and_then(Value::as_str)
        .context("result.context.apiVersion was not a string")?;
    if actual_api_version != expected_api_version {
        anyhow::bail!(
            "result.context.apiVersion expected '{}', received '{}'",
            expected_api_version,
            actual_api_version
        );
    }

    let context_slot = context_object
        .get("slot")
        .and_then(Value::as_u64)
        .context("result.context.slot was not a u64")?;
    if context_slot == 0 {
        anyhow::bail!("result.context.slot must be greater than 0");
    }

    let actual_value = result_object
        .get("value")
        .context("result was missing required 'value' field")?;
    if actual_value != expected_value {
        anyhow::bail!("result.value did not match the expected signature statuses snapshot");
    }

    let value_array = actual_value
        .as_array()
        .context("result.value was not an array")?;

    Ok(format!(
        "statuses={} contextSlot={}",
        value_array.len(),
        context_slot
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

    fn expected_value() -> Value {
        json!([
            {
                "confirmationStatus": "finalized",
                "confirmations": null,
                "err": null,
                "slot": 2,
                "status": {
                    "Ok": null
                }
            }
        ])
    }

    fn expectation() -> MethodExpectation {
        MethodExpectation::SignatureStatuses {
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
            expected_value: expected_value(),
            expected_api_version: "3.1.11".to_string(),
        }
    }

    #[test]
    fn validates_signature_statuses_shape_and_values() {
        let result = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 123
                },
                "value": expected_value()
            }),
        )
        .expect("expected success");

        assert_eq!(result, "statuses=1 contextSlot=123");
    }

    #[test]
    fn rejects_signature_status_value_mismatch() {
        let error = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 123
                },
                "value": [
                    {
                        "confirmationStatus": "processed",
                        "confirmations": null,
                        "err": null,
                        "slot": 2,
                        "status": {
                            "Ok": null
                        }
                    }
                ]
            }),
        )
        .expect_err("value mismatch should fail");

        assert!(
            error
                .to_string()
                .contains("result.value did not match the expected signature statuses snapshot")
        );
    }
}
