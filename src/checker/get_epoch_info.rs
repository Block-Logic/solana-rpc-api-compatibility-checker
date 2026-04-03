use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let required_result_attributes = match expectation {
        MethodExpectation::EpochInfo {
            required_result_attributes,
        } => required_result_attributes,
        other => anyhow::bail!("getEpochInfo expected an epochInfo validator, received {other:?}"),
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getEpochInfo validator")?;

    assert_required_result_attributes(result_object, required_result_attributes)?;

    let absolute_slot = require_u64(result_object, "absoluteSlot")?;
    let block_height = require_u64(result_object, "blockHeight")?;
    let epoch = require_u64(result_object, "epoch")?;
    let slot_index = require_u64(result_object, "slotIndex")?;
    let slots_in_epoch = require_u64(result_object, "slotsInEpoch")?;
    let transaction_count = require_optional_u64(result_object, "transactionCount")?;

    if slots_in_epoch == 0 {
        anyhow::bail!("slotsInEpoch must be greater than 0");
    }

    if slot_index >= slots_in_epoch {
        anyhow::bail!(
            "slotIndex must be less than slotsInEpoch, received slotIndex={} and slotsInEpoch={}",
            slot_index,
            slots_in_epoch
        );
    }

    if absolute_slot < slot_index {
        anyhow::bail!(
            "absoluteSlot must be greater than or equal to slotIndex, received absoluteSlot={} and slotIndex={}",
            absolute_slot,
            slot_index
        );
    }

    let transaction_count_summary = match transaction_count {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    };

    Ok(format!(
        "epoch={} absoluteSlot={} blockHeight={} slotIndex={} slotsInEpoch={} transactionCount={}",
        epoch, absolute_slot, block_height, slot_index, slots_in_epoch, transaction_count_summary
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

fn require_optional_u64(
    result_object: &serde_json::Map<String, Value>,
    field_name: &str,
) -> Result<Option<u64>> {
    let value = result_object
        .get(field_name)
        .with_context(|| format!("result object was missing required '{}' field", field_name))?;

    if value.is_null() {
        return Ok(None);
    }

    value.as_u64().map(Some).with_context(|| {
        format!(
            "result field '{}' was neither null nor an unsigned integer",
            field_name
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_epoch_info_result() {
        let result = validate(
            &MethodExpectation::EpochInfo {
                required_result_attributes: vec![
                    "absoluteSlot".to_string(),
                    "blockHeight".to_string(),
                    "epoch".to_string(),
                    "slotIndex".to_string(),
                    "slotsInEpoch".to_string(),
                    "transactionCount".to_string(),
                ],
            },
            &json!({
                "absoluteSlot": 10,
                "blockHeight": 8,
                "epoch": 1,
                "slotIndex": 2,
                "slotsInEpoch": 32,
                "transactionCount": 99
            }),
        )
        .expect("expected success");

        assert!(result.contains("epoch=1"));
    }

    #[test]
    fn rejects_slot_index_out_of_range() {
        let error = validate(
            &MethodExpectation::EpochInfo {
                required_result_attributes: vec![
                    "absoluteSlot".to_string(),
                    "blockHeight".to_string(),
                    "epoch".to_string(),
                    "slotIndex".to_string(),
                    "slotsInEpoch".to_string(),
                    "transactionCount".to_string(),
                ],
            },
            &json!({
                "absoluteSlot": 10,
                "blockHeight": 8,
                "epoch": 1,
                "slotIndex": 32,
                "slotsInEpoch": 32,
                "transactionCount": null
            }),
        )
        .expect_err("slotIndex >= slotsInEpoch should fail");

        assert!(
            error
                .to_string()
                .contains("slotIndex must be less than slotsInEpoch")
        );
    }

    #[test]
    fn rejects_missing_required_result_attribute() {
        let error = validate(
            &MethodExpectation::EpochInfo {
                required_result_attributes: vec![
                    "absoluteSlot".to_string(),
                    "blockHeight".to_string(),
                    "epoch".to_string(),
                    "slotIndex".to_string(),
                    "slotsInEpoch".to_string(),
                    "transactionCount".to_string(),
                ],
            },
            &json!({
                "absoluteSlot": 10,
                "blockHeight": 8,
                "epoch": 1,
                "slotIndex": 2,
                "slotsInEpoch": 32
            }),
        )
        .expect_err("missing transactionCount should fail");

        assert!(
            error
                .to_string()
                .contains("result object was missing required 'transactionCount' field")
        );
    }
}
