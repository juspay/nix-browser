#![feature(let_chains)]
use dioxus_desktop::{LogicalSize, WindowBuilder};

mod app;
mod cli;
mod logging;

#[tokio::main]
async fn main() {
    use clap::Parser;
    let args = crate::cli::Args::parse();
    crate::logging::setup_logging(&args.verbosity);

    // Set data directory for persisting [Signal]s. On macOS, this is ~/Library/Application Support/nix-browser.
    dioxus_std::storage::set_dir!();

    dioxus_desktop::launch_cfg(
        app::App,
        dioxus_desktop::Config::new()
            .with_custom_head(r#" <link rel="stylesheet" href="assets/tailwind.css"> "#.to_string())
            .with_window(
                WindowBuilder::new()
                    .with_title("Nix Browser")
                    .with_inner_size(LogicalSize::new(800, 700)),
            ),
    )
}
