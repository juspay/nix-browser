use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use semver::Version;

use crate::traits::{Check, CheckResult, Checkable};

use nix_rs::{flake::url::FlakeUrl, info};

/// Check if direnv is installed
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct Direnv {
    pub(crate) enable: bool,
    /// Whether to produce [Check::required] checks
    pub(crate) required: bool,
}

impl Default for Direnv {
    fn default() -> Self {
        Self {
            enable: true,
            required: false,
        }
    }
}

impl Checkable for Direnv {
    fn check(&self, _nix_info: &info::NixInfo, flake_url: Option<FlakeUrl>) -> Vec<Check> {
        let mut checks = vec![];
        if !self.enable {
            return checks;
        }

        let direnv_version_check = version_check();
        let direnv_version_green = direnv_version_check.result.green();
        checks.push(direnv_version_check);
        if !direnv_version_green {
            return checks;
        }

        let direnv_install_check = install_check(self.required);
        let direnv_installed = direnv_install_check.result.green();
        checks.push(direnv_install_check);

        if direnv_installed {
            // This check is currently only relevant if the flake is local and an `.envrc` exists.
            match flake_url.as_ref().and_then(|url| url.as_local_path()) {
                None => {}
                Some(local_path) => {
                    if local_path.join(".envrc").exists() {
                        checks.push(activation_check(local_path, self.required));
                    }
                }
            }
        }

        checks
    }
}

/// [Check] that direnv was installed.

fn install_check(required: bool) -> Check {
    let suggestion = "Install direnv <https://nixos.asia/en/direnv#setup>".to_string();
    let direnv_status = DirenvStatus::detect();
    Check {
        title: "Direnv installation".to_string(),
        info: format!(
            "direnv installed at = {:?}",
            direnv_status.as_ref().map(|s| &s.config.self_path)
        ),
        result: match direnv_status {
            Ok(_direnv_status) => CheckResult::Green,
            Err(e) => CheckResult::Red {
                msg: format!("Unable to locate direnv: {}", e),
                suggestion,
            },
        },
        required,
    }
}

/// [Check] that direnv version >= 2.33.0 for `direnv status --json` support

fn version_check() -> Check {
    let suggestion = "Upgrade direnv to >= 2.33.0".to_string();
    let direnv_version = direnv_version();
    let parsed_direnv_version = direnv_version
        .as_ref()
        .map(|version| Version::parse(version).ok());
    Check {
        title: "Direnv version".to_string(),
        info: format!("direnv version = {:?}", direnv_version),
        // Use semver to compare versions
        result: match parsed_direnv_version {
            Ok(Some(version)) if version >= Version::parse("2.33.0").unwrap() => CheckResult::Green,
            Ok(Some(version)) => CheckResult::Red {
                msg: format!("direnv version {} is not supported", version),
                suggestion,
            },
            Ok(None) => CheckResult::Red {
                msg: "Unable to parse direnv version".to_string(),
                suggestion,
            },
            Err(e) => CheckResult::Red {
                msg: format!("Unable to locate direnv: {}", e),
                suggestion,
            },
        },
        required: false,
    }
}

/// [Check] that direnv was activated on the local flake

fn activation_check(local_flake: &std::path::Path, required: bool) -> Check {
    let suggestion = format!("Run `direnv allow` under `{}`", local_flake.display());
    Check {
        title: "Direnv allowed".to_string(),
        info: format!("Local flake: {:?} (has .envrc and is allowed)", local_flake),
        result: match is_direnv_allowed_on(local_flake) {
            Ok(true) => CheckResult::Green,
            Ok(false) => CheckResult::Red {
                msg: "direnv was not allowed on this project".to_string(),
                suggestion,
            },
            Err(e) => CheckResult::Red {
                msg: format!("Unable to check direnv status: {}", e),
                suggestion,
            },
        },
        required,
    }
}

/// [Check] if direnv was already allowed in [project_dir]

fn is_direnv_allowed_on(project_dir: &std::path::Path) -> anyhow::Result<bool> {
    let output = std::process::Command::new("direnv")
        .args(["status", "--json"])
        .current_dir(project_dir)
        .output()?;
    if output.status.success() {
        let out = String::from_utf8_lossy(&output.stdout);
        let status = DirenvStatus::from_json(&out)?;
        Ok(status.state.is_found_rc_allowed())
    } else {
        anyhow::bail!("Unable to run direnv status --json: {:?}", output.stderr)
    }
}

fn direnv_version() -> anyhow::Result<String> {
    let output = std::process::Command::new("direnv")
        .args(["--version"])
        .output()?;
    if output.status.success() {
        let out = String::from_utf8_lossy(&output.stdout);
        let trimmed_out = out.trim();
        Ok(trimmed_out.to_string())
    } else {
        anyhow::bail!("Unable to run direnv --version: {:?}", output.stderr)
    }
}

/// Information about the direnv status
#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirenvStatus {
    config: DirenvConfig,
    state: DirenvState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirenvConfig {
    /// Path to the config folder of direnv
    #[serde(rename = "ConfigDir")]
    config_dir: PathBuf,
    /// Path to the direnv binary
    #[serde(rename = "SelfPath")]
    self_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirenvState {
    /// Information about the .envrc found in the current directory
    #[serde(rename = "foundRC")]
    found_rc: Option<DirenvRC>,
    /// Information about the .envrc that is currently allowed using `direnv allow`
    #[serde(rename = "loadedRC")]
    loaded_rc: Option<DirenvRC>,
}

impl DirenvState {
    /// Check if the .envrc file is allowed
    fn is_allowed(&self) -> bool {
        self.found_rc.as_ref().map_or(false, |rc| rc.allowed == 0)
    }
}

// Information about the .envrc file
#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirenvRC {
    /// Can be 0, 1 or 2
    /// 0: Allowed
    /// 1: NotAllowed
    /// 2: Denied
    allowed: u32,
    /// Path to the .envrc file
    path: PathBuf,
}

impl DirenvStatus {
    /// Parse the output of `direnv status --json`
    fn from_json(json: &str) -> anyhow::Result<Self> {
        let status: DirenvStatus = serde_json::from_str(json)?;
        Ok(status)
    }

    /// Detect user's direnv installation
    fn detect() -> anyhow::Result<Self> {
        let output = std::process::Command::new("direnv")
            .args(["status", "--json"])
            .output()?;
        if output.status.success() {
            let out = String::from_utf8_lossy(&output.stdout);
            let status = DirenvStatus::from_json(&out)?;
            Ok(status)
        } else {
            anyhow::bail!("Unable to run direnv status --json: {:?}", output.stderr)
        }
    }
}
