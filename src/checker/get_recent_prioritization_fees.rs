use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (minimum_result_count, required_fee_attributes) = match expectation {
        MethodExpectation::RecentPrioritizationFees {
            minimum_result_count,
            required_fee_attributes,
        } => (*minimum_result_count, required_fee_attributes),
        other => anyhow::bail!(
            "getRecentPrioritizationFees expected a recentPrioritizationFees validator, received {other:?}"
        ),
    };

    let fees = result.as_array().context(
        "result field was not an array as required by the getRecentPrioritizationFees validator",
    )?;

    if fees.len() < minimum_result_count {
        anyhow::bail!(
            "result array length {} was smaller than the required minimum {}",
            fees.len(),
            minimum_result_count
        );
    }

    for (index, fee) in fees.iter().enumerate() {
        let fee_object = fee
            .as_object()
            .with_context(|| format!("result[{index}] was not an object"))?;
        assert_required_attributes(
            fee_object,
            required_fee_attributes,
            &format!("result[{index}]"),
        )?;

        for field_name in required_fee_attributes {
            fee_object
                .get(field_name)
                .and_then(Value::as_u64)
                .with_context(|| format!("result[{index}].{field_name} was not a u64"))?;
        }
    }

    Ok(format!("fees={}", fees.len()))
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
        MethodExpectation::RecentPrioritizationFees {
            minimum_result_count: 1,
            required_fee_attributes: vec!["prioritizationFee".to_string(), "slot".to_string()],
        }
    }

    #[test]
    fn validates_recent_prioritization_fees_shape() {
        let result = validate(
            &expectation(),
            &json!([
                {
                    "prioritizationFee": 0,
                    "slot": 99
                }
            ]),
        )
        .expect("expected success");

        assert_eq!(result, "fees=1");
    }

    #[test]
    fn rejects_missing_fee_field() {
        let error = validate(
            &expectation(),
            &json!([
                {
                    "slot": 99
                }
            ]),
        )
        .expect_err("missing prioritizationFee should fail");

        assert!(
            error
                .to_string()
                .contains("result[0] was missing required 'prioritizationFee' field")
        );
    }
}
