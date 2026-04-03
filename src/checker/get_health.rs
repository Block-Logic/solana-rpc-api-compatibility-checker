use crate::fixture::MethodExpectation;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn validate(expectation: &MethodExpectation, result: &Value) -> Result<String> {
    let allowed_values = match expectation {
        MethodExpectation::StringResult { allowed_values } => allowed_values,
        other => anyhow::bail!("getHealth expected a stringResult validator, received {other:?}"),
    };

    let actual = result
        .as_str()
        .context("result field was not a string as required by the getHealth validator")?;

    if !allowed_values.iter().any(|value| value == actual) {
        anyhow::bail!(
            "expected result to be one of {:?}, received '{}'",
            allowed_values,
            actual
        );
    }

    Ok(format!("result='{}'", actual))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_allowed_string_result() {
        let expectation = MethodExpectation::StringResult {
            allowed_values: vec!["ok".to_string()],
        };

        let result =
            validate(&expectation, &Value::String("ok".to_string())).expect("expected success");

        assert_eq!(result, "result='ok'");
    }
}
