use std::path::Path;

use anyhow::Context;
use colored::Colorize;
use nix_health::{traits::CheckResult, NixHealth};
use nix_rs::{command::NixCmd, env::NixEnv, flake::eval::nix_eval_attr_json, info::NixInfo};
use serde::de::DeserializeOwned;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    human_panic::setup_panic!();
    let nix_info = NixInfo::from_nix(&NixCmd::default())
        .await
        .with_context(|| "Unable to gather nix info")?;
    let nix_env = NixEnv::detect()
        .await
        .with_context(|| "Unable to gather system info")?;
    let health: NixHealth = get_config().await?;
    let checks = &health.run_checks(&nix_info, &nix_env);
    println!("Checking the health of your Nix setup:\n");
    for check in checks {
        match &check.result {
            CheckResult::Green => {
                println!("{}", format!("✅ {}", check.title).green().bold());
                println!("   {}", check.info.blue());
            }
            CheckResult::Red { msg, suggestion } => {
                println!("{}", format!("❌ {}", check.title).red().bold());
                println!("   {}", check.info.blue());
                println!("   {}", msg.yellow());
                println!("   {}", suggestion);
            }
        }
        println!();
    }
    if checks
        .iter()
        .any(|c| matches!(c.result, CheckResult::Red { .. }))
    {
        println!("{}", "!! Some checks failed (see above)".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "✅ All checks passed".green().bold());
        Ok(())
    }
}

async fn get_config<T>() -> anyhow::Result<T>
where
    T: Default + DeserializeOwned,
{
    if Path::new("flake.nix").exists() {
        let v = nix_eval_attr_json(".#nix-health.default".into()).await?;
        Ok(v)
    } else {
        Ok(T::default())
    }
}
