//! Application state

mod datum;
mod db;

use std::fmt::Display;

use dioxus::prelude::{use_context, use_context_provider, use_future, Scope};
use dioxus_signals::{use_signal, Signal};
use nix_health::NixHealth;
use nix_rs::{
    command::NixCmdError,
    flake::{url::FlakeUrl, Flake},
};

use self::{datum::Datum, db::Db};

/// Our dioxus application state is a struct of [Signal]
///
/// They use [Datum] which is a glorified [Option] to distinguis between initial
/// loading and subsequent refreshing.
///
/// Use [Action] to mutate this state, in addition to [Signal::set].
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct AppState {
    /// The sqlite database pool
    pub db: Signal<Option<Result<Db, sqlx::Error>>>,

    pub nix_info: Signal<Datum<Result<nix_rs::info::NixInfo, SystemError>>>,
    pub health_checks: Signal<Datum<Result<Vec<nix_health::traits::Check>, SystemError>>>,

    /// User selected [FlakeUrl]
    pub flake_url: Signal<Option<FlakeUrl>>,
    /// [Flake] for [AppState::flake_url]
    pub flake: Signal<Datum<Result<Flake, NixCmdError>>>,
    /// List of recently selected [AppState::flake_url]s
    pub recent_flakes: Signal<Vec<FlakeUrl>>,

    /// [Action] represents the next modification to perform on [AppState] signals
    pub action: Signal<(usize, Action)>,
}

/// An action to be performed on [AppState]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Refresh the [AppState::flake] signal using [AppState::flake_url] signal's current value
    RefreshFlake,

    /// Refresh [AppState::nix_info] signal
    #[default]
    GetNixInfo,
}

impl Action {
    /// Return a [Signal] containing only the given [Action]
    ///
    /// The signal value will be the [Action]'s index in the original signal.
    pub fn signal_for<F>(cx: Scope, sig: Signal<(usize, Action)>, f: F) -> Signal<usize>
    where
        F: Fn(Action) -> bool + 'static,
    {
        signal_filter_map(
            cx,
            sig,
            0,
            move |(idx, action)| {
                if f(action) {
                    Some(idx)
                } else {
                    None
                }
            },
        )
    }
}

impl AppState {
    /// Perform an [Action] on the state
    ///
    /// This eventuates an update on the appropriate signals the state holds.
    pub fn act(&self, action: Action) {
        self.action.with_mut(|(i, v)| {
            *i += 1;
            *v = action;
        });
    }

    /// Get the [AppState] from context
    pub fn use_state<T>(cx: Scope<T>) -> Self {
        *use_context::<Self>(cx).unwrap()
    }

    pub fn provide_state(cx: Scope) {
        tracing::debug!("🏗️ Providing AppState");
        let state = *use_context_provider(cx, || {
            tracing::debug!("🔨 Creating AppState default value");
            AppState::default()
        });
        // FIXME: Can we avoid calling build_network multiple times?
        state.build_network(cx);
        use_future(cx, (), |_| async move {
            // XXX: Simulating slowness
            // tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            state.initialize().await;
        });
    }

    async fn initialize(self) {
        let db = Db::new().await;

        if let Ok(db) = db.as_ref() {
            self.recent_flakes
                .set(db.recent_flakes().await.unwrap_or_default());
        }

        self.db.set(Some(db));
    }

    /// Return the initialization state of [AppState]
    pub fn initialization_state(self) -> Option<Result<(), SystemError>> {
        let v = self.db.read();
        v.as_ref().map(|x| {
            x.as_ref()
                .map(|_| ())
                .map_err(|e| Into::<SystemError>::into(e.to_string()))
        })
    }

    /// Get [Db]. Only safe after initialization (see [AppState::initialization_state]])
    pub(crate) fn get_db(self) -> Db {
        self.db.read().as_ref().unwrap().as_ref().unwrap().clone()
    }

    /// Build the Signal network
    ///
    /// If a signal's value is dependent on another signal's value, you must
    /// define that relationship here.
    fn build_network(self, cx: Scope) {
        tracing::debug!("🕸️ Building AppState network");
        // Build `state.flake` signal when `state.flake_url` changes or the
        // RefreshFlake action is triggered
        {
            let flake_url = self.flake_url.read().clone();
            let refresh_action =
                Action::signal_for(cx, self.action, |act| act == Action::RefreshFlake);
            let idx = *refresh_action.read();
            use_future(cx, (&flake_url, &idx), |(flake_url, idx)| async move {
                if let Some(flake_url) = flake_url {
                    tracing::info!("Updating flake [{}] {} ...", flake_url, idx);
                    // TODO: refactor?
                    if let Err(err) = self.get_db().register_flake(&flake_url).await {
                        tracing::error!("Failed to register flake in db: {}", err);
                    } else {
                        match self.get_db().recent_flakes().await {
                            Ok(recent_flakes) => {
                                self.recent_flakes.set(recent_flakes);
                            }
                            Err(err) => {
                                tracing::error!("Failed to get recent flakes: {}", err);
                            }
                        }
                    }
                    Datum::refresh_with(self.flake, async move {
                        Flake::from_nix(&nix_rs::command::NixCmd::default(), flake_url.clone())
                            .await
                    })
                    .await
                }
            });
        }

        // Update recent flakes
        {
            let flake_url = self.flake_url.read().clone();
            use_future(cx, (&flake_url,), |(flake_url,)| async move {
                if let Some(flake_url) = flake_url {
                    let mut recent_flakes = self.recent_flakes.read().clone();
                    if !recent_flakes.contains(&flake_url) {
                        recent_flakes.push(flake_url.clone());
                        self.recent_flakes.set(recent_flakes);
                    }
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
            use_future(cx, (&idx,), |(idx,)| async move {
                tracing::info!("Updating nix info [{}] ...", idx);
                Datum::refresh_with(self.nix_info, async {
                    nix_rs::info::NixInfo::from_nix(&nix_rs::command::NixCmd::default())
                        .await
                        .map_err(|e| SystemError {
                            message: format!("Error getting nix info: {:?}", e),
                        })
                })
                .await;
            });
        }
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

/// Like [std::iter::Iterator::filter_map] but applied on a dioxus [Signal]
///
/// Since `Signal`s always have a value, an `initial` value must be provided.
///
/// Upstream issue: https://github.com/DioxusLabs/dioxus/issues/1555
fn signal_filter_map<T, U, F>(cx: Scope, sig: Signal<T>, initial: U, f: F) -> Signal<U>
where
    F: Fn(T) -> Option<U> + 'static,
    T: Copy,
{
    let res: Signal<U> = use_signal(cx, || initial);
    dioxus_signals::use_effect(cx, move || {
        let value = *sig.read();
        if let Some(value) = f(value) {
            res.set(value);
        }
    });
    res
}
