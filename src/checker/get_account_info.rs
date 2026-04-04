use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let (
        required_result_attributes,
        required_context_attributes,
        required_value_attributes,
        expected_value_attributes,
        expected_owner,
        expected_data_encoding,
        expected_parsed_program,
        required_parsed_attributes,
    ) = match expectation {
        MethodExpectation::AccountInfo {
            required_result_attributes,
            required_context_attributes,
            required_value_attributes,
            expected_value_attributes,
            expected_owner,
            expected_data_encoding,
            expected_parsed_program,
            required_parsed_attributes,
        } => (
            required_result_attributes,
            required_context_attributes,
            required_value_attributes,
            expected_value_attributes,
            expected_owner,
            expected_data_encoding,
            expected_parsed_program.as_deref(),
            required_parsed_attributes,
        ),
        other => {
            anyhow::bail!("getAccountInfo expected an accountInfo validator, received {other:?}")
        }
    };

    let result_object = result
        .as_object()
        .context("result field was not an object as required by the getAccountInfo validator")?;
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
        .get("slot")
        .and_then(Value::as_u64)
        .context("result.context.slot was not a u64")?;
    context_object
        .get("apiVersion")
        .and_then(Value::as_str)
        .context("result.context.apiVersion was not a string")?;

    let value_object = result_object
        .get("value")
        .and_then(Value::as_object)
        .context("result.value was not an object")?;
    assert_required_attributes(value_object, required_value_attributes, "result.value")?;
    let expected_value_object = expected_value_attributes
        .as_object()
        .context("accountInfo expected_value_attributes was not an object")?;

    let executable = value_object
        .get("executable")
        .and_then(Value::as_bool)
        .context("result.value.executable was not a boolean")?;
    let expected_executable = expected_value_object
        .get("executable")
        .and_then(Value::as_bool)
        .context("expected_value_attributes.executable was not a boolean")?;
    if executable != expected_executable {
        anyhow::bail!(
            "result.value.executable expected {}, received {}",
            expected_executable,
            executable
        );
    }

    let lamports = value_object
        .get("lamports")
        .and_then(Value::as_u64)
        .context("result.value.lamports was not a u64")?;
    if lamports == 0 {
        anyhow::bail!("result.value.lamports must be greater than 0");
    }

    let owner = value_object
        .get("owner")
        .and_then(Value::as_str)
        .context("result.value.owner was not a string")?;
    if owner != expected_owner {
        anyhow::bail!(
            "result.value.owner expected '{}', received '{}'",
            expected_owner,
            owner
        );
    }

    let rent_epoch = value_object
        .get("rentEpoch")
        .and_then(Value::as_u64)
        .context("result.value.rentEpoch was not a u64")?;
    let expected_rent_epoch = expected_value_object
        .get("rentEpoch")
        .and_then(Value::as_u64)
        .context("expected_value_attributes.rentEpoch was not a u64")?;
    if rent_epoch != expected_rent_epoch {
        anyhow::bail!(
            "result.value.rentEpoch expected {}, received {}",
            expected_rent_epoch,
            rent_epoch
        );
    }

    let space = value_object
        .get("space")
        .and_then(Value::as_u64)
        .context("result.value.space was not a u64")?;
    let expected_space = expected_value_object
        .get("space")
        .and_then(Value::as_u64)
        .context("expected_value_attributes.space was not a u64")?;
    if space != expected_space {
        anyhow::bail!(
            "result.value.space expected {}, received {}",
            expected_space,
            space
        );
    }

    validate_account_data(
        value_object
            .get("data")
            .context("result.value.data was missing")?,
        expected_data_encoding,
        expected_parsed_program,
        required_parsed_attributes,
    )?;

    Ok(format!(
        "space={} encoding={}",
        space, expected_data_encoding
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
    data: &Value,
    expected_data_encoding: &str,
    expected_parsed_program: Option<&str>,
    required_parsed_attributes: &[String],
) -> Result<()> {
    match expected_data_encoding {
        "base58" | "base64" | "base64+zstd" => {
            let data_array = data
                .as_array()
                .context("result.value.data was not an array")?;

            if data_array.len() != 2 {
                anyhow::bail!(
                    "result.value.data expected 2 elements, received {}",
                    data_array.len()
                );
            }

            data_array[0]
                .as_str()
                .context("result.value.data[0] was not a string")?;
            let encoding = data_array[1]
                .as_str()
                .context("result.value.data[1] was not a string")?;

            if encoding != expected_data_encoding {
                anyhow::bail!(
                    "result.value.data[1] expected '{}', received '{}'",
                    expected_data_encoding,
                    encoding
                );
            }
        }
        "jsonParsed" => {
            let data_object = data
                .as_object()
                .context("result.value.data was not an object")?;

            for field_name in ["parsed", "program", "space"] {
                if !data_object.contains_key(field_name) {
                    anyhow::bail!("result.value.data was missing required '{field_name}' field");
                }
            }

            if let Some(expected_program) = expected_parsed_program {
                let actual_program = data_object
                    .get("program")
                    .and_then(Value::as_str)
                    .context("result.value.data.program was not a string")?;
                if actual_program != expected_program {
                    anyhow::bail!(
                        "result.value.data.program expected '{}', received '{}'",
                        expected_program,
                        actual_program
                    );
                }
            }

            data_object
                .get("space")
                .and_then(Value::as_u64)
                .context("result.value.data.space was not a u64")?;

            let parsed_object = data_object
                .get("parsed")
                .and_then(Value::as_object)
                .context("result.value.data.parsed was not an object")?;
            assert_required_attributes(
                parsed_object,
                required_parsed_attributes,
                "result.value.data.parsed",
            )?;

            parsed_object
                .get("type")
                .and_then(Value::as_str)
                .context("result.value.data.parsed.type was not a string")?;
            parsed_object
                .get("info")
                .and_then(Value::as_object)
                .context("result.value.data.parsed.info was not an object")?;
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
    fn validates_base64_account_info() {
        let result = validate(
            &MethodExpectation::AccountInfo {
                required_result_attributes: vec!["context".to_string(), "value".to_string()],
                required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
                required_value_attributes: vec![
                    "data".to_string(),
                    "executable".to_string(),
                    "lamports".to_string(),
                    "owner".to_string(),
                    "rentEpoch".to_string(),
                    "space".to_string(),
                ],
                expected_value_attributes: json!({
                    "executable": false,
                    "rentEpoch": 42,
                    "space": 82
                }),
                expected_owner: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                expected_data_encoding: "base64".to_string(),
                expected_parsed_program: None,
                required_parsed_attributes: Vec::new(),
            },
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 1
                },
                "value": {
                    "data": ["Zm9v", "base64"],
                    "executable": false,
                    "lamports": 123,
                    "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                    "rentEpoch": 42,
                    "space": 82
                }
            }),
        )
        .expect("expected success");

        assert_eq!(result, "space=82 encoding=base64");
    }

    #[test]
    fn validates_json_parsed_account_info() {
        let result = validate(
            &MethodExpectation::AccountInfo {
                required_result_attributes: vec!["context".to_string(), "value".to_string()],
                required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
                required_value_attributes: vec![
                    "data".to_string(),
                    "executable".to_string(),
                    "lamports".to_string(),
                    "owner".to_string(),
                    "rentEpoch".to_string(),
                    "space".to_string(),
                ],
                expected_value_attributes: json!({
                    "executable": false,
                    "rentEpoch": 42,
                    "space": 82
                }),
                expected_owner: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                expected_data_encoding: "jsonParsed".to_string(),
                expected_parsed_program: Some("spl-token".to_string()),
                required_parsed_attributes: vec!["info".to_string(), "type".to_string()],
            },
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 1
                },
                "value": {
                    "data": {
                        "program": "spl-token",
                        "space": 82,
                        "parsed": {
                            "type": "mint",
                            "info": {}
                        }
                    },
                    "executable": false,
                    "lamports": 123,
                    "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                    "rentEpoch": 42,
                    "space": 82
                }
            }),
        )
        .expect("expected success");

        assert_eq!(result, "space=82 encoding=jsonParsed");
    }

    #[test]
    fn rejects_missing_value() {
        let error = validate(
            &MethodExpectation::AccountInfo {
                required_result_attributes: vec!["context".to_string(), "value".to_string()],
                required_context_attributes: vec!["apiVersion".to_string(), "slot".to_string()],
                required_value_attributes: vec![
                    "data".to_string(),
                    "executable".to_string(),
                    "lamports".to_string(),
                    "owner".to_string(),
                    "rentEpoch".to_string(),
                    "space".to_string(),
                ],
                expected_value_attributes: json!({
                    "executable": false,
                    "rentEpoch": 42,
                    "space": 82
                }),
                expected_owner: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                expected_data_encoding: "base58".to_string(),
                expected_parsed_program: None,
                required_parsed_attributes: Vec::new(),
            },
            &json!({
                "context": {
                    "apiVersion": "3.1.11",
                    "slot": 1
                },
                "value": null
            }),
        )
        .expect_err("null value should fail");

        assert!(error.to_string().contains("result.value was not an object"));
    }
}
