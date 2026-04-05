use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, expected_result) = match expectation {
        MethodExpectation::InflationGovernor {
            required_result_attributes,
            expected_result,
        } => (required_result_attributes, expected_result),
        other => anyhow::bail!(
            "getInflationGovernor expected an inflationGovernor validator, received {other:?}"
        ),
    };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getInflationGovernor validator",
    )?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    if result != expected_result {
        anyhow::bail!("result payload did not match the expected inflation governor snapshot");
    }

    let foundation = require_f64(result_object, "foundation")?;
    let foundation_term = require_f64(result_object, "foundationTerm")?;
    let initial = require_f64(result_object, "initial")?;
    let taper = require_f64(result_object, "taper")?;
    let terminal = require_f64(result_object, "terminal")?;

    Ok(format!(
        "foundation={} foundationTerm={} initial={} taper={} terminal={}",
        foundation, foundation_term, initial, taper, terminal
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expected_result() -> Value {
        json!({
            "foundation": 0.0,
            "foundationTerm": 0.0,
            "initial": 0.08,
            "taper": 0.15,
            "terminal": 0.015
        })
    }

    #[test]
    fn validates_matching_inflation_governor_snapshot() {
        let result = validate(
            &MethodExpectation::InflationGovernor {
                required_result_attributes: vec![
                    "foundation".to_string(),
                    "foundationTerm".to_string(),
                    "initial".to_string(),
                    "taper".to_string(),
                    "terminal".to_string(),
                ],
                expected_result: expected_result(),
            },
            &expected_result(),
        )
        .expect("expected success");

        assert!(result.contains("initial=0.08"));
    }

    #[test]
    fn rejects_inflation_governor_snapshot_mismatch() {
        let error = validate(
            &MethodExpectation::InflationGovernor {
                required_result_attributes: vec![
                    "foundation".to_string(),
                    "foundationTerm".to_string(),
                    "initial".to_string(),
                    "taper".to_string(),
                    "terminal".to_string(),
                ],
                expected_result: expected_result(),
            },
            &json!({
                "foundation": 0.0,
                "foundationTerm": 0.0,
                "initial": 0.09,
                "taper": 0.15,
                "terminal": 0.015
            }),
        )
        .expect_err("mismatched snapshot should fail");

        assert!(
            error
                .to_string()
                .contains("result payload did not match the expected inflation governor snapshot")
        );
    }
}
