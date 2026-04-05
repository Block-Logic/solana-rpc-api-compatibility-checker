use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, required_context_attributes, required_value_attributes) =
        match expectation {
            MethodExpectation::LatestBlockhash {
                required_result_attributes,
                required_context_attributes,
                required_value_attributes,
            } => (
                required_result_attributes,
                required_context_attributes,
                required_value_attributes,
            ),
            other => {
                anyhow::bail!(
                    "getLatestBlockhash expected a latestBlockhash validator, received {other:?}"
                )
            }
        };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getLatestBlockhash validator",
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

    let value_object = result_object
        .get("value")
        .and_then(Value::as_object)
        .context("result.value was not an object")?;
    assert_required_attributes(value_object, required_value_attributes, "result.value")?;

    let blockhash = value_object
        .get("blockhash")
        .and_then(Value::as_str)
        .context("result.value.blockhash was not a string")?;
    if blockhash.is_empty() {
        anyhow::bail!("result.value.blockhash was empty");
    }

    let last_valid_block_height = value_object
        .get("lastValidBlockHeight")
        .and_then(Value::as_u64)
        .context("result.value.lastValidBlockHeight was not a u64")?;

    Ok(format!(
        "slot={} apiVersion={} blockhash={} lastValidBlockHeight={}",
        slot, api_version, blockhash, last_valid_block_height
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
        MethodExpectation::LatestBlockhash {
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
            required_value_attributes: vec![
                "blockhash".to_string(),
                "lastValidBlockHeight".to_string(),
            ],
        }
    }

    #[test]
    fn validates_latest_blockhash_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411246299
                },
                "value": {
                    "blockhash": "HeJRJUpxRca7jo1B9hYusNvhEbeJwQugD7h8L3qdVeTh",
                    "lastValidBlockHeight": 389345874
                }
            }),
        )
        .expect("expected success");

        assert!(result.contains("slot=411246299"));
        assert!(result.contains("lastValidBlockHeight=389345874"));
    }

    #[test]
    fn rejects_missing_value_attribute() {
        let error = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411246299
                },
                "value": {
                    "blockhash": "HeJRJUpxRca7jo1B9hYusNvhEbeJwQugD7h8L3qdVeTh"
                }
            }),
        )
        .expect_err("missing lastValidBlockHeight should fail");

        assert!(
            error
                .to_string()
                .contains("result.value was missing required 'lastValidBlockHeight' field")
        );
    }
}
