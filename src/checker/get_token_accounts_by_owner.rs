use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (
        minimum_result_count,
        required_result_attributes,
        required_context_attributes,
        required_value_entry_attributes,
        required_account_attributes,
        required_token_amount_attributes,
        expected_account_owner,
        expected_data_program,
        expected_mint,
        expected_token_owner,
    ) = match expectation {
        MethodExpectation::TokenAccountsByOwner {
            minimum_result_count,
            required_result_attributes,
            required_context_attributes,
            required_value_entry_attributes,
            required_account_attributes,
            required_token_amount_attributes,
            expected_account_owner,
            expected_data_program,
            expected_mint,
            expected_token_owner,
        } => (
            *minimum_result_count,
            required_result_attributes,
            required_context_attributes,
            required_value_entry_attributes,
            required_account_attributes,
            required_token_amount_attributes,
            expected_account_owner,
            expected_data_program,
            expected_mint,
            expected_token_owner,
        ),
        other => anyhow::bail!(
            "getTokenAccountsByOwner expected a tokenAccountsByOwner validator, received {other:?}"
        ),
    };

    let result_object = result.as_object().context(
        "result field was not an object as required by the getTokenAccountsByOwner validator",
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
    context_object
        .get("apiVersion")
        .and_then(Value::as_str)
        .context("result.context.apiVersion was not a string")?;
    context_object
        .get("slot")
        .and_then(Value::as_u64)
        .context("result.context.slot was not a u64")?;

    let entries = result_object
        .get("value")
        .and_then(Value::as_array)
        .context("result.value was not an array")?;
    if entries.len() < minimum_result_count {
        anyhow::bail!(
            "result.value length {} was smaller than the required minimum {}",
            entries.len(),
            minimum_result_count
        );
    }

    for (index, entry) in entries.iter().enumerate() {
        validate_entry(
            index,
            entry,
            required_value_entry_attributes,
            required_account_attributes,
            required_token_amount_attributes,
            expected_account_owner,
            expected_data_program,
            expected_mint,
            expected_token_owner,
        )?;
    }

    Ok(format!("tokenAccounts={}", entries.len()))
}

fn validate_entry(
    index: usize,
    entry: &Value,
    required_value_entry_attributes: &[String],
    required_account_attributes: &[String],
    required_token_amount_attributes: &[String],
    expected_account_owner: &str,
    expected_data_program: &str,
    expected_mint: &str,
    expected_token_owner: &str,
) -> Result<()> {
    let entry_object = entry
        .as_object()
        .with_context(|| format!("result.value[{index}] was not an object"))?;
    assert_required_attributes(
        entry_object,
        required_value_entry_attributes,
        &format!("result.value[{index}]"),
    )?;

    entry_object
        .get("pubkey")
        .and_then(Value::as_str)
        .with_context(|| format!("result.value[{index}].pubkey was not a string"))?;

    let account_object = entry_object
        .get("account")
        .and_then(Value::as_object)
        .with_context(|| format!("result.value[{index}].account was not an object"))?;
    assert_required_attributes(
        account_object,
        required_account_attributes,
        &format!("result.value[{index}].account"),
    )?;

    account_object
        .get("executable")
        .and_then(Value::as_bool)
        .with_context(|| format!("result.value[{index}].account.executable was not a boolean"))?;
    account_object
        .get("lamports")
        .and_then(Value::as_u64)
        .with_context(|| format!("result.value[{index}].account.lamports was not a u64"))?;
    account_object
        .get("rentEpoch")
        .and_then(Value::as_u64)
        .with_context(|| format!("result.value[{index}].account.rentEpoch was not a u64"))?;
    account_object
        .get("space")
        .and_then(Value::as_u64)
        .with_context(|| format!("result.value[{index}].account.space was not a u64"))?;

    let account_owner = account_object
        .get("owner")
        .and_then(Value::as_str)
        .with_context(|| format!("result.value[{index}].account.owner was not a string"))?;
    if account_owner != expected_account_owner {
        anyhow::bail!(
            "result.value[{index}].account.owner expected '{}', received '{}'",
            expected_account_owner,
            account_owner
        );
    }

    validate_data(
        index,
        account_object
            .get("data")
            .with_context(|| format!("result.value[{index}].account.data was missing"))?,
        required_token_amount_attributes,
        expected_data_program,
        expected_mint,
        expected_token_owner,
    )
}

