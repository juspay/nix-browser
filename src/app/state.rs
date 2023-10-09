//! Application state

use std::fmt::Display;

use dioxus::prelude::{use_context, use_context_provider, use_future, Scope};
use dioxus_signals::Signal;
use nix_health::NixHealth;
use nix_rs::{
    command::NixCmdError,
    flake::{url::FlakeUrl, Flake},
};
use tracing::instrument;

/// Catch all error to use in UI components
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SystemError {
    pub message: String,
}

impl Display for SystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<String> for SystemError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct AppState {
    pub nix_info: Signal<Option<Result<nix_rs::info::NixInfo, SystemError>>>,
    pub nix_env: Signal<Option<Result<nix_rs::env::NixEnv, SystemError>>>,
    pub health_checks: Signal<Option<Result<Vec<nix_health::traits::Check>, SystemError>>>,

    pub flake_url: Signal<FlakeUrl>,
    pub flake: Signal<Option<Result<Flake, NixCmdError>>>,
}

impl AppState {
    pub async fn initialize(&self) {
        tracing::info!("Initializing app state");
        if self.nix_info.read().is_none() {
            self.update_nix_info().await;
        }
        if self.nix_env.read().is_none() {
            self.update_nix_env().await;
        }
        if self.health_checks.read().is_none() {
            self.update_health_checks().await;
        }
    }

    #[instrument(name = "update-nix-info", skip(self))]
    pub async fn update_nix_info(&self) {
        tracing::info!("Updating nix info ...");
        // NOTE: Without tokio::spawn, this will run in main desktop thread,
        // and will hang at some point.
        let nix_info = tokio::spawn(async move {
            nix_rs::info::NixInfo::from_nix(&nix_rs::command::NixCmd::default())
                .await
                .map_err(|e| SystemError {
                    message: format!("Error getting nix info: {:?}", e),
                })
        })
        .await
        .unwrap();
        tracing::info!("Got nix info, about to mut");
        self.nix_info.with_mut(move |x| {
            *x = Some(nix_info);
            tracing::info!("Updated nix info");
        });
    }

    #[instrument(name = "update-nix-env", skip(self))]
    pub async fn update_nix_env(&self) {
        tracing::info!("Updating nix env ...");
        let nix_env = tokio::spawn(async move {
            nix_rs::env::NixEnv::detect(None)
                .await
                .map_err(|e| e.to_string().into())
        })
        .await
        .unwrap();
        tracing::info!("Got nix env, about to mut");
        self.nix_env.with_mut(move |x| {
            *x = Some(nix_env);
            tracing::info!("Updated nix env");
        });
    }

    #[instrument(name = "update-health-checks", skip(self))]
    pub async fn update_health_checks(&self) {
        tracing::info!("Updating health checks ...");
        // Update depenencies
        self.update_nix_info().await;
        self.update_nix_env().await;
        let get_nix_health = move || -> Result<Vec<nix_health::traits::Check>, SystemError> {
            let nix_env = self.nix_env.read();
            let nix_env: &nix_rs::env::NixEnv = nix_env
                .as_ref()
                .unwrap()
                .as_ref()
                .map_err(|e| Into::<SystemError>::into(e.to_string()))?;
            let nix_info = self.nix_info.read();
            let nix_info: &nix_rs::info::NixInfo = nix_info
                .as_ref()
                .unwrap()
                .as_ref()
                .map_err(|e| Into::<SystemError>::into(e.to_string()))?;
            let health_checks = NixHealth::default().run_checks(nix_info, nix_env);
            Ok(health_checks)
        };
        let health_checks = get_nix_health();
        tracing::info!("Got health checks, about to mut");
        self.health_checks.with_mut(move |x| {
            *x = Some(health_checks);
            tracing::info!("Updated health checks");
        });
    }

    #[instrument(name = "update-flake", skip(self))]
    pub async fn update_flake(&self) {
        tracing::info!("Updating flake ...");
        let flake_url = self.flake_url.read().clone();
        let flake = tokio::spawn(async move {
            Flake::from_nix(&nix_rs::command::NixCmd::default(), flake_url.clone()).await
        })
        .await
        .unwrap();
        tracing::info!("Got flake, about to mut");
        self.flake.with_mut(move |x| {
            *x = Some(flake);
            tracing::info!("Updated flake");
        });
    }

    /// Get the [AppState] from context
    pub fn use_state(cx: Scope) -> Self {
        *use_context(cx).unwrap()
    }

    pub fn provide_state(cx: Scope) {
        use_context_provider(cx, AppState::default);
        let state = AppState::use_state(cx);
        use_future(cx, (), |_| async move {
            state.initialize().await;
        });
    }
}
