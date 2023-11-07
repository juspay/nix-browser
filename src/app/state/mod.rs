//! Application state

pub mod action;
mod datum;
mod error;

use assoc::AssocExt;
use dioxus::prelude::{use_context, use_context_provider, use_future, Scope};
use dioxus_signals::Signal;
use dioxus_std::storage::{new_storage, LocalStorage};
use nix_health::NixHealth;
use nix_rs::{
    flake::{url::FlakeUrl, Flake},
    info::NixInfo,
};

use self::{action::Action, datum::Datum, error::SystemError};

/// Our dioxus application state is a struct of [Signal]s that store app state.
///
/// They use [Datum] which is a glorified [Option] to distinguish between initial
/// loading and subsequent refreshing.
///
/// Use [Action] to mutate this state, in addition to [Signal::set].
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct AppState {
    pub nix_info: Signal<Datum<Result<NixInfo, SystemError>>>,
    pub health_checks: Signal<Datum<Result<Vec<nix_health::traits::Check>, SystemError>>>,

    /// User selected [FlakeUrl]
    pub flake_url: Signal<Option<FlakeUrl>>,
    /// [Flake] for [AppState::flake_url]
    pub flake: Signal<Datum<Result<Flake, SystemError>>>,
    /// Cached [Flake] values indexed by [FlakeUrl]
    pub flake_cache: Signal<Vec<(FlakeUrl, Flake)>>,
    /// List of recently selected [AppState::flake_url]s
    pub recent_flakes: Signal<Vec<FlakeUrl>>,

    /// [Action] represents the next modification to perform on [AppState] signals
    pub action: Signal<(usize, Action)>,
}

impl AppState {
    fn new(cx: Scope) -> Self {
        tracing::debug!("🔨 Creating new AppState");
        let recent_flakes =
            new_storage::<LocalStorage, _>(cx, "recent_flakes".to_string(), FlakeUrl::suggestions);
        let flake_cache = new_storage::<LocalStorage, _>(cx, "flake_cache".to_string(), Vec::new);
        AppState {
            recent_flakes,
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

    /// Perform an [Action] on the state
    ///
    /// This eventuates an update on the appropriate signals the state holds.
    pub fn act(&self, action: Action) {
        self.action.with_mut(|(i, v)| {
            *i += 1;
            *v = action;
        });
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
        // Build `state.flake` signal when `state.flake_url` changes or the
        // RefreshFlake action is triggered
        {
            let update_flake = |refresh: bool| async move {
                let flake_url = self.flake_url.read().clone();
                if let Some(flake_url) = flake_url {
                    let flake_url_2 = flake_url.clone();
                    tracing::info!("Updating flake [{}] refresh={} ...", flake_url, refresh);
                    Datum::refresh_with(self.flake, async move {
                        Flake::from_nix(&nix_rs::command::NixCmd::default(), flake_url_2)
                            .await
                            .map_err(|e| Into::<SystemError>::into(e.to_string()))
                    })
                    .await;
                    if let Some(Ok(flake)) = self.flake.read().current_value() {
                        tracing::info!("Caching flake [{}]", flake_url);
                        self.flake_cache.with_mut(|cache| {
                            vec_push_as_latest(cache, (flake_url.clone(), flake.clone()))
                                .truncate(8);
                        });
                    }
                }
            };
            let flake_url = self.flake_url.read().clone();
            let refresh_action =
                Action::signal_for(cx, self.action, |act| act == Action::RefreshFlake);
            let idx = *refresh_action.read();
            // ... when URL changes.
            use_future(cx, (&flake_url,), |(flake_url,)| async move {
                if let Some(flake_url) = flake_url {
                    if let Some(cached_flake) = self.flake_cache.read().get(&flake_url).cloned() {
                        tracing::info!("Cache hit for {}", flake_url);
                        Datum::refresh_with(self.flake, async { Ok(cached_flake) }).await;
                    } else {
                        self.act(Action::RefreshFlake);
                    }
                }
            });
            // ... when refresh button is clicked.
            use_future(cx, (&idx,), |(idx,)| update_flake(idx.is_some()));
        }

        // Update recent_flakes
        {
            let flake_url = self.flake_url.read().clone();
            use_future(cx, (&flake_url,), |(flake_url,)| async move {
                if let Some(flake_url) = flake_url {
                    self.recent_flakes.with_mut(|items| {
                        vec_push_as_latest(items, flake_url).truncate(8);
                    });
                }
            });
        }

        // Build `state.health_checks` when nix_info changes
        {
            let nix_info = self.nix_info.read().clone();
            use_future(cx, (&nix_info,), |(nix_info1,)| async move {
                if let Some(nix_info) = nix_info1.current_value().map(|x| {
                    x.as_ref()
                        .map_err(|e| Into::<SystemError>::into(e.to_string()))
                        .map(|v| v.clone())
                }) {
                    Datum::refresh_with(self.health_checks, async move {
                        let health_checks = NixHealth::default().run_checks(&nix_info?, None);
                        Ok(health_checks)
                    })
                    .await;
                }
            });
        }

        // Build `state.nix_info` when GetNixInfo action is triggered
        {
            let get_nix_info_action =
                Action::signal_for(cx, self.action, |act| act == Action::GetNixInfo);
            let idx = *get_nix_info_action.read();
            use_future(cx, (&idx,), |(last_event_idx,)| async move {
                tracing::info!("Updating nix info [{:?}] ...", last_event_idx);
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

/// Push an item to the front of a vector
///
/// If the item already exits, move it to the front.
fn vec_push_as_latest<T: PartialEq>(vec: &mut Vec<T>, item: T) -> &mut Vec<T> {
    if let Some(idx) = vec.iter().position(|x| *x == item) {
        vec.remove(idx);
    }
    vec.insert(0, item);
    vec
}
