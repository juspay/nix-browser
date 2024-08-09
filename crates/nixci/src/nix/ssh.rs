use crate::cli;

use super::nix_store::StorePath;
use anyhow::Context;
use tokio::process::Command;

use cli::BuildConfig;

/// Runs `nix run command through ssh on remote machine` in Rust
///
pub async fn nix_run_on_ssh(
    build_cfg: &BuildConfig,
    remote_address: &str,
    omnix_input: &str,
    flake_url: &str,
) -> anyhow::Result<Vec<StorePath>> {
    let mut cmd = Command::new("ssh");

    // Add the remote address
    cmd.arg(remote_address);

    // Construct the base nix run command
    let mut nix_cmd = vec![
        "nix run".to_string(),
        format!("{}#default", omnix_input),
        "--".to_string(),
        "ci".to_string(),
        "build".to_string(),
        flake_url.to_string(),
    ];

    // Add print-all-dependencies flag if necessary
    if build_cfg.print_all_dependencies {
        nix_cmd.push("--print-all-dependencies".to_string());
    }

    // Add extra nix build arguments
    nix_cmd.push("--".to_string());
    nix_cmd.extend(build_cfg.extra_nix_build_args.iter().cloned());

    // Join all arguments into a single string
    let args = nix_cmd.join(" ");

    // Add the nix command arguments to the base ssh-command
    cmd.arg(args);

    nix_rs::command::trace_cmd(&cmd);

    let status = cmd
        .status()
        .await
        .context("Failed to execute SSH command")?;
    if status.success() {
        Ok(Vec::new())
    } else {
        let exit_code = status.code().unwrap_or(1);
        anyhow::bail!("SSH command failed with exit code: {}", exit_code)
    }
}
