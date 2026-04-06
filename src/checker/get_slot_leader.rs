use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    match expectation {
        MethodExpectation::SlotLeader => {}
        other => {
            anyhow::bail!("getSlotLeader expected a slotLeader validator, received {other:?}")
        }
    }

    let slot_leader = result
        .as_str()
        .context("result field was not a string as required by the getSlotLeader validator")?;

    if slot_leader.is_empty() {
        anyhow::bail!("result string must not be empty");
    }

    Ok(format!("slotLeader='{slot_leader}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_non_empty_slot_leader_string() {
        let result = validate(&MethodExpectation::SlotLeader, &json!("validator-pubkey"))
            .expect("expected success");

        assert_eq!(result, "slotLeader='validator-pubkey'");
    }

    #[test]
    fn rejects_empty_slot_leader_string() {
        let error = validate(&MethodExpectation::SlotLeader, &json!(""))
            .expect_err("empty slot leader should fail");

        assert!(
            error
                .to_string()
                .contains("result string must not be empty")
        );
    }
}
