use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    match expectation {
        MethodExpectation::GenesisHash => {}
        other => {
            anyhow::bail!("getGenesisHash expected a genesisHash validator, received {other:?}")
        }
    }

    let value = result
        .as_str()
        .context("result field was not a string as required by the getGenesisHash validator")?;

    if value.is_empty() {
        anyhow::bail!("result string must not be empty");
    }

    Ok(format!("genesisHash='{value}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_non_empty_genesis_hash_string() {
        let result =
            validate(&MethodExpectation::GenesisHash, &json!("abc123")).expect("expected success");

        assert_eq!(result, "genesisHash='abc123'");
    }

    #[test]
    fn rejects_empty_genesis_hash_string() {
        let error = validate(&MethodExpectation::GenesisHash, &json!(""))
            .expect_err("empty genesis hash should fail");

        assert!(
            error
                .to_string()
                .contains("result string must not be empty")
        );
    }
}
