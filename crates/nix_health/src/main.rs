use anyhow::Context;
use colored::Colorize;
use nix_health::{report::Report, traits::Check, NixHealth};
use nix_rs::{command::NixCmd, info::NixInfo};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    human_panic::setup_panic!();
    let info = NixInfo::from_nix(&NixCmd::default())
        .await
        .with_context(|| "Unable to gather nix info")?;
    let health = NixHealth::check(&info);
    println!("Checking the health of your Nix setup:\n");
    for check in &health {
        let report = check.report();
        match report {
            Report::Green => {
                println!("{}", format!("✅ {}", check.name()).green().bold());
                println!("   {}", check.information().blue());
            }
            Report::Red(details) => {
                println!("{}", format!("❌ {}", check.name()).red().bold());
                println!("   {}", check.information().blue());
                println!("   {}", details.msg.yellow());
                println!("   {}", details.suggestion);
            }
        }
        println!();
    }
    if health.into_iter().any(|c| c.report().is_red()) {
        println!("{}", "!! Some checks failed (see above)".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "✅ All checks passed".green().bold());
        Ok(())
    }
}
