#![feature(associated_type_defaults)]
//! Health checks for the user's Nix install

pub mod check;
pub mod report;
pub mod traits;

use nix_rs::{info, system};

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use self::check::{
    caches::Caches, flake_enabled::FlakeEnabled, max_jobs::MaxJobs, min_nix_version::MinNixVersion,
    trusted_users::TrustedUsers,
};
use self::report::{NoDetails, Report, WithDetails};
use self::traits::Check;

/// Nix Health check information for user's install
///
/// Each field represents an individual check which satisfies the [Check] trait.
///
/// NOTE: This struct is isomorphic to [Vec<Box<&dyn Check>>]. We cannot use the
/// latter due to (wasm) serialization limitation with dyn trait objects. An
// [IntoIterator] impl is provide towards this end.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NixHealth {
    pub max_jobs: MaxJobs,
    pub caches: Caches,
    pub flake_enabled: FlakeEnabled,
    pub min_nix_version: MinNixVersion,
    pub trusted_users: TrustedUsers,
}

impl<'a> IntoIterator for &'a NixHealth {
    type Item = &'a dyn Check<Report = Report<WithDetails>>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    /// Return an iterator to iterate on the fields of [NixHealth]
    fn into_iter(self) -> Self::IntoIter {
        let items: Vec<Self::Item> = vec![
            &self.max_jobs,
            &self.caches,
            &self.flake_enabled,
            &self.min_nix_version,
            &self.trusted_users,
        ];
        items.into_iter()
    }
}

impl Check for NixHealth {
    type Report = Report<NoDetails>;
    fn check(nix_info: &info::NixInfo, sys_info: &system::SysInfo) -> Self {
        NixHealth {
            max_jobs: MaxJobs::check(nix_info, sys_info),
            caches: Caches::check(nix_info, sys_info),
            flake_enabled: FlakeEnabled::check(nix_info, sys_info),
            min_nix_version: MinNixVersion::check(nix_info, sys_info),
            trusted_users: TrustedUsers::check(nix_info, sys_info),
        }
    }
    fn name(&self) -> &'static str {
        "Nix Health"
    }
    fn report(&self) -> Report<NoDetails> {
        if self.into_iter().all(|c| c.report() == Report::Green) {
            Report::Green
        } else {
            Report::Red(NoDetails)
        }
    }
}

impl Display for NixHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
