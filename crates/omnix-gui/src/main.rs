#![feature(let_chains)]
use dioxus::prelude::*;
use dioxus_desktop::{LogicalSize, WindowBuilder};

mod app;
mod cli;

fn main() {
    use clap::Parser;
    let args = crate::cli::Args::parse();
    omnix_common::logging::setup_logging(&args.verbosity, false);

    // Set data directory for persisting [Signal]s. On macOS, this is ~/Library/Application Support/omnix-gui.
    dioxus_sdk::storage::set_dir!();

    let config = dioxus_desktop::Config::new()
        .with_custom_head(r#" <link rel="stylesheet" href="tailwind.css"> "#.to_string())
        .with_window(
            WindowBuilder::new()
                .with_title("Omnix")
                .with_inner_size(LogicalSize::new(800, 700)),
        );
    LaunchBuilder::desktop().with_cfg(config).launch(app::App)
}
