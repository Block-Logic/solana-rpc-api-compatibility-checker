use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let required_result_attributes = match expectation {
        MethodExpectation::Identity {
            required_result_attributes,
        } => required_result_attributes,
        other => anyhow::bail!("getIdentity expected an identity validator, received {other:?}"),
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getIdentity validator")?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    let identity = result_object
        .get("identity")
        .and_then(Value::as_str)
        .context("result.identity was not a string")?;

    if identity.is_empty() {
        anyhow::bail!("result.identity must not be empty");
    }

    Ok(format!("identity='{identity}'"))
}

fn assert_required_result_attributes(
    result_object: &serde_json::Map<String, Value>,
    required_result_attributes: &[String],
) -> Result<()> {
    for field_name in required_result_attributes {
        if !result_object.contains_key(field_name) {
            anyhow::bail!("result object was missing required '{}' field", field_name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::Identity {
            required_result_attributes: vec!["identity".to_string()],
        }
    }

    #[test]
    fn validates_identity_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "identity": "validator-pubkey"
            }),
        )
        .expect("expected success");

        assert_eq!(result, "identity='validator-pubkey'");
    }

    #[test]
    fn rejects_missing_identity_field() {
        let error = validate(&expectation(), &json!({})).expect_err("missing identity should fail");

        assert!(
            error
                .to_string()
                .contains("result object was missing required 'identity' field")
        );
    }

    #[test]
    fn rejects_empty_identity_string() {
        let error = validate(
            &expectation(),
            &json!({
                "identity": ""
            }),
        )
        .expect_err("empty identity should fail");

        assert!(
            error
                .to_string()
                .contains("result.identity must not be empty")
        );
    }
}