fn validate_data(
    index: usize,
    data: &Value,
    required_token_amount_attributes: &[String],
    expected_data_program: &str,
    expected_mint: &str,
    expected_token_owner: &str,
) -> Result<()> {
    let data_object = data
        .as_object()
        .with_context(|| format!("result.value[{index}].account.data was not an object"))?;
    assert_required_literal_attributes(
        data_object,
        &["parsed", "program", "space"],
        &format!("result.value[{index}].account.data"),
    )?;

    let program = data_object
        .get("program")
        .and_then(Value::as_str)
        .with_context(|| format!("result.value[{index}].account.data.program was not a string"))?;
    if program != expected_data_program {
        anyhow::bail!(
            "result.value[{index}].account.data.program expected '{}', received '{}'",
            expected_data_program,
            program
        );
    }
    data_object
        .get("space")
        .and_then(Value::as_u64)
        .with_context(|| format!("result.value[{index}].account.data.space was not a u64"))?;

    let parsed_object = data_object
        .get("parsed")
        .and_then(Value::as_object)
        .with_context(|| format!("result.value[{index}].account.data.parsed was not an object"))?;
    assert_required_literal_attributes(
        parsed_object,
        &["info", "type"],
        &format!("result.value[{index}].account.data.parsed"),
    )?;

    parsed_object
        .get("type")
        .and_then(Value::as_str)
        .with_context(|| {
            format!("result.value[{index}].account.data.parsed.type was not a string")
        })?;

    let info_object = parsed_object
        .get("info")
        .and_then(Value::as_object)
        .with_context(|| {
            format!("result.value[{index}].account.data.parsed.info was not an object")
        })?;
    assert_required_literal_attributes(
        info_object,
        &["isNative", "mint", "owner", "state", "tokenAmount"],
        &format!("result.value[{index}].account.data.parsed.info"),
    )?;

    info_object
        .get("isNative")
        .and_then(Value::as_bool)
        .with_context(|| {
            format!("result.value[{index}].account.data.parsed.info.isNative was not a boolean")
        })?;
    info_object
        .get("state")
        .and_then(Value::as_str)
        .with_context(|| {
            format!("result.value[{index}].account.data.parsed.info.state was not a string")
        })?;

    let mint = info_object
        .get("mint")
        .and_then(Value::as_str)
        .with_context(|| {
            format!("result.value[{index}].account.data.parsed.info.mint was not a string")
        })?;
    if mint != expected_mint {
        anyhow::bail!(
            "result.value[{index}].account.data.parsed.info.mint expected '{}', received '{}'",
            expected_mint,
            mint
        );
    }

    let owner = info_object
        .get("owner")
        .and_then(Value::as_str)
        .with_context(|| {
            format!("result.value[{index}].account.data.parsed.info.owner was not a string")
        })?;
    if owner != expected_token_owner {
        anyhow::bail!(
            "result.value[{index}].account.data.parsed.info.owner expected '{}', received '{}'",
            expected_token_owner,
            owner
        );
    }

    validate_token_amount(
        index,
        info_object.get("tokenAmount").with_context(|| {
            format!("result.value[{index}].account.data.parsed.info.tokenAmount was missing")
        })?,
        required_token_amount_attributes,
    )
}

