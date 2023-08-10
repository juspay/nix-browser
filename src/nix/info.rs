//! Information about the user's Nix installation
use leptos::*;
use serde::{Deserialize, Serialize};

use crate::nix::config::NixConfig;

/// All the information about the user's Nix installation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NixInfo {
    /// Nix version string
    pub nix_version: String,
    pub nix_config: NixConfig,
}

/// Determine [NixInfo] on the user's system
#[server(GetNixInfo, "/api")]
pub async fn get_nix_info() -> Result<NixInfo, ServerFnError> {
    use tokio::process::Command;
    use tracing::info_span;
    let _span = info_span!("get_nix_info").entered();
    let mut cmd = Command::new("nix");
    cmd.arg("--version");
    let stdout = crate::command::run_command_in_server_fn(&mut cmd).await?;
    // TODO: Parse the version string
    let nix_version = String::from_utf8(stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| <std::string::FromUtf8Error as Into<ServerFnError>>::into(e))?;
    let nix_config = super::config::run_nix_show_config().await?;
    tracing::info!("Got nix info. Version = {}", nix_version);
    Ok(NixInfo {
        nix_version,
        nix_config,
    })
}

impl IntoView for NixInfo {
    fn into_view(self, cx: Scope) -> View {
        view! { cx,
            <div class="flex flex-col p-4 space-y-8 border-2 border-black rounded shadow-md bg-primary-100">
                <div>
                    <b>
                        Nix Version
                    </b>
                    <pre>{self.nix_version}</pre>
                </div>
                <div>
                    <b>
                        Nix Config
                    </b>
                    {self.nix_config}
                </div>
            </div>
        }
        .into_view(cx)
    }
}
