pub mod cli;
pub mod config;
pub mod github;
pub mod nix;

use anyhow::Context;
use clap::CommandFactory;
use clap_complete::generate;
use std::collections::HashSet;
use std::io;

use cli::{BuildConfig, CliArgs, Command};
use colored::Colorize;
use nix::{
    devour_flake::DevourFlakeOutput,
    nix_store::{DrvOut, NixStoreCmd, StorePath},
};
use nix_health::{traits::Checkable, NixHealth};
use nix_rs::{command::NixCmd, config::NixConfig, flake::url::FlakeUrl, info::NixInfo};
use tracing::instrument;

/// Run nixci on the given [CliArgs], returning the built outputs in sorted order.
#[instrument(name = "nixci", skip(command))]
pub async fn nixci(
    nixcmd: &NixCmd,
    command: &Command,
    verbose: bool,
) -> anyhow::Result<Vec<StorePath>> {
    match command {
        cli::Command::Build(build_cfg) => {
            let cfg = cli::Command::get_config(nixcmd, &build_cfg.flake_ref).await?;
            let nix_config = NixConfig::get().await.as_ref()?;
            let nix_info = NixInfo::new(nix_config.clone())
                .await
                .with_context(|| "Unable to gather nix info")?;
            match &build_cfg.on {
                Some(host) => remote_build(nixcmd, &build_cfg, &cfg, host).await,
                None => {
                    // First, run the necessary health checks
                    check_nix_version(&cfg.flake_url, &nix_info).await?;
                    // Then, do the build
                    nixci_build(nixcmd, verbose, build_cfg, &cfg, &nix_info.nix_config).await
                }
            }
        }
        cli::Command::DumpGithubActionsMatrix {
            systems, flake_ref, ..
        } => {
            let cfg = cli::Command::get_config(nixcmd, flake_ref).await?;
            let matrix = github::matrix::GitHubMatrix::from(systems.clone(), &cfg.subflakes);
            println!("{}", serde_json::to_string(&matrix)?);
            Ok(vec![])
        }
        cli::Command::Completion { shell } => {
            let mut cli = CliArgs::command();
            let name = cli.get_name().to_string();
            generate(*shell, &mut cli, name, &mut io::stdout());
            Ok(vec![])
        }
    }
}

async fn remote_build(
    cmd: &NixCmd,
    build_cfg: &BuildConfig,
    cfg: &config::Config,
    host: &String,
) -> anyhow::Result<Vec<StorePath>> {
    let omnix_input = format!("{}", env!("OMNIX"));

    let metadata = nix_rs::flake::metadata::get_flake_metadata_json(cmd, &cfg.flake_url.0).await?;

    nix_rs::copy::run_nix_copy(
        cmd,
        &host,
        &omnix_input,
        &metadata.path,
        &build_cfg.extra_nix_build_args,
    )
    .await?;

    let result = nix::ssh::nix_run_on_ssh(&build_cfg, &host, &omnix_input, &metadata.path).await?;
    Ok(result)
}

async fn nixci_build(
    cmd: &NixCmd,
    verbose: bool,
    build_cfg: &BuildConfig,
    cfg: &config::Config,
    nix_config: &NixConfig,
) -> anyhow::Result<Vec<StorePath>> {
    let mut all_outs = HashSet::new();

    let all_devour_flake_outs = nixci_subflakes(cmd, verbose, build_cfg, cfg, nix_config).await?;

    if build_cfg.print_all_dependencies {
        let all_deps = NixStoreCmd
            .fetch_all_deps(all_devour_flake_outs.into_iter().collect())
            .await?;
        all_outs.extend(all_deps.into_iter());
    } else {
        let store_paths: HashSet<StorePath> = all_devour_flake_outs
            .into_iter()
            .map(DrvOut::as_store_path)
            .collect();
        all_outs.extend(store_paths);
    }

    for out in &all_outs {
        println!("{}", out);
    }

    Ok(all_outs.into_iter().collect())
}

async fn nixci_subflakes(
    cmd: &NixCmd,
    verbose: bool,
    build_cfg: &BuildConfig,
    cfg: &config::Config,
    nix_config: &NixConfig,
) -> anyhow::Result<HashSet<DrvOut>> {
    let mut result = HashSet::new();
    let systems = build_cfg.get_systems(cmd, nix_config).await?;

    for (subflake_name, subflake) in &cfg.subflakes.0 {
        let name = format!("{}.{}", cfg.name, subflake_name).italic();
        if cfg
            .selected_subflake
            .as_ref()
            .is_some_and(|s| s != subflake_name)
        {
            tracing::info!("🍊 {} {}", name, "skipped (deselected out)".dimmed());
            continue;
        }
        tracing::info!("🍎 {}", name);
        if subflake.can_build_on(&systems) {
            let outs = nixci_subflake(
                cmd,
                verbose,
                build_cfg,
                &cfg.flake_url,
                subflake_name,
                subflake,
            )
            .await?;
            result.extend(outs.0);
        } else {
            tracing::info!(
                "🍊 {} {}",
                name,
                "skipped (cannot build on this system)".dimmed()
            );
        }
    }

    Ok(result)
}

#[instrument(skip(build_cfg, url))]
async fn nixci_subflake(
    cmd: &NixCmd,
    verbose: bool,
    build_cfg: &BuildConfig,
    url: &FlakeUrl,
    subflake_name: &str,
    subflake: &config::SubFlakish,
) -> anyhow::Result<DevourFlakeOutput> {
    if subflake.override_inputs.is_empty() {
        nix::lock::nix_flake_lock_check(cmd, &url.sub_flake_url(subflake.dir.clone())).await?;
    }

    let nix_args = subflake.nix_build_args_for_flake(build_cfg, url);
    let outs = nix::devour_flake::devour_flake(cmd, verbose, nix_args).await?;
    Ok(outs)
}

pub async fn check_nix_version(flake_url: &FlakeUrl, nix_info: &NixInfo) -> anyhow::Result<()> {
    let nix_health = NixHealth::from_flake(flake_url).await?;
    let checks = nix_health.nix_version.check(nix_info, Some(flake_url));
    let exit_code = NixHealth::print_report_returning_exit_code(&checks);

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}
