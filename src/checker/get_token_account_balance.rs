use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (required_result_attributes, required_context_attributes, required_value_attributes) =
        match expectation {
            MethodExpectation::TokenAccountBalance {
                required_result_attributes,
                required_context_attributes,
                required_value_attributes,
            } => (
                required_result_attributes,
                required_context_attributes,
                required_value_attributes,
            ),
            other => anyhow::bail!(
                "getTokenAccountBalance expected a tokenAccountBalance validator, received {other:?}"
            ),
        };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getTokenAccountBalance validator",
    )?;
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

    let amount = value_object
        .get("amount")
        .and_then(Value::as_str)
        .context("result.value.amount was not a string")?;
    if amount.parse::<u64>().is_err() {
        anyhow::bail!("result.value.amount was not a base-10 u64 string");
    }

    let decimals = value_object
        .get("decimals")
        .and_then(Value::as_u64)
        .context("result.value.decimals was not a u64")?;

    let ui_amount = value_object
        .get("uiAmount")
        .context("result.value.uiAmount was missing")?;
    if !ui_amount.is_null() && ui_amount.as_f64().is_none() {
        anyhow::bail!("result.value.uiAmount was neither null nor a number");
    }

    let ui_amount_string = value_object
        .get("uiAmountString")
        .and_then(Value::as_str)
        .context("result.value.uiAmountString was not a string")?;

    Ok(format!(
        "slot={} apiVersion={} amount={} decimals={} uiAmountString={}",
        slot, api_version, amount, decimals, ui_amount_string
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
        MethodExpectation::TokenAccountBalance {
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
            required_value_attributes: vec![
                "amount".to_string(),
                "decimals".to_string(),
                "uiAmount".to_string(),
                "uiAmountString".to_string(),
            ],
        }
    }

    #[test]
    fn validates_token_account_balance_shape() {
        let result = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411327097
                },
                "value": {
                    "amount": "47209263",
                    "decimals": 6,
                    "uiAmount": 47.209263,
                    "uiAmountString": "47.209263"
                }
            }),
        )
        .expect("expected success");

        assert_eq!(
            result,
            "slot=411327097 apiVersion=3.1.11 amount=47209263 decimals=6 uiAmountString=47.209263"
        );
    }

    #[test]
    fn accepts_null_ui_amount() {
        let result = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411327097
                },
                "value": {
                    "amount": "47209263",
                    "decimals": 6,
                    "uiAmount": null,
                    "uiAmountString": "47.209263"
                }
            }),
        )
        .expect("expected success");

        assert!(result.contains("amount=47209263"));
    }

    #[test]
    fn rejects_non_numeric_amount_string() {
        let error = validate(
            &expectation(),
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 411327097
                },
                "value": {
                    "amount": "not-a-number",
                    "decimals": 6,
                    "uiAmount": 47.209263,
                    "uiAmountString": "47.209263"
                }
            }),
        )
        .expect_err("non-numeric amount should fail");

        assert!(
            error
                .to_string()
                .contains("result.value.amount was not a base-10 u64 string")
        );
    }
}
