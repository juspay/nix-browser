use std::str::FromStr;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use nix_rs::{
    command::NixCmd,
    config::NixConfig,
    flake::{system::System, url::FlakeUrl},
};

use crate::{
    config,
    github::pull_request::{PullRequest, PullRequestRef},
    nix::{
        devour_flake,
        system_list::{SystemsList, SystemsListFlakeRef},
    },
};

/// A reference to some flake living somewhere
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlakeRef {
    /// A github PR
    GithubPR(PullRequestRef),
    /// A flake URL supported by Nix commands
    Flake(FlakeUrl),
}

impl FromStr for FlakeRef {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<FlakeRef, String> {
        let flake_ref = match PullRequestRef::from_web_url(s) {
            Some(pr) => FlakeRef::GithubPR(pr),
            None => FlakeRef::Flake(FlakeUrl(s.to_string())),
        };
        Ok(flake_ref)
    }
}

impl FlakeRef {
    /// Convert the value to a flake URL that Nix command will recognize.
    pub async fn to_flake_url(&self) -> Result<FlakeUrl> {
        match self {
            FlakeRef::GithubPR(pr) => {
                let pr = PullRequest::get(pr).await?;
                Ok(pr.flake_url())
            }
            FlakeRef::Flake(url) => Ok(url.clone()),
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author = "Sridhar Ratnakumar", version, about)]
/// nixci - Define and build CI for Nix projects anywhere <https://github.com/srid/nixci>
pub struct CliArgs {
    /// Whether to be verbose
    ///
    /// If enabled, also the full nix command output is shown.
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Nix command global options
    #[command(flatten)]
    pub nixcmd: NixCmd,

    #[clap(subcommand)]
    pub command: Command,
}

impl CliArgs {
    /// Parse `CliArgs` from command-line args
    pub async fn parse() -> anyhow::Result<Self> {
        let mut args = <Self as Parser>::parse();
        args.preprocess().await?;
        Ok(args)
    }

    // Pre-process `CliArgs`
    pub async fn preprocess(&mut self) -> anyhow::Result<()> {
        // Avoid using `--extra-experimental-features` if possible.
        self.nixcmd.with_flakes();
        // Adjust to devour_flake's expectations
        if let Command::Build(build_cfg) = &mut self.command {
            devour_flake::transform_override_inputs(&mut build_cfg.extra_nix_build_args);
        }
        Ok(())
    }
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Build all outputs of a flake
    Build(BuildConfig),

    /// Print the Github Actions matrix configuration as JSON
    #[clap(name = "gh-matrix")]
    DumpGithubActionsMatrix {
        /// Flake URL or github URL
        ///
        /// A specific nixci configuration can be specified
        /// using '#': e.g. `nixci .#extra-tests`
        #[arg(default_value = ".")]
        flake_ref: FlakeRef,

        /// Systems to include in the matrix
        #[arg(long, value_parser, value_delimiter = ',')]
        systems: Vec<System>,
    },

    /// Generates shell completion scripts
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

impl Command {
    /// Get the nixci [config::Config] associated with this subcommand
    pub async fn get_config(cmd: &NixCmd, flake_ref: &FlakeRef) -> anyhow::Result<config::Config> {
        let url = flake_ref.to_flake_url().await?;
        tracing::info!("{}", format!("🍏 Building {}", url.0).bold());
        let cfg = config::Config::from_flake_url(cmd, &url).await?;
        tracing::debug!("Config: {cfg:?}");
        Ok(cfg)
    }
}

#[derive(Parser, Debug, Clone)]
pub struct BuildConfig {
    /// The systems list to build for. If empty, build for current system.
    ///
    /// Must be a flake reference which, when imported, must return a Nix list
    /// of systems. You may use one of the lists from
    /// <https://github.com/nix-systems>.
    #[arg(long, default_value = "github:nix-systems/empty")]
    pub systems: SystemsListFlakeRef,

    /// Flake URL or github URL
    ///
    /// A specific nixci` configuration can be specified
    /// using '#': e.g. `nixci .#extra-tests`
    #[arg(default_value = ".")]
    pub flake_ref: FlakeRef,

    /// Additional arguments to pass through to `nix build`
    #[arg(last = true, default_values_t = vec![
    "--refresh".to_string(),
    "-j".to_string(),
    "auto".to_string(),
    ])]
    pub extra_nix_build_args: Vec<String>,

    /// Print build and runtime dependencies along with out paths
    ///
    /// By default, `nixci build` prints only the out paths. This option is
    /// useful to explicitly push all dependencies to a cache.
    #[clap(long, short = 'd')]
    pub print_all_dependencies: bool,

    /// Run om ci build remotely
    #[clap(long)]
    pub on: Option<String>,
}

impl BuildConfig {
    pub async fn get_systems(&self, cmd: &NixCmd, nix_config: &NixConfig) -> Result<Vec<System>> {
        let systems = SystemsList::from_flake(cmd, &self.systems).await?.0;
        if systems.is_empty() {
            let current_system = &nix_config.system.value;
            Ok(vec![current_system.clone()])
        } else {
            Ok(systems)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_pr() {
        assert_eq!(
            FlakeRef::from_str("https://github.com/srid/nixci/pull/19").unwrap(),
            FlakeRef::GithubPR(PullRequestRef {
                owner: "srid".to_string(),
                repo: "nixci".to_string(),
                pr: 19
            })
        );
    }

    #[test]
    fn test_current_dir() {
        assert_eq!(
            FlakeRef::from_str(".").unwrap(),
            FlakeRef::Flake(FlakeUrl(".".to_string()))
        );
    }

    #[test]
    fn test_flake_url() {
        assert_eq!(
            FlakeRef::from_str("github:srid/nixci").unwrap(),
            FlakeRef::Flake(FlakeUrl("github:srid/nixci".to_string()))
        );
    }
}
