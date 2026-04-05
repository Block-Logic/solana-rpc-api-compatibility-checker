use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (expected_result_length, required_reward_attributes) = match expectation {
        MethodExpectation::InflationReward {
            expected_result_length,
            required_reward_attributes,
        } => (*expected_result_length, required_reward_attributes),
        other => anyhow::bail!(
            "getInflationReward expected an inflationReward validator, received {other:?}"
        ),
    };

    let result_array = result
        .as_array()
        .context("result field was not an array as required by the getInflationReward validator")?;

    if result_array.len() != expected_result_length {
        anyhow::bail!(
            "result array length {} did not match expected length {}",
            result_array.len(),
            expected_result_length
        );
    }

    let mut non_null_entries = 0usize;

    for (index, reward) in result_array.iter().enumerate() {
        if reward.is_null() {
            continue;
        }

        non_null_entries += 1;
        let reward_object = reward
            .as_object()
            .with_context(|| format!("result[{index}] was neither null nor an object"))?;

        assert_required_attributes(
            reward_object,
            required_reward_attributes,
            &format!("result[{index}]"),
        )?;

        reward_object
            .get("epoch")
            .and_then(Value::as_u64)
            .with_context(|| format!("result[{index}].epoch was not a u64"))?;
        reward_object
            .get("effectiveSlot")
            .and_then(Value::as_u64)
            .with_context(|| format!("result[{index}].effectiveSlot was not a u64"))?;
        reward_object
            .get("amount")
            .and_then(Value::as_u64)
            .with_context(|| format!("result[{index}].amount was not a u64"))?;
        reward_object
            .get("postBalance")
            .and_then(Value::as_u64)
            .with_context(|| format!("result[{index}].postBalance was not a u64"))?;

        if let Some(commission) = reward_object.get("commission") {
            commission
                .as_u64()
                .with_context(|| format!("result[{index}].commission was not a u64"))?;
        }
    }

    Ok(format!(
        "rewards={} nonNullRewards={}",
        result_array.len(),
        non_null_entries
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
        MethodExpectation::InflationReward {
            expected_result_length: 1,
            required_reward_attributes: vec![
                "epoch".to_string(),
                "effectiveSlot".to_string(),
                "amount".to_string(),
                "postBalance".to_string(),
            ],
        }
    }

    #[test]
    fn validates_null_reward_entry() {
        let result = validate(&expectation(), &json!([null])).expect("expected success");

        assert_eq!(result, "rewards=1 nonNullRewards=0");
    }

    #[test]
    fn validates_reward_object_shape() {
        let result = validate(
            &expectation(),
            &json!([{
                "epoch": 951,
                "effectiveSlot": 123,
                "amount": 2500,
                "postBalance": 499999442500u64,
                "commission": 5
            }]),
        )
        .expect("expected success");

        assert_eq!(result, "rewards=1 nonNullRewards=1");
    }

    #[test]
    fn rejects_missing_reward_attribute() {
        let error = validate(
            &expectation(),
            &json!([{
                "epoch": 951,
                "effectiveSlot": 123,
                "amount": 2500
            }]),
        )
        .expect_err("missing postBalance should fail");

        assert!(
            error
                .to_string()
                .contains("result[0] was missing required 'postBalance' field")
        );
    }
}
