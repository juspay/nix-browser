use nix_rs::{info, version::NixVersion};
use serde::{Deserialize, Serialize};

use crate::{
    report::{Report, WithDetails},
    traits::Check,
};

/// Check that [crate::version::NixVersion] is set to a good value.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinNixVersion(pub NixVersion);

impl Check for MinNixVersion {
    fn check(nix_info: &info::NixInfo, _sys_info: &info::SysInfo) -> Self {
        MinNixVersion(nix_info.nix_version.clone())
    }
    fn name(&self) -> &'static str {
        "Minimum Nix Version"
    }
    fn report(&self) -> Report<WithDetails> {
        let min_required = NixVersion {
            major: 2,
            minor: 13,
            patch: 0,
        };
        if self.0 >= min_required {
            Report::Green
        } else {
            Report::Red(WithDetails {
                msg: format!("Your Nix version ({}) is too old; we require at least {}", self.0, min_required),
                suggestion: "See https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-upgrade-nix.html".into(),
            })
        }
    }
}
