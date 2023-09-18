use nix_rs::{env, info, version::NixVersion};
use serde::{Deserialize, Serialize};

use crate::traits::*;

/// Check that [nix_rs::version::NixVersion] is set to a good value.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct MinNixVersion {
    pub min_required: NixVersion,
}

impl Default for MinNixVersion {
    fn default() -> Self {
        MinNixVersion {
            min_required: NixVersion {
                major: 2,
                minor: 13,
                patch: 0,
            },
        }
    }
}

impl Checkable for MinNixVersion {
    fn check(&self, nix_info: &info::NixInfo, _nix_env: &env::NixEnv) -> Option<Check> {
        let val = &nix_info.nix_version;
        let check = Check {
            title: "Minimum Nix Version".to_string(),
            info: format!("nix version = {}", val),
            result: if val >= &self.min_required {
                CheckResult::Pass
            } else {
                CheckResult::Fail {
                    msg: format!("Your Nix version ({}) is too old; we require at least {}", val, self.min_required),
                    suggestion: "See https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-upgrade-nix.html".into(),
                }
            },
            required: true,
        };
        Some(check)
    }
}
