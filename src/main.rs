mod checker;
mod config;
mod fixture;

use anyhow::Result;
use checker::run_checks;
use config::Config;
use fixture::load_rpc_fixtures;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let fixtures = load_rpc_fixtures("fixtures/rpc")?;

    if fixtures.is_empty() {
        anyhow::bail!("no RPC fixtures were found in fixtures/rpc");
    }

    let report = run_checks(&config, &fixtures).await?;
    report.print_summary();

    if report.has_failures() {
        anyhow::bail!("one or more compatibility checks failed");
    }

    Ok(())
}
