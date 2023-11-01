//! Frontend UI entry point

// Workaround for https://github.com/rust-lang/rust-analyzer/issues/15344
#![allow(non_snake_case)]

mod flake;
mod health;
mod info;
mod state;
mod widget;

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use dioxus_signals::Signal;
use nix_rs::flake::url::FlakeUrl;

use crate::app::{
    flake::{Flake, FlakeRaw},
    health::Health,
    info::Info,
    state::AppState,
    widget::{Loader, RefreshButton},
};

#[derive(Routable, PartialEq, Debug, Clone)]
#[rustfmt::skip]
enum Route {
    #[layout(Wrapper)]
        #[route("/")]
        Dashboard {},
        #[route("/flake")]
        Flake {},
        #[route("/flake/raw")]
        FlakeRaw {},
        #[route("/health")]
        Health {},
        #[route("/info")]
        Info {},
}

/// Main frontend application container
pub fn App(cx: Scope) -> Element {
    AppState::provide_state(cx);
    render! {
        body { class: "bg-base-100 overflow-hidden", Router::<Route> {} }
    }
}

fn Wrapper(cx: Scope) -> Element {
    render! {
        div { class: "flex flex-col text-center justify-between w-full h-screen",
            TopBar {}
            div { class: "m-2 py-2 overflow-auto", Outlet::<Route> {} }
            Footer {}
        }
    }
}

#[component]
fn TopBar(cx: Scope) -> Element {
    let state = AppState::use_state(cx);
    let health_checks = state.health_checks.read();
    let nix_info = state.nix_info.read();
    render! {
        div { class: "flex justify-between items-center w-full p-2 bg-primary-100 shadow",
            div { class: "flex space-x-2",
                Link { to: Route::Dashboard {}, "🏠" }
            }
            div { class: "flex space-x-2",
                ViewRefreshButton {}
                Link { to: Route::Health {},
                    span { title: "Nix Health Status",
                        match (*health_checks).current_value() {
                            Some(Ok(checks)) => render! {
                                if checks.iter().all(|check| check.result.green()) {
                                    "✅"
                                } else {
                                    "❌"
                                }
                            },
                            Some(Err(err)) => render! { "{err}" },
                            None => render! { Loader {} },
                        }
                    }
                }
                Link { to: Route::Info {},
                    span {
                        "Nix "
                        match (*nix_info).current_value() {
                            Some(Ok(info)) => render! {
                                "{info.nix_version} on {info.nix_env.os}"
                            },
                            Some(Err(err)) => render! { "{err}" },
                            None => render! { Loader {} },
                        }
                    }
                }
            }
        }
    }
}

/// Intended to refresh the data behind the current route.
#[component]
fn ViewRefreshButton(cx: Scope) -> Element {
    let state = AppState::use_state(cx);
    let (busy, action) = match use_route(cx).unwrap() {
        Route::Flake {} => Some((
            state.flake.read().is_loading_or_refreshing(),
            state::Action::RefreshFlake,
        )),
        Route::Health {} => Some((
            state.health_checks.read().is_loading_or_refreshing(),
            state::Action::GetNixInfo,
        )),
        Route::Info {} => Some((
            state.nix_info.read().is_loading_or_refreshing(),
            state::Action::GetNixInfo,
        )),
        _ => None,
    }?;
    render! {
        RefreshButton {
            busy: busy,
            handler: move |_| {
                state.act(action);
            }
        }
    }
}

#[component]
fn Footer(cx: Scope) -> Element {
    render! {
        footer { class: "flex flex-row justify-center w-full bg-primary-100 p-2",
            a { href: "https://github.com/juspay/nix-browser", img { src: "images/128x128.png", class: "h-4" } }
        }
    }
}

// Home page
fn Dashboard(cx: Scope) -> Element {
    tracing::debug!("Rendering Dashboard page");
    let state = AppState::use_state(cx);
    render! {
        div { class: "pl-4",
            h2 { class: "text-2xl", "Enter a flake URL:" }
            // TODO: search input here
            p { "TODO: search input" }
            h2 { class: "text-2xl", "Or, try one of these:" }
            div { class: "flex flex-col",
                for flake in state.recent_flakes.read().clone() {
                    a {
                        onclick: move |_| {
                            let state = AppState::use_state(cx);
                            let nav = use_navigator(cx);
                            state.set_flake_url(flake.clone());
                            nav.replace(Route::Flake {});
                        },
                        class: "cursor-pointer text-primary-600 underline hover:no-underline",
                        "{flake.clone()}"
                    }
                }
            }
        }
    }
}
