use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (
        minimum_result_count,
        required_result_attributes,
        required_account_attributes,
        expected_owner,
        expected_data_encoding,
        expected_parsed_program,
        required_parsed_attributes,
    ) = match expectation {
        MethodExpectation::ProgramAccounts {
            minimum_result_count,
            required_result_attributes,
            required_account_attributes,
            expected_owner,
            expected_data_encoding,
            expected_parsed_program,
            required_parsed_attributes,
        } => (
            *minimum_result_count,
            required_result_attributes,
            required_account_attributes,
            expected_owner,
            expected_data_encoding,
            expected_parsed_program.as_deref(),
            required_parsed_attributes,
        ),
        other => anyhow::bail!(
            "getProgramAccounts expected a programAccounts validator, received {other:?}"
        ),
    };

    let entries = result
        .as_array()
        .context("result field was not an array as required by the getProgramAccounts validator")?;

    if entries.len() < minimum_result_count {
        anyhow::bail!(
            "result array length {} was smaller than the required minimum {}",
            entries.len(),
            minimum_result_count
        );
    }

    for (index, entry) in entries.iter().enumerate() {
        let entry_object = entry
            .as_object()
            .with_context(|| format!("result[{index}] was not an object"))?;

        assert_required_attributes(
            entry_object,
            required_result_attributes,
            &format!("result[{index}]"),
        )?;

        entry_object
            .get("pubkey")
            .and_then(Value::as_str)
            .with_context(|| format!("result[{index}].pubkey was not a string"))?;

        let account_object = entry_object
            .get("account")
            .and_then(Value::as_object)
            .with_context(|| format!("result[{index}].account was not an object"))?;

        assert_required_attributes(
            account_object,
            required_account_attributes,
            &format!("result[{index}].account"),
        )?;

        account_object
            .get("executable")
            .and_then(Value::as_bool)
            .with_context(|| format!("result[{index}].account.executable was not a boolean"))?;
        account_object
            .get("lamports")
            .and_then(Value::as_u64)
            .with_context(|| format!("result[{index}].account.lamports was not a u64"))?;

        let owner = account_object
            .get("owner")
            .and_then(Value::as_str)
            .with_context(|| format!("result[{index}].account.owner was not a string"))?;
        if owner != expected_owner {
            anyhow::bail!(
                "result[{index}].account.owner expected '{}', received '{}'",
                expected_owner,
                owner
            );
        }

        account_object
            .get("rentEpoch")
            .and_then(Value::as_u64)
            .with_context(|| format!("result[{index}].account.rentEpoch was not a u64"))?;
        account_object
            .get("space")
            .and_then(Value::as_u64)
            .with_context(|| format!("result[{index}].account.space was not a u64"))?;

        validate_account_data(
            index,
            account_object
                .get("data")
                .with_context(|| format!("result[{index}].account.data was missing"))?,
            expected_data_encoding,
            expected_parsed_program,
            required_parsed_attributes,
        )?;
    }

    Ok(format!(
        "accounts={} encoding={}",
        entries.len(),
        expected_data_encoding
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

fn validate_account_data(
    index: usize,
    data: &Value,
    expected_data_encoding: &str,
    expected_parsed_program: Option<&str>,
    required_parsed_attributes: &[String],
) -> Result<()> {
    match expected_data_encoding {
        "base64" | "base64+zstd" => {
            let data_array = data
                .as_array()
                .with_context(|| format!("result[{index}].account.data was not an array"))?;

            if data_array.len() != 2 {
                anyhow::bail!(
                    "result[{index}].account.data expected 2 elements, received {}",
                    data_array.len()
                );
            }

            data_array[0]
                .as_str()
                .with_context(|| format!("result[{index}].account.data[0] was not a string"))?;
            let encoding = data_array[1]
                .as_str()
                .with_context(|| format!("result[{index}].account.data[1] was not a string"))?;

            if encoding != expected_data_encoding {
                anyhow::bail!(
                    "result[{index}].account.data[1] expected '{}', received '{}'",
                    expected_data_encoding,
                    encoding
                );
            }
        }
        "jsonParsed" => {
            let data_object = data
                .as_object()
                .with_context(|| format!("result[{index}].account.data was not an object"))?;

            for field_name in ["parsed", "program", "space"] {
                if !data_object.contains_key(field_name) {
                    anyhow::bail!(
                        "result[{index}].account.data was missing required '{field_name}' field"
                    );
                }
            }

            if let Some(expected_program) = expected_parsed_program {
                let actual_program = data_object
                    .get("program")
                    .and_then(Value::as_str)
                    .with_context(|| {
                        format!("result[{index}].account.data.program was not a string")
                    })?;
                if actual_program != expected_program {
                    anyhow::bail!(
                        "result[{index}].account.data.program expected '{}', received '{}'",
                        expected_program,
                        actual_program
                    );
                }
            }

            data_object
                .get("space")
                .and_then(Value::as_u64)
                .with_context(|| format!("result[{index}].account.data.space was not a u64"))?;

            let parsed_object = data_object
                .get("parsed")
                .and_then(Value::as_object)
                .with_context(|| {
                    format!("result[{index}].account.data.parsed was not an object")
                })?;

            assert_required_attributes(
                parsed_object,
                required_parsed_attributes,
                &format!("result[{index}].account.data.parsed"),
            )?;

            parsed_object
                .get("type")
                .and_then(Value::as_str)
                .with_context(|| {
                    format!("result[{index}].account.data.parsed.type was not a string")
                })?;
            parsed_object
                .get("info")
                .and_then(Value::as_object)
                .with_context(|| {
                    format!("result[{index}].account.data.parsed.info was not an object")
                })?;
        }
        other => anyhow::bail!("unsupported expected_data_encoding '{other}'"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_base64_program_accounts() {
        let result = validate(
            &MethodExpectation::ProgramAccounts {
                minimum_result_count: 1,
                required_result_attributes: vec!["pubkey".to_string(), "account".to_string()],
                required_account_attributes: vec![
                    "data".to_string(),
                    "executable".to_string(),
                    "lamports".to_string(),
                    "owner".to_string(),
                    "rentEpoch".to_string(),
                    "space".to_string(),
                ],
                expected_owner: "Stake11111111111111111111111111111111111111".to_string(),
                expected_data_encoding: "base64".to_string(),
                expected_parsed_program: None,
                required_parsed_attributes: Vec::new(),
            },
            &json!([{
                "pubkey": "abc",
                "account": {
                    "data": ["Zm9v", "base64"],
                    "executable": false,
                    "lamports": 123,
                    "owner": "Stake11111111111111111111111111111111111111",
                    "rentEpoch": 42,
                    "space": 200
                }
            }]),
        )
        .expect("expected success");

        assert_eq!(result, "accounts=1 encoding=base64");
    }

    #[test]
    fn validates_json_parsed_program_accounts() {
        let result = validate(
            &MethodExpectation::ProgramAccounts {
                minimum_result_count: 1,
                required_result_attributes: vec!["pubkey".to_string(), "account".to_string()],
                required_account_attributes: vec![
                    "data".to_string(),
                    "executable".to_string(),
                    "lamports".to_string(),
                    "owner".to_string(),
                    "rentEpoch".to_string(),
                    "space".to_string(),
                ],
                expected_owner: "Stake11111111111111111111111111111111111111".to_string(),
                expected_data_encoding: "jsonParsed".to_string(),
                expected_parsed_program: Some("stake".to_string()),
                required_parsed_attributes: vec!["info".to_string(), "type".to_string()],
            },
            &json!([{
                "pubkey": "abc",
                "account": {
                    "data": {
                        "program": "stake",
                        "space": 200,
                        "parsed": {
                            "type": "delegated",
                            "info": {}
                        }
                    },
                    "executable": false,
                    "lamports": 123,
                    "owner": "Stake11111111111111111111111111111111111111",
                    "rentEpoch": 42,
                    "space": 200
                }
            }]),
        )
        .expect("expected success");

        assert_eq!(result, "accounts=1 encoding=jsonParsed");
    }

    #[test]
    fn rejects_empty_result_array() {
        let error = validate(
            &MethodExpectation::ProgramAccounts {
                minimum_result_count: 1,
                required_result_attributes: vec!["pubkey".to_string(), "account".to_string()],
                required_account_attributes: vec![
                    "data".to_string(),
                    "executable".to_string(),
                    "lamports".to_string(),
                    "owner".to_string(),
                    "rentEpoch".to_string(),
                    "space".to_string(),
                ],
                expected_owner: "Stake11111111111111111111111111111111111111".to_string(),
                expected_data_encoding: "base64+zstd".to_string(),
                expected_parsed_program: None,
                required_parsed_attributes: Vec::new(),
            },
            &json!([]),
        )
        .expect_err("empty results should fail");

        assert!(
            error
                .to_string()
                .contains("result array length 0 was smaller than the required minimum 1")
        );
    }
}