fn validate_token_amount(
    index: usize,
    token_amount: &Value,
    required_token_amount_attributes: &[String],
) -> Result<()> {
    let token_amount_object = token_amount.as_object().with_context(|| {
        format!("result.value[{index}].account.data.parsed.info.tokenAmount was not an object")
    })?;
    assert_required_attributes(
        token_amount_object,
        required_token_amount_attributes,
        &format!("result.value[{index}].account.data.parsed.info.tokenAmount"),
    )?;

    let amount = token_amount_object
        .get("amount")
        .and_then(Value::as_str)
        .with_context(|| {
            format!(
                "result.value[{index}].account.data.parsed.info.tokenAmount.amount was not a string"
            )
        })?;
    if amount.parse::<u64>().is_err() {
        anyhow::bail!(
            "result.value[{index}].account.data.parsed.info.tokenAmount.amount was not a base-10 u64 string"
        );
    }
    token_amount_object
        .get("decimals")
        .and_then(Value::as_u64)
        .with_context(|| {
            format!(
                "result.value[{index}].account.data.parsed.info.tokenAmount.decimals was not a u64"
            )
        })?;
    let ui_amount = token_amount_object.get("uiAmount").with_context(|| {
        format!("result.value[{index}].account.data.parsed.info.tokenAmount.uiAmount was missing")
    })?;
    if !ui_amount.is_null() && ui_amount.as_f64().is_none() {
        anyhow::bail!(
            "result.value[{index}].account.data.parsed.info.tokenAmount.uiAmount was neither null nor a number"
        );
    }
    token_amount_object
        .get("uiAmountString")
        .and_then(Value::as_str)
        .with_context(|| {
            format!(
                "result.value[{index}].account.data.parsed.info.tokenAmount.uiAmountString was not a string"
            )
        })?;

    Ok(())
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

fn assert_required_literal_attributes(
    object: &serde_json::Map<String, Value>,
    required_attributes: &[&str],
    location: &str,
) -> Result<()> {
    for field_name in required_attributes {
        if !object.contains_key(*field_name) {
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
        MethodExpectation::TokenAccountsByOwner {
            minimum_result_count: 1,
            required_result_attributes: vec!["context".to_string(), "value".to_string()],
            required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
            required_value_entry_attributes: vec!["account".to_string(), "pubkey".to_string()],
            required_account_attributes: vec![
                "data".to_string(),
                "executable".to_string(),
                "lamports".to_string(),
                "owner".to_string(),
                "rentEpoch".to_string(),
                "space".to_string(),
            ],
            required_token_amount_attributes: vec![
                "amount".to_string(),
                "decimals".to_string(),
                "uiAmount".to_string(),
                "uiAmountString".to_string(),
            ],
            expected_account_owner: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            expected_data_program: "spl-token".to_string(),
            expected_mint: "mint-1".to_string(),
            expected_token_owner: "owner-1".to_string(),
        }
    }

    fn valid_result() -> Value {
        json!({
            "context": {
                "apiVersion": "3.1.11",
                "slot": 411329792
            },
            "value": [
                {
                    "account": {
                        "data": {
                            "parsed": {
                                "info": {
                                    "isNative": false,
                                    "mint": "mint-1",
                                    "owner": "owner-1",
                                    "state": "initialized",
                                    "tokenAmount": {
                                        "amount": "47209263",
                                        "decimals": 6,
                                        "uiAmount": 47.209263,
                                        "uiAmountString": "47.209263"
                                    }
                                },
                                "type": "account"
                            },
                            "program": "spl-token",
                            "space": 165
                        },
                        "executable": false,
                        "lamports": 2039280,
                        "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                        "rentEpoch": 18446744073709551615u64,
                        "space": 165
                    },
                    "pubkey": "token-account-1"
                }
            ]
        })
    }

    #[test]
    fn validates_token_accounts_by_owner_shape() {
        let result = validate(&expectation(), &valid_result()).expect("expected success");

        assert_eq!(result, "tokenAccounts=1");
    }

    #[test]
    fn rejects_wrong_mint() {
        let mut result = valid_result();
        result["value"][0]["account"]["data"]["parsed"]["info"]["mint"] = json!("other-mint");

        let error = validate(&expectation(), &result).expect_err("wrong mint should fail");

        assert!(
            error
                .to_string()
                .contains("account.data.parsed.info.mint expected 'mint-1'")
        );
    }
}
