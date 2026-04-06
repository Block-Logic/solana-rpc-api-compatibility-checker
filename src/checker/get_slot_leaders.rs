use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let expected_result_length = match expectation {
        MethodExpectation::SlotLeaders {
            expected_result_length,
        } => *expected_result_length,
        other => {
            anyhow::bail!("getSlotLeaders expected a slotLeaders validator, received {other:?}")
        }
    };

    let leaders = result
        .as_array()
        .context("result field was not an array as required by the getSlotLeaders validator")?;

    if leaders.len() != expected_result_length {
        anyhow::bail!(
            "result array length expected {}, received {}",
            expected_result_length,
            leaders.len()
        );
    }

    for (index, leader) in leaders.iter().enumerate() {
        let leader = leader
            .as_str()
            .with_context(|| format!("result[{index}] was not a string"))?;
        if leader.is_empty() {
            anyhow::bail!("result[{index}] must not be empty");
        }
    }

    Ok(format!("slotLeaders={}", leaders.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::SlotLeaders {
            expected_result_length: 3,
        }
    }

    #[test]
    fn validates_slot_leaders_shape() {
        let result = validate(&expectation(), &json!(["leader-1", "leader-1", "leader-2"]))
            .expect("expected success");

        assert_eq!(result, "slotLeaders=3");
    }

    #[test]
    fn rejects_wrong_result_length() {
        let error = validate(&expectation(), &json!(["leader-1", "leader-2"]))
            .expect_err("wrong length should fail");

        assert!(
            error
                .to_string()
                .contains("result array length expected 3, received 2")
        );
    }

    #[test]
    fn rejects_empty_leader_string() {
        let error = validate(&expectation(), &json!(["leader-1", "", "leader-2"]))
            .expect_err("empty leader should fail");

        assert!(error.to_string().contains("result[1] must not be empty"));
    }
}
