//! Application state

mod datum;
mod db;
mod error;
mod refresh;

use dioxus::prelude::{use_context, use_context_provider, use_future, Scope};
use dioxus_signals::Signal;
use nix_health::NixHealth;
use nix_rs::{
    flake::{url::FlakeUrl, Flake},
    info::NixInfo,
};

use self::{datum::Datum, error::SystemError, refresh::Refresh};

/// Our dioxus application state is a struct of [Signal]s that store app state.
///
/// They use [Datum] which is a glorified [Option] to distinguish between initial
/// loading and subsequent refreshing.
///
/// Use [Action] to mutate this state, in addition to [Signal::set].
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct AppState {
    /// [NixInfo] as detected on the user's system
    pub nix_info: Signal<Datum<Result<NixInfo, SystemError>>>,
    pub nix_info_refresh: Signal<Refresh>,

    /// User's Nix health [nix_health::traits::Check]s
    pub health_checks: Signal<Datum<Result<Vec<nix_health::traits::Check>, SystemError>>>,
    pub health_checks_refresh: Signal<Refresh>,

    /// User selected [FlakeUrl]
    pub flake_url: Signal<Option<FlakeUrl>>,
    /// Trigger to refresh [AppState::flake]
    pub flake_refresh: Signal<Refresh>,
    /// [Flake] for [AppState::flake_url]
    pub flake: Signal<Datum<Result<Flake, SystemError>>>,

    /// Cached [Flake] values indexed by [FlakeUrl]
    ///
    /// Most recently updated flakes appear first.
    pub flake_cache: Signal<db::FlakeCache>,
}

impl AppState {
    fn new(cx: Scope) -> Self {
        tracing::info!("🔨 Creating new AppState");
        // TODO: Should we use new_synced_storage, instead? To allow multiple app windows?
        let flake_cache = db::FlakeCache::new_signal(cx);
        AppState {
            flake_cache,
            ..AppState::default()
        }
    }

    /// Get the [AppState] from context
    pub fn use_state(cx: Scope) -> Self {
        *use_context::<Self>(cx).unwrap()
    }

    pub fn provide_state(cx: Scope) {
        tracing::debug!("🏗️ Providing AppState");
        let state = *use_context_provider(cx, || Self::new(cx));
        // FIXME: Can we avoid calling build_network multiple times?
        state.build_network(cx);
    }

    /// Return the [String] representation of the current [AppState::flake_url] value. If there is none, return empty string.
    pub fn get_flake_url_string(&self) -> String {
        self.flake_url
            .read()
            .clone()
            .map_or("".to_string(), |url| url.to_string())
    }

    pub fn set_flake_url(&self, url: FlakeUrl) {
        tracing::info!("setting flake url to {}", &url);
        self.flake_url.set(Some(url));
    }
}

impl AppState {
    /// Build the Signal network
    ///
    /// If a signal's value is dependent on another signal's value, you must
    /// define that relationship here.
    fn build_network(self, cx: Scope) {
        tracing::debug!("🕸️ Building AppState network");
        // Build `state.flake` signal dependent signals change
        {
            // ... when [AppState::flake_url] changes.
            let flake_url = self.flake_url.read().clone();
            use_future(cx, (&flake_url,), |(flake_url,)| async move {
                if let Some(flake_url) = flake_url {
                    let maybe_flake = self.flake_cache.read().get(&flake_url);
                    if let Some(cached_flake) = maybe_flake {
                        Datum::set_value(self.flake, Ok(cached_flake)).await;
                    } else {
                        self.flake_refresh.write().request_refresh();
                    }
                }
            });
            // ... when refresh button is clicked.
            let refresh = *self.flake_refresh.read();
            use_future(cx, (&refresh,), |(refresh,)| async move {
                let flake_url = self.flake_url.read().clone();
                if let Some(flake_url) = flake_url {
                    let flake_url_2 = flake_url.clone();
                    tracing::info!("Updating flake [{}] refresh={} ...", &flake_url, refresh);
                    let res = Datum::refresh_with(self.flake, async move {
                        Flake::from_nix(&nix_rs::command::NixCmd::default(), flake_url_2)
                            .await
                            .map_err(|e| Into::<SystemError>::into(e.to_string()))
                    })
                    .await;
                    if let Some(Ok(flake)) = res {
                        self.flake_cache.with_mut(|cache| {
                            cache.update(flake_url, flake);
                        });
                    }
                }
            });
        }

        // Build `state.health_checks`
        {
            let nix_info = self.nix_info.read().clone();
            let refresh = *self.health_checks_refresh.read();
            use_future(
                cx,
                (&nix_info, &refresh),
                |(nix_info, refresh)| async move {
                    if let Some(nix_info) = nix_info.current_value().map(|x| {
                        x.as_ref()
                            .map_err(|e| Into::<SystemError>::into(e.to_string()))
                            .map(|v| v.clone())
                    }) {
                        tracing::info!("Updating nix health [{}] ...", refresh);
                        Datum::refresh_with(self.health_checks, async move {
                            let health_checks = NixHealth::default().run_checks(&nix_info?, None);
                            Ok(health_checks)
                        })
                        .await;
                    }
                },
            );
        }

        // Build `state.nix_info`
        {
            let refresh = *self.nix_info_refresh.read();
            use_future(cx, (&refresh,), |(refresh,)| async move {
                tracing::info!("Updating nix info [{}] ...", refresh);
                Datum::refresh_with(self.nix_info, async {
                    NixInfo::from_nix(&nix_rs::command::NixCmd::default())
                        .await
                        .map_err(|e| SystemError {
                            message: format!("Error getting nix info: {:?}", e),
                        })
                })
                .await;
            });
        }
    }
}
