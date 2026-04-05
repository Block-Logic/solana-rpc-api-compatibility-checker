use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (minimum_result_count, required_sample_attributes) = match expectation {
        MethodExpectation::RecentPerformanceSamples {
            minimum_result_count,
            required_sample_attributes,
        } => (*minimum_result_count, required_sample_attributes),
        other => anyhow::bail!(
            "getRecentPerformanceSamples expected a recentPerformanceSamples validator, received {other:?}"
        ),
    };

    let samples = result.as_array().context(
        "result field was not an array as required by the getRecentPerformanceSamples validator",
    )?;

    if samples.len() < minimum_result_count {
        anyhow::bail!(
            "result array length {} was smaller than the required minimum {}",
            samples.len(),
            minimum_result_count
        );
    }

    for (index, sample) in samples.iter().enumerate() {
        let sample_object = sample
            .as_object()
            .with_context(|| format!("result[{index}] was not an object"))?;
        assert_required_attributes(
            sample_object,
            required_sample_attributes,
            &format!("result[{index}]"),
        )?;

        for field_name in required_sample_attributes {
            sample_object
                .get(field_name)
                .and_then(Value::as_u64)
                .with_context(|| format!("result[{index}].{field_name} was not a u64"))?;
        }
    }

    Ok(format!("samples={}", samples.len()))
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
        MethodExpectation::RecentPerformanceSamples {
            minimum_result_count: 1,
            required_sample_attributes: vec![
                "numNonVoteTransactions".to_string(),
                "numSlots".to_string(),
                "numTransactions".to_string(),
                "samplePeriodSecs".to_string(),
                "slot".to_string(),
            ],
        }
    }

    #[test]
    fn validates_recent_performance_samples_shape() {
        let result = validate(
            &expectation(),
            &json!([
                {
                    "numNonVoteTransactions": 10,
                    "numSlots": 2,
                    "numTransactions": 12,
                    "samplePeriodSecs": 60,
                    "slot": 99
                }
            ]),
        )
        .expect("expected success");

        assert_eq!(result, "samples=1");
    }

    #[test]
    fn rejects_missing_sample_field() {
        let error = validate(
            &expectation(),
            &json!([
                {
                    "numNonVoteTransactions": 10,
                    "numSlots": 2,
                    "numTransactions": 12,
                    "samplePeriodSecs": 60
                }
            ]),
        )
        .expect_err("missing slot should fail");

        assert!(
            error
                .to_string()
                .contains("result[0] was missing required 'slot' field")
        );
    }
}
