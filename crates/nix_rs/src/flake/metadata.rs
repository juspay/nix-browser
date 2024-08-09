use crate::command::{NixCmd, NixCmdError};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct FlakeMetadata {
    pub path: String,
}

/// Runs `nix flake metadata json` in Rust
pub async fn get_flake_metadata_json(
    cmd: &NixCmd,
    flake_url: &str,
) -> Result<FlakeMetadata, NixCmdError> {
    let json = cmd
        .run_with_args_expecting_json::<FlakeMetadata>(&["flake", "metadata", "--json", flake_url])
        .await?;
    Ok(json)
}
