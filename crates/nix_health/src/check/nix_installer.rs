use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::Path};
use thiserror::Error;

use nix_rs::info;

use crate::traits::*;

#[derive(Debug, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Clone)]
pub struct NixInstallerVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Display for NixInstallerVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum BadNixInstallerVersion {
    #[error("Invalid format")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("Parse error: nix-installer --version cannot be parsed")]
    Parse(#[from] std::num::ParseIntError),
    #[error("Failed to run nix-installer --version")]
    Command,
    #[error("Invalid format")]
    InvalidFormat,
}

/// Check that https://github.com/DeterminateSystems/nix-installer is used
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct NixInstaller {
    pub min_required: NixInstallerVersion,
}

impl Default for NixInstaller {
    fn default() -> Self {
        NixInstaller {
            min_required: NixInstallerVersion {
                major: 0,
                minor: 17,
                patch: 0,
            },
        }
    }
}

impl NixInstallerVersion {
    pub fn get_version(executable_path: &Path) -> Result<Self, BadNixInstallerVersion> {
        let output = std::process::Command::new(executable_path)
            .arg("--version")
            .output()
            .map_err(|_| BadNixInstallerVersion::Command)?;
        let output = String::from_utf8(output.stdout)?;
        let version = output
            .split_whitespace()
            .last()
            .ok_or(BadNixInstallerVersion::InvalidFormat)?;
        let mut version = version.split('.').map(|x| x.parse::<u32>());
        let major = version
            .next()
            .ok_or(BadNixInstallerVersion::InvalidFormat)??;
        let minor = version
            .next()
            .ok_or(BadNixInstallerVersion::InvalidFormat)??;
        let patch = version
            .next()
            .ok_or(BadNixInstallerVersion::InvalidFormat)??;
        Ok(NixInstallerVersion {
            major,
            minor,
            patch,
        })
    }
}

impl Checkable for NixInstaller {
    fn check(&self, _: &info::NixInfo, _: Option<&nix_rs::flake::url::FlakeUrl>) -> Vec<Check> {
        let nix_installer_path = Path::new("/nix/nix-installer");
        let nix_installer_exists = nix_installer_path.exists();
        let nix_installer_version =
            NixInstallerVersion::get_version(nix_installer_path).unwrap_or(NixInstallerVersion {
                major: 0,
                minor: 0,
                patch: 0,
            });
        // Check if /nix/nix-installer is present, if it is present check if nix-installer --version is >= 0.14
        let check = Check {
            title: "DetSys nix-installer".to_string(),
            info: format!("/nix/nix-installer present = {}", nix_installer_exists),
            result: if nix_installer_exists && nix_installer_version < self.min_required {
                CheckResult::Red {
                    msg: format!("Your nix-installer version ({}) is too old; we require at least {}", nix_installer_version, self.min_required),
                    suggestion: "Uninstall from https://github.com/DeterminateSystems/nix-installer?tab=readme-ov-file#uninstalling and reinstall".into(),
                }
            } else if !nix_installer_exists {
                CheckResult::Red {
                    msg: "You are not using the recommended way to install Nix. ".to_string(),
                    suggestion: "Uninstall the existing installation: https://nix.dev/manual/nix/2.18/installation/uninstall.html#macos. Then follow https://nixos.asia/en/install".into(),
                }
            } else {
                CheckResult::Green
            },
            required: true,
        };
        vec![check]
    }
}
