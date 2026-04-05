use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let required_result_attributes = match expectation {
        MethodExpectation::EpochSchedule {
            required_result_attributes,
        } => required_result_attributes,
        other => {
            anyhow::bail!(
                "getEpochSchedule expected an epochSchedule validator, received {other:?}"
            )
        }
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getEpochSchedule validator")?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    let first_normal_epoch = require_u64(result_object, "firstNormalEpoch")?;
    let first_normal_slot = require_u64(result_object, "firstNormalSlot")?;
    let leader_schedule_slot_offset = require_u64(result_object, "leaderScheduleSlotOffset")?;
    let slots_per_epoch = require_u64(result_object, "slotsPerEpoch")?;
    let warmup = result_object
        .get("warmup")
        .and_then(Value::as_bool)
        .context("result field 'warmup' was not a boolean")?;

    if slots_per_epoch == 0 {
        anyhow::bail!("slotsPerEpoch must be greater than 0");
    }

    if leader_schedule_slot_offset == 0 {
        anyhow::bail!("leaderScheduleSlotOffset must be greater than 0");
    }

    Ok(format!(
        "slotsPerEpoch={} leaderScheduleSlotOffset={} firstNormalEpoch={} firstNormalSlot={} warmup={}",
        slots_per_epoch, leader_schedule_slot_offset, first_normal_epoch, first_normal_slot, warmup
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
        MethodExpectation::EpochSchedule {
            required_result_attributes: vec![
                "firstNormalEpoch".to_string(),
                "firstNormalSlot".to_string(),
                "leaderScheduleSlotOffset".to_string(),
                "slotsPerEpoch".to_string(),
                "warmup".to_string(),
            ],
        }
    }

    #[test]
    fn validates_epoch_schedule_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "firstNormalEpoch": 0,
                "firstNormalSlot": 0,
                "leaderScheduleSlotOffset": 432000,
                "slotsPerEpoch": 432000,
                "warmup": false
            }),
        )
        .expect("expected success");

        assert!(result.contains("slotsPerEpoch=432000"));
        assert!(result.contains("warmup=false"));
    }

    #[test]
    fn rejects_missing_required_result_attribute() {
        let error = validate(
            &expectation(),
            &json!({
                "firstNormalEpoch": 0,
                "firstNormalSlot": 0,
                "leaderScheduleSlotOffset": 432000,
                "slotsPerEpoch": 432000
            }),
        )
        .expect_err("missing warmup should fail");

        assert!(
            error
                .to_string()
                .contains("result object was missing required 'warmup' field")
        );
    }

    #[test]
    fn rejects_non_boolean_warmup() {
        let error = validate(
            &expectation(),
            &json!({
                "firstNormalEpoch": 0,
                "firstNormalSlot": 0,
                "leaderScheduleSlotOffset": 432000,
                "slotsPerEpoch": 432000,
                "warmup": "false"
            }),
        )
        .expect_err("string warmup should fail");

        assert!(
            error
                .to_string()
                .contains("result field 'warmup' was not a boolean")
        );
    }
}
