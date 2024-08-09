//! Rust module for Nix flakes

pub mod eval;
pub mod metadata;
pub mod outputs;
pub mod schema;
pub mod system;
pub mod url;

use serde::{Deserialize, Serialize};

use tracing::instrument;

use self::{outputs::FlakeOutputs, schema::FlakeSchema, system::System, url::FlakeUrl};

use crate::{
    command::{NixCmd, NixCmdError},
    config::NixConfig,
};

/// All the information about a Nix flake
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flake {
    /// The flake url which this struct represents
    pub url: FlakeUrl,
    /// `nix flake show` output
    pub output: FlakeOutputs,
    /// Flake output schema (typed version of [FlakeOutputs])
    pub schema: FlakeSchema,
    // TODO: Add `nix flake metadata` info.
}

impl Flake {
    /// Get [Flake] info for the given flake url

    #[instrument(name = "flake", skip(nix_cmd))]
    pub async fn from_nix(
        nix_cmd: &NixCmd,
        nix_config: &NixConfig,
        url: FlakeUrl,
    ) -> Result<Flake, NixCmdError> {
        let output = FlakeOutputs::from_nix(nix_cmd, &url).await?;
        let schema = FlakeSchema::from(&output, &nix_config.system.value);
        Ok(Flake {
            url,
            output: output.clone(),
            schema,
        })
    }
}
