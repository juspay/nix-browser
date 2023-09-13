use nix_rs::{config::ConfigVal, info, system};

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{
    report::{Report, WithDetails},
    traits::Check,
};

/// Check that [nix_rs::config::NixConfig::experimental_features] is set to a good value.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlakeEnabled(pub ConfigVal<Vec<String>>);

impl Check for FlakeEnabled {
    fn check(nix_info: &info::NixInfo, _sys_info: &system::SysInfo) -> Self {
        FlakeEnabled(nix_info.nix_config.experimental_features.clone())
    }
    fn name(&self) -> &'static str {
        "Flakes Enabled"
    }
    fn report(&self) -> Report<WithDetails> {
        let val = &self.0.value;
        if val.contains(&"flakes".to_string()) && val.contains(&"nix-command".to_string()) {
            Report::Green
        } else {
            Report::Red(WithDetails {
                msg: "Nix flakes are not enabled".into(),
                suggestion: "See https://nixos.wiki/wiki/Flakes#Enable_flakes".into(),
            })
        }
    }
}

impl Display for FlakeEnabled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "experimental-features = {}", self.0.value.join(" "))
    }
}
