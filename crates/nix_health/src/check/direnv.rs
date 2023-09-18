use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use crate::traits::{Check, CheckResult, Checkable};
#[cfg(feature = "ssr")]
use nix_rs::{env, info};

/// Check if direnv is in use
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Direnv {
    enable: bool,
}

#[cfg(feature = "ssr")]
impl Checkable for Direnv {
    fn check(&self, _nix_info: &info::NixInfo, nix_env: &env::NixEnv) -> Option<Check> {
        if !self.enable {
            return None;
        }
        // This check is currently only relevant if the flake is local
        // TODO: direnv check should still happen for non-flakes, if only to
        // test that `direnv` is installed and working (if not activated on a
        // local project).
        let local_path = nix_env
            .current_flake
            .as_ref()
            .and_then(|url| url.as_local_path())?;
        let suggestion = format!("Install direnv <https://zero-to-flakes.com/direnv/#setup> and run `direnv allow` under `{}`", local_path.display());
        let direnv_install = DirenvInstall::detect();
        let check = Check {
            title: "Direnv activated".to_string(),
            // TODO: Show direnv path
            info: format!(
                "Local flake: {:?}; direnv install = {:?}",
                local_path, direnv_install
            ),
            result: match direnv_active(local_path) {
                Ok(true) => CheckResult::Green,
                Ok(false) => CheckResult::Red {
                    msg: "direnv is not active".to_string(),
                    suggestion,
                },
                Err(e) => CheckResult::Red {
                    msg: format!("Unable to check direnv status: {}", e),
                    suggestion,
                },
            },
        };
        Some(check)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DirenvInstall {
    /// Path to the direnv binary
    pub bin_path: PathBuf,

    /// Contents of `direnvrc`
    pub direnv_config: Option<String>,

    // bash_path used by direnv
    pub bash_path: Option<PathBuf>,
}

impl DirenvInstall {
    #[cfg(feature = "ssr")]
    pub fn detect() -> anyhow::Result<Self> {
        let bin_path = which::which("direnv")?;
        let output = std::process::Command::new(&bin_path)
            .arg("status")
            .output()?;
        let out = String::from_utf8_lossy(&output.stdout);
        let mut bash_path = None;
        let mut direnv_config = None;
        for line in out.lines() {
            if let Some(path) = line.strip_prefix("bash_path ") {
                bash_path = Some(PathBuf::from(path));
            }
            if let Some(config_dir) = line.strip_prefix("DIRENV_CONFIG ") {
                let config_file = PathBuf::from(config_dir).join("direnvrc");
                // Read config_file and assign to direnv_config
                if config_file.exists() {
                    let config = std::fs::read_to_string(config_file)?;
                    direnv_config = Some(config);
                }
            }
        }
        Ok(Self {
            bin_path,
            direnv_config,
            bash_path,
        })
    }
}

/// Check if direnv was already activated in [project_dir]
#[cfg(feature = "ssr")]
pub fn direnv_active(project_dir: &std::path::Path) -> anyhow::Result<bool> {
    let cmd = "direnv status | grep 'Found RC allowed true'";
    // TODO: Don't use `sh`
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(project_dir)
        .output()?;
    Ok(output.status.success())
}