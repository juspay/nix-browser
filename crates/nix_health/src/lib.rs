#![feature(associated_type_defaults)]
//! Health checks for the user's Nix install

pub mod check;
pub mod report;
pub mod traits;

use check::direnv::Direnv;
use serde::{Deserialize, Serialize};

use self::check::{
    caches::Caches, flake_enabled::FlakeEnabled, max_jobs::MaxJobs, min_nix_version::MinNixVersion,
    rosetta::Rosetta, trusted_users::TrustedUsers,
};

/// Nix Health check information for user's install
///
/// Each field represents an individual check which satisfies the [Checkable] trait.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct NixHealth {
    #[serde(default)]
    pub max_jobs: MaxJobs,
    #[serde(default)]
    pub caches: Caches,
    #[serde(default)]
    pub flake_enabled: FlakeEnabled,
    #[serde(default)]
    pub nix_version: MinNixVersion,
    #[serde(default)]
    pub trusted_users: TrustedUsers,
    #[serde(default)]
    pub rosetta: Rosetta,
    #[serde(default)]
    pub direnv: Direnv,
}

#[cfg(feature = "ssr")]
impl<'a> IntoIterator for &'a NixHealth {
    type Item = &'a dyn traits::Checkable;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    /// Return an iterator to iterate on the fields of [NixHealth]
    fn into_iter(self) -> Self::IntoIter {
        let mut items: Vec<Self::Item> = vec![
            &self.rosetta,
            &self.nix_version,
            &self.flake_enabled,
            &self.max_jobs,
            &self.caches,
            &self.trusted_users,
            &self.direnv,
        ];
        // direnv has a sub-check
        if self.direnv.enable {
            items.push(&self.direnv.allowed);
        }
        items.into_iter()
    }
}

impl NixHealth {
    /// Create [NixHealth] using configuration from the given flake
    ///
    /// Fallback to using the default health check config if the flake doesn't
    /// override it.
    #[cfg(feature = "ssr")]
    pub async fn from_flake(
        url: nix_rs::flake::url::FlakeUrl,
    ) -> Result<Self, nix_rs::command::NixCmdError> {
        use json_value_merge::Merge;
        use nix_rs::flake::eval::nix_eval_attr_json;
        use serde_json::Value;
        let mut default_value = serde_json::to_value(Self::default())?;
        let v: Value = nix_eval_attr_json(&url).await?;
        default_value.merge(&v);
        let v = serde_json::from_value(default_value)?;
        Ok(v)
    }

    /// Run all checks and collect the results
    #[cfg(feature = "ssr")]
    pub fn run_checks(
        &self,
        nix_info: &nix_rs::info::NixInfo,
        nix_env: &nix_rs::env::NixEnv,
    ) -> Vec<traits::Check> {
        self.into_iter()
            .flat_map(|c| c.check(nix_info, nix_env))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::check::{caches::Caches, min_nix_version::MinNixVersion};

    #[test]
    fn test_json_deserialize_empty() {
        let json = r#"{}"#;
        let v: super::NixHealth = serde_json::from_str(json).unwrap();
        assert_eq!(v.nix_version, MinNixVersion::default());
        assert_eq!(v.caches, Caches::default());
        println!("{:?}", v);
    }

    #[test]
    fn test_json_deserialize_nix_version() {
        let json = r#"{ "nix-version": { "min-required": "2.17.0" } }"#;
        let v: super::NixHealth = serde_json::from_str(json).unwrap();
        assert_eq!(v.nix_version.min_required.to_string(), "2.17.0");
        assert_eq!(v.caches, Caches::default());
    }

    #[test]
    fn test_json_deserialize_caches() {
        let json = r#"{ "caches": { "required": ["https://foo.cachix.org"] } }"#;
        let v: super::NixHealth = serde_json::from_str(json).unwrap();
        assert_eq!(
            v.caches.required,
            vec![url::Url::parse("https://foo.cachix.org").unwrap()]
        );
        assert_eq!(v.nix_version, MinNixVersion::default());
    }
}
