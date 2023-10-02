//! Frontend UI entry point

// Workaround for https://github.com/rust-lang/rust-analyzer/issues/15344
#![allow(non_snake_case)]

mod flake;
mod health;
mod info;

use leptos::*;
use leptos_extra::{
    query::{self},
    signal::{provide_signal, SignalWithResult},
};
use leptos_meta::*;
use leptos_query::*;
use leptos_router::*;
use nix_rs::{command::Refresh, flake::url::FlakeUrl};

use crate::{app::flake::*, app::health::*, app::info::*, widget::*};

/// Main frontend application container
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_query_client();
    provide_signal::<FlakeUrl>(
        FlakeUrl::suggestions()
            .first()
            .map(Clone::clone)
            .unwrap_or_default(),
    );
    provide_signal::<Refresh>(false.into()); // refresh flag is unused, but we may add it to UI later.

    view! {
        <Stylesheet id="leptos" href="/pkg/nix-browser.css"/>
        <Title formatter=|s| format!("{s} ― nix-browser")/>
        <Router fallback=|| {
            view! { <NotFound/> }
        }>
            <Body class="overflow-y-scroll"/>
            <div class="flex justify-center w-full min-h-screen bg-center bg-cover bg-base-200">
                <div class="flex flex-col items-stretch mx-auto sm:container sm:max-w-screen-md">
                    <Nav/>
                    <main class="flex flex-col px-2 mb-8 space-y-3 text-center">
                        <Routes>
                            <Route path="" view=Dashboard/>
                            <Route path="/flake" view=NixFlakeRoute>
                                <Route path="" view=NixFlakeHomeRoute/>
                                <Route path="raw" view=NixFlakeRawRoute/>
                            </Route>
                            <Route path="/health" view=NixHealthRoute/>
                            <Route path="/info" view=NixInfoRoute/>
                            <Route path="/about" view=About/>
                        </Routes>
                    </main>
                </div>
            </div>
        </Router>
    }
}

/// Navigation bar
///
/// TODO Switch to breadcrumbs, as it simplifes the design overall.
#[component]
fn Nav() -> impl IntoView {
    let class = "px-3 py-2";
    view! {
        <nav class="flex flex-row w-full mb-8 text-white md:rounded-b bg-primary-800">
            <A exact=true href="/" class=class>
                "Dashboard"
            </A>
            <A exact=false href="/flake" class=class>
                "Flake"
            </A>
            <A exact=true href="/health" class=class>
                "Nix Health"
            </A>
            <A exact=true href="/info" class=class>
                "Nix Info"
            </A>
            <A exact=true href="/about" class=class>
                "About"
            </A>
            <div class=format!("flex-grow font-bold text-end {}", class)>"🌍 nix-browser"</div>
        </nav>
    }
}

/// Home page
#[component]
fn Dashboard() -> impl IntoView {
    tracing::debug!("Rendering Dashboard page");
    let result = query::use_server_query(|| (), get_nix_health);
    let data = result.data;
    let healthy = Signal::derive(move || {
        data.with_result(|checks| checks.iter().all(|check| check.result.green()))
    });
    // A Card component
    #[component]
    fn Card(href: &'static str, children: Children) -> impl IntoView {
        view! {
            <A
                href=href
                class="flex items-center justify-center w-48 h-48 p-2 m-2 border-2 rounded-lg shadow border-base-400 active:shadow-none bg-base-100 hover:bg-primary-200"
            >
                <span class="text-3xl text-base-800">{children()}</span>
            </A>
        }
    }
    view! {
        <Title text="Dashboard"/>
        <h1 class="text-5xl font-bold">"Dashboard"</h1>
        <div id="cards" class="flex flex-row flex-wrap">
            <SuspenseWithErrorHandling>
                <Card href="/health">
                    "Nix Health Check "
                    {move || {
                        healthy
                            .with_result(move |green| {
                                view! { <CheckResultSummaryView green=*green/> }
                            })
                    }}

                </Card>
            </SuspenseWithErrorHandling>
            <Card href="/info">"Nix Info ℹ️"</Card>
            <Card href="/flake">"Flake Overview ❄️️"</Card>
        </div>
    }
}

/// About page
#[component]
fn About() -> impl IntoView {
    view! {
        <Title text="About"/>
        <h1 class="text-5xl font-bold">"About"</h1>
        <p>
            "nix-browser is still work in progress. Track its development "
            <LinkExternal link="https://github.com/juspay/nix-browser" text="on Github"/>
        </p>
    }
}
