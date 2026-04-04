mod checker;
mod config;
mod fixture;

use anyhow::Result;
use checker::{run_checks, run_checks_with_options};
use config::Config;
use fixture::{RpcFixture, load_rpc_fixtures};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    method: Option<String>,
    show_failure_response: bool,
}

impl CliArgs {
    fn parse() -> Result<Self> {
        Self::parse_from(std::env::args().skip(1))
    }

    fn parse_from(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut method = None;
        let mut show_failure_response = false;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--method" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--method requires a value"))?;
                    method = Some(value);
                }
                "--show-failure-response" => {
                    show_failure_response = true;
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => anyhow::bail!("unrecognized argument '{other}'"),
            }
        }

        Ok(Self {
            method,
            show_failure_response,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli_args = CliArgs::parse()?;
    let config = Config::from_env()?;
    let fixtures = select_fixtures(
        load_rpc_fixtures("fixtures/rpc")?,
        cli_args.method.as_deref(),
    )?;

    if fixtures.is_empty() {
        anyhow::bail!("no RPC fixtures were found in fixtures/rpc");
    }

    let report = if cli_args.show_failure_response {
        run_checks_with_options(&config, &fixtures, true).await?
    } else {
        run_checks(&config, &fixtures).await?
    };
    report.print_summary();

    if report.has_failures() {
        anyhow::bail!("one or more compatibility checks failed");
    }

    Ok(())
}

fn select_fixtures(fixtures: Vec<RpcFixture>, method: Option<&str>) -> Result<Vec<RpcFixture>> {
    let Some(method) = method else {
        return Ok(fixtures);
    };

    let filtered = fixtures
        .into_iter()
        .filter(|fixture| fixture.method == method)
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        anyhow::bail!("no fixtures were found for method '{method}'");
    }

    Ok(filtered)
}

fn print_usage() {
    println!("Usage: cargo run -- [--method <rpc-method>] [--show-failure-response]");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_method_argument() {
        let args = CliArgs::parse_from(vec!["--method".to_string(), "getBlock".to_string()])
            .expect("expected parse success");

        assert_eq!(
            args,
            CliArgs {
                method: Some("getBlock".to_string()),
                show_failure_response: false,
            }
        );
    }

    #[test]
    fn parses_show_failure_response_flag() {
        let args = CliArgs::parse_from(vec![
            "--method".to_string(),
            "getProgramAccounts".to_string(),
            "--show-failure-response".to_string(),
        ])
        .expect("expected parse success");

        assert_eq!(
            args,
            CliArgs {
                method: Some("getProgramAccounts".to_string()),
                show_failure_response: true,
            }
        );
    }

    #[test]
    fn rejects_missing_method_value() {
        let error = CliArgs::parse_from(vec!["--method".to_string()])
            .expect_err("missing method value should fail");

        assert!(error.to_string().contains("--method requires a value"));
    }

    #[test]
    fn filters_fixtures_by_method() {
        let fixtures = vec![
            RpcFixture {
                name: "health".to_string(),
                method: "getHealth".to_string(),
                request: fixture::RequestFixture { params: Vec::new() },
                expectation: fixture::ResponseExpectation {
                    transport: fixture::TransportExpectation {
                        content_type_prefix: "application/json".to_string(),
                        charset: "utf-8".to_string(),
                    },
                    envelope: fixture::JsonRpcEnvelopeExpectation {
                        jsonrpc_version: "2.0".to_string(),
                        required_attributes: vec![
                            "jsonrpc".to_string(),
                            "result".to_string(),
                            "id".to_string(),
                        ],
                        allow_error: false,
                        expected_error: None,
                    },
                    validator: fixture::MethodExpectation::StringResult {
                        allowed_values: vec!["ok".to_string()],
                    },
                },
            },
            RpcFixture {
                name: "epoch".to_string(),
                method: "getEpochInfo".to_string(),
                request: fixture::RequestFixture { params: Vec::new() },
                expectation: fixture::ResponseExpectation {
                    transport: fixture::TransportExpectation {
                        content_type_prefix: "application/json".to_string(),
                        charset: "utf-8".to_string(),
                    },
                    envelope: fixture::JsonRpcEnvelopeExpectation {
                        jsonrpc_version: "2.0".to_string(),
                        required_attributes: vec![
                            "jsonrpc".to_string(),
                            "result".to_string(),
                            "id".to_string(),
                        ],
                        allow_error: false,
                        expected_error: None,
                    },
                    validator: fixture::MethodExpectation::EpochInfo {
                        required_result_attributes: vec!["epoch".to_string()],
                    },
                },
            },
        ];

        let filtered =
            select_fixtures(fixtures, Some("getEpochInfo")).expect("expected filter success");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].method, "getEpochInfo");
    }
}
