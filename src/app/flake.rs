//! UI for /flake segment of the app

use std::{collections::BTreeMap, path::PathBuf};

use dioxus::prelude::*;
use dioxus_router::prelude::Link;
use nix_rs::flake::{
    outputs::{FlakeOutputs, Type, Val},
    schema::FlakeSchema,
    url::FlakeUrl,
    Flake,
};

use crate::{
    app::widget::FolderDialogButton,
    app::{state::AppState, widget::Loader, Route},
};

#[component]
pub fn Flake(cx: Scope) -> Element {
    let state = AppState::use_state(cx);
    let flake = state.flake.read();
    let busy = (*flake).is_loading_or_refreshing();
    render! {
        h1 { class: "text-5xl font-bold", "Flake browser" }
        div { class: "p-2 my-1 flex w-full",
            input {
                class: "flex-1 w-full p-1 mb-4 font-mono",
                id: "nix-flake-input",
                "type": "text",
                value: "{state.get_flake_url_string()}",
                disabled: busy,
                onchange: move |ev| {
                    let url: FlakeUrl = ev.value.clone().into();
                    state.set_flake_url(url);
                }
            }
            div { class: "ml-2 flex flex-col",
                FolderDialogButton {
                    handler: move |flake_path: PathBuf| {
                        let url: FlakeUrl = flake_path.into();
                        state.set_flake_url(url);
                    }
                }
            }
        }
        if flake.is_loading_or_refreshing() {
            render! { Loader {} }
        }
        flake.render_with(cx, |v| render! { FlakeView { flake: v.clone() } })
    }
}

#[component]
pub fn FlakeRaw(cx: Scope) -> Element {
    let state = AppState::use_state(cx);
    // use_future(cx, (), |_| async move { state.update_flake().await });
    let flake = state.flake.read();
    render! {
        div {
            Link { to: Route::Flake {}, "⬅ Back" }
            div { class: "px-4 py-2 font-mono text-xs text-left text-gray-500 border-2 border-black",
                flake.render_with(cx, |v| render! { FlakeOutputsRawView { outs: v.output.clone() } } )
            }
        }
    }
}

#[component]
pub fn FlakeView(cx: Scope, flake: Flake) -> Element {
    render! {
        div { class: "flex flex-col my-4",
            h3 { class: "text-lg font-bold", flake.url.to_string() }
            div { class: "text-sm italic text-gray-600",
                Link { to: Route::FlakeRaw {}, "View raw output" }
            }
            FlakeSchemaView { schema: &flake.schema }
        }
    }
}

#[component]
pub fn SectionHeading(cx: Scope, title: &'static str, extra: Option<String>) -> Element {
    render! {
        h3 { class: "p-2 mt-4 mb-2 font-bold bg-gray-300 border-b-2 border-l-2 border-black text-l",
            "{title}"
            match extra {
                Some(v) => render! { span { class: "text-xs text-gray-500 ml-1", "(", "{v}", ")" } },
                None => render! { "" }
            }
        }
    }
}

#[component]
pub fn FlakeSchemaView<'a>(cx: Scope, schema: &'a FlakeSchema) -> Element {
    let system = schema.system.clone();
    render! {
        div {
            h2 { class: "my-2",
                div { class: "text-xl font-bold text-primary-600", "{system.human_readable()}" }
                span { class: "font-mono text-xs text-gray-500", "(", "{system }", ")" }
            }
            div { class: "text-left",
                BtreeMapView { title: "Packages", tree: &schema.packages }
                BtreeMapView { title: "Legacy Packages", tree: &schema.legacy_packages }
                BtreeMapView { title: "Dev Shells", tree: &schema.devshells }
                BtreeMapView { title: "Checks", tree: &schema.checks }
                BtreeMapView { title: "Apps", tree: &schema.apps }
                SectionHeading { title: "Formatter" }
                match schema.formatter.as_ref() {
                    Some(v) => {
                        let k = v.name.clone().unwrap_or("formatter".to_string());
                        render! { FlakeValView { k: k.clone(), v: v.clone() } }
                    },
                    None => render! { "" }
                },
                SectionHeading { title: "Other" }
                match &schema.other {
                    Some(v) => render! { FlakeOutputsRawView { outs: FlakeOutputs::Attrset(v.clone()) } },
                    None => render! { "" }
                }
            }
        }
    }
}

#[component]
pub fn BtreeMapView<'a>(
    cx: Scope,
    title: &'static str,
    tree: &'a BTreeMap<String, Val>,
) -> Element {
    render! {
        div {
            SectionHeading { title: title, extra: tree.len().to_string() }
            BtreeMapBodyView { tree: tree }
        }
    }
}

#[component]
pub fn BtreeMapBodyView<'a>(cx: Scope, tree: &'a BTreeMap<String, Val>) -> Element {
    render! {
        div { class: "flex flex-wrap justify-start",
            for (k , v) in tree.iter() {
                FlakeValView { k: k.clone(), v: v.clone() }
            }
        }
    }
}

#[component]
pub fn FlakeValView(cx: Scope, k: String, v: Val) -> Element {
    render! {
        div {
            title: "{v.type_}",
            class: "flex flex-col p-2 my-2 mr-2 space-y-2 bg-white border-4 border-gray-300 rounded hover:border-gray-400",
            div { class: "flex flex-row justify-start space-x-2 font-bold text-primary-500",
                div { v.type_.to_icon() }
                div { "{k}" }
            }
            match &v.name {
                Some(name_val) => render! { div { class: "font-mono text-xs text-gray-500", "{name_val}" } },
                None => render! { "" } // No-op for None
            },
            match &v.description {
                Some(desc_val) => render! { div { class: "font-light", "{desc_val}" } },
                None => render! { "" } // No-op for None
            }
        }
    }
}

/// This component renders recursively. This view is used to see the raw flake
/// output only; it is not useful for general UX.
///
/// WARNING: This may cause performance problems if the tree is large.
#[component]
pub fn FlakeOutputsRawView(cx: Scope, outs: FlakeOutputs) -> Element {
    #[component]
    fn ValView<'a>(cx: Scope, val: &'a Val) -> Element {
        render! {
            span {
                b { val.name.clone() }
                " ("
                TypeView { type_: &val.type_ }
                ") "
                em { val.description.clone() }
            }
        }
    }

    #[component]
    pub fn TypeView<'a>(cx: Scope, type_: &'a Type) -> Element {
        render! {
            span {
                match type_ {
                    Type::NixosModule => "nixosModule ❄️",
                    Type::Derivation => "derivation 📦",
                    Type::App => "app 📱",
                    Type::Template => "template 🏗️",
                    Type::Unknown => "unknown ❓",
                }
            }
        }
    }

    match outs {
        FlakeOutputs::Val(v) => render! { ValView { val: v } },
        FlakeOutputs::Attrset(v) => render! {
            ul { class: "list-disc",
                for (k , v) in v.iter() {
                    li { class: "ml-4",
                        span { class: "px-2 py-1 font-bold text-primary-500", "{k}" }
                        FlakeOutputsRawView { outs: v.clone() }
                    }
                }
            }
        },
    }
}
