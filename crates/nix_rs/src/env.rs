//! Information about the environment in which Nix will run
use os_info;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use std::fs;
use std::{env, io};
use thiserror::Error;

/// The environment in which Nix operates
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NixEnv {
    /// value of $USER
    pub current_user: String,
    /// Underlying nix system information
    pub nix_system: NixSystem,
}

/// The system under which Nix is installed and operates
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NixSystem {
    /// https://github.com/LnL7/nix-darwin
    NixDarwin,
    /// https://nixos.org/
    NixOS,
    /// Nix is individually installed on Linux or macOS
    Other(os_info::Type),
}

impl NixSystem {
    #[cfg(feature = "ssr")]
    pub fn detect() -> Self {
        let os_type = os_info::get().os_type();
        fn is_symlink(file_path: &str) -> io::Result<bool> {
            let metadata = fs::symlink_metadata(file_path)?;
            Ok(metadata.file_type().is_symlink())
        }
        match os_type {
            // To detect that we are on NixDarwin, we check if /etc/nix/nix.conf
            // is a symlink (which nix-darwin manages like NixOS does)
            os_info::Type::Macos if is_symlink("/etc/nix/nix.conf").unwrap_or(false) => {
                NixSystem::NixDarwin
            }
            os_info::Type::NixOS => NixSystem::NixOS,
            _ => NixSystem::Other(os_type),
        }
    }

    /// The Nix for this [NixSystem] is configured automatically through a `configuration.nix`
    pub fn has_configuration_nix(&self) -> bool {
        self == &NixSystem::NixOS || self == &NixSystem::NixDarwin
    }
}

/// Errors while trying to fetch system info
#[derive(Error, Debug)]
pub enum NixEnvError {
    #[error("Failed to fetch ENV: {0}")]
    EnvVarError(#[from] env::VarError),
}
impl NixEnv {
    /// Determine [NixEnv] on the user's system
    #[cfg(feature = "ssr")]
    pub async fn get_info() -> Result<NixEnv, NixEnvError> {
        let current_user = env::var("USER")?;
        let nix_system = NixSystem::detect();
        Ok(NixEnv {
            current_user,
            nix_system,
        })
    }
}