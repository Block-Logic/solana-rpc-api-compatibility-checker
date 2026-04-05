use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let required_result_attributes = match expectation {
        MethodExpectation::InflationRate {
            required_result_attributes,
        } => required_result_attributes,
        other => {
            anyhow::bail!(
                "getInflationRate expected an inflationRate validator, received {other:?}"
            )
        }
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getInflationRate validator")?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    let total = require_f64(result_object, "total")?;
    let validator = require_f64(result_object, "validator")?;
    let foundation = require_f64(result_object, "foundation")?;
    let epoch = require_u64(result_object, "epoch")?;

    Ok(format!(
        "epoch={} total={} validator={} foundation={}",
        epoch, total, validator, foundation
    ))
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

fn require_f64(result_object: &serde_json::Map<String, Value>, field_name: &str) -> Result<f64> {
    result_object
        .get(field_name)
        .with_context(|| format!("result object was missing required '{}' field", field_name))?
        .as_f64()
        .with_context(|| format!("result field '{}' was not a number", field_name))
}

fn require_u64(result_object: &serde_json::Map<String, Value>, field_name: &str) -> Result<u64> {
    result_object
        .get(field_name)
        .with_context(|| format!("result object was missing required '{}' field", field_name))?
        .as_u64()
        .with_context(|| format!("result field '{}' was not an unsigned integer", field_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::InflationRate {
            required_result_attributes: vec![
                "epoch".to_string(),
                "foundation".to_string(),
                "total".to_string(),
                "validator".to_string(),
            ],
        }
    }

    #[test]
    fn validates_inflation_rate_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "epoch": 951,
                "foundation": 0.0,
                "total": 0.03918552640613479,
                "validator": 0.03918552640613479
            }),
        )
        .expect("expected success");

        assert!(result.contains("epoch=951"));
    }

    #[test]
    fn rejects_missing_required_result_attribute() {
        let error = validate(
            &expectation(),
            &json!({
                "epoch": 951,
                "foundation": 0.0,
                "total": 0.03918552640613479
            }),
        )
        .expect_err("missing validator should fail");

        assert!(
            error
                .to_string()
                .contains("result object was missing required 'validator' field")
        );
    }
}
