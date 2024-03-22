//! Nix health check UI

use dioxus::prelude::*;
use nix_health::traits::{Check, CheckResult};
use dioxus_markdown::Markdown;
use crate::{app::state::AppState, app::widget::Loader};

pub fn renderMarkDown<'a>(cx: Scope<'a>, string: &'a str) -> Element<'a> {
    cx.render(rsx! {
        div {
            Markdown {
                class: "prose",
                content: string,
            }
        }
    })
}

/// Nix health checks
pub fn Health(cx: Scope) -> Element {
    let state = AppState::use_state(cx);
    let health_checks = state.health_checks.read();
    let title = "Nix Health";
    render! {
        renderMarkDown(cx, "# Markdown Working")
        h1 { class: "text-5xl font-bold", title }
        if health_checks.is_loading_or_refreshing() {
            render! { Loader {} }
        }
        health_checks.render_with(cx, |checks| render! {
            div { class: "flex flex-col items-stretch justify-start space-y-8 text-left",
                for check in checks {
                    println!("{:?}", check)
                        ViewCheck { check: check.clone() }
                        renderMarkDown(cx, "# hello \n - ok")
                }
            }
        })
    }
}



#[component]
fn ViewCheck(cx: Scope, check: Check) -> Element {
    render! {
        div { class: "contents",
            details {
                open: check.result != CheckResult::Green,
                class: "my-2 bg-white border-2 rounded-lg cursor-pointer hover:bg-primary-100 border-base-300",
                summary { class: "p-4 text-xl font-bold",
                    CheckResultSummaryView { green: check.result.green() }
                    " "
                    check.title.clone()
                }
                div { class: "p-4",
                    div { class: "p-2 my-2 font-mono text-sm bg-black text-base-100", check.info.clone() }
                    div { class: "flex flex-col justify-start space-y-4",
                        match check.result.clone() {
                            CheckResult::Green => render! { "" },
                            CheckResult::Red { msg, suggestion } => render! {
                                h3 { class: "my-2 font-bold text-l" }
                                div { class: "p-2 bg-red-400 rounded bg-border", msg }
                                h3 { class: "my-2 font-bold text-l" }
                                div { class: "p-2 bg-blue-400 rounded bg-border", suggestion }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CheckResultSummaryView(cx: Scope, green: bool) -> Element {
    if *green {
        render! { span { class: "text-green-500", "✓" } }
    } else {
        render! { span { class: "text-red-500", "✗" } }
    }
}
