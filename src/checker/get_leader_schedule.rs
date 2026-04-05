use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let minimum_validator_count = match expectation {
        MethodExpectation::LeaderSchedule {
            minimum_validator_count,
        } => *minimum_validator_count,
        other => anyhow::bail!(
            "getLeaderSchedule expected a leaderSchedule validator, received {other:?}"
        ),
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getLeaderSchedule validator")?;

    if result_object.len() < minimum_validator_count {
        anyhow::bail!(
            "result object must contain at least {} validator entr{} , received {}",
            minimum_validator_count,
            if minimum_validator_count == 1 {
                "y"
            } else {
                "ies"
            },
            result_object.len()
        );
    }

    let (first_identity, first_schedule) = result_object
        .iter()
        .next()
        .context("result object was unexpectedly empty")?;

    if first_identity.is_empty() {
        anyhow::bail!("result object contained an empty validator identity key");
    }

    let first_schedule = first_schedule
        .as_array()
        .context("result.<identity> was not an array")?;

    for (identity, schedule) in result_object {
        if identity.is_empty() {
            anyhow::bail!("result object contained an empty validator identity key");
        }

        let schedule = schedule
            .as_array()
            .with_context(|| format!("result.{identity} was not an array"))?;

        for (index, slot_index) in schedule.iter().enumerate() {
            slot_index
                .as_u64()
                .with_context(|| format!("result.{identity}[{index}] was not a u64"))?;
        }
    }

    Ok(format!(
        "validators={} firstIdentity={} firstScheduleLength={}",
        result_object.len(),
        first_identity,
        first_schedule.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::LeaderSchedule {
            minimum_validator_count: 1,
        }
    }

    #[test]
    fn validates_leader_schedule_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "validator-1": [0, 1, 2, 3],
                "validator-2": [10, 11]
            }),
        )
        .expect("expected success");

        assert_eq!(
            result,
            "validators=2 firstIdentity=validator-1 firstScheduleLength=4"
        );
    }

    #[test]
    fn rejects_non_numeric_slot_index() {
        let error = validate(
            &expectation(),
            &json!({
                "validator-1": [0, "1"]
            }),
        )
        .expect_err("non-numeric slot index should fail");

        assert!(
            error
                .to_string()
                .contains("result.validator-1[1] was not a u64")
        );
    }
}
