use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, required_context_attributes, required_value_attributes) =
        match expectation {
            MethodExpectation::Supply {
                required_result_attributes,
                required_context_attributes,
                required_value_attributes,
            } => (
                required_result_attributes,
                required_context_attributes,
                required_value_attributes,
            ),
            other => anyhow::bail!("getSupply expected a supply validator, received {other:?}"),
        };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getSupply validator")?;
    assert_required_attributes(result_object, required_result_attributes, "result")?;

    let context_object = result_object
        .get("context")
        .and_then(Value::as_object)
        .context("result.context was not an object")?;
    assert_required_attributes(
        context_object,
        required_context_attributes,
        "result.context",
    )?;

    let api_version = context_object
        .get("apiVersion")
        .and_then(Value::as_str)
        .context("result.context.apiVersion was not a string")?;
    let slot = context_object
        .get("slot")
        .and_then(Value::as_u64)
        .context("result.context.slot was not a u64")?;

    let value_object = result_object
        .get("value")
        .and_then(Value::as_object)
        .context("result.value was not an object")?;
    assert_required_attributes(value_object, required_value_attributes, "result.value")?;

    let total = require_u64(value_object, "total")?;
    let circulating = require_u64(value_object, "circulating")?;
    let non_circulating = require_u64(value_object, "nonCirculating")?;
    if total < circulating {
        anyhow::bail!(
            "result.value.total must be greater than or equal to result.value.circulating"
        );
    }
    if total < non_circulating {
        anyhow::bail!(
            "result.value.total must be greater than or equal to result.value.nonCirculating"
        );
    }

    let non_circulating_accounts = value_object
        .get("nonCirculatingAccounts")
        .and_then(Value::as_array)
        .context("result.value.nonCirculatingAccounts was not an array")?;
    for (index, account) in non_circulating_accounts.iter().enumerate() {
        let account = account.as_str().with_context(|| {
            format!("result.value.nonCirculatingAccounts[{index}] was not a string")
        })?;
        if account.is_empty() {
            anyhow::bail!("result.value.nonCirculatingAccounts[{index}] must not be empty");
        }
    }

    Ok(format!(
        "slot={} apiVersion={} total={} circulating={} nonCirculating={} nonCirculatingAccounts={}",
        slot,
        api_version,
        total,
        circulating,
        non_circulating,
        non_circulating_accounts.len()
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

fn require_u64(object: &serde_json::Map<String, Value>, field_name: &str) -> Result<u64> {
    object
        .get(field_name)
        .and_then(Value::as_u64)
        .with_context(|| format!("result.value.{field_name} was not a u64"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expectation() -> MethodExpectation {
        MethodExpectation::Supply {
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
            required_value_attributes: vec![
                "circulating".to_string(),
                "nonCirculating".to_string(),
                "nonCirculatingAccounts".to_string(),
                "total".to_string(),
            ],
        }
    }

    #[test]
    fn validates_supply_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411326060
                },
                "value": {
                    "circulating": 90,
                    "nonCirculating": 10,
                    "nonCirculatingAccounts": ["account-1", "account-2"],
                    "total": 100
                }
            }),
        )
        .expect("expected success");

        assert_eq!(
            result,
            "slot=411326060 apiVersion=3.1.11 total=100 circulating=90 nonCirculating=10 nonCirculatingAccounts=2"
        );
    }

    #[test]
    fn rejects_non_string_non_circulating_account() {
        let error = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411326060
                },
                "value": {
                    "circulating": 90,
                    "nonCirculating": 10,
                    "nonCirculatingAccounts": ["account-1", 2],
                    "total": 100
                }
            }),
        )
        .expect_err("non-string account should fail");

        assert!(
            error
                .to_string()
                .contains("result.value.nonCirculatingAccounts[1] was not a string")
        );
    }
}
