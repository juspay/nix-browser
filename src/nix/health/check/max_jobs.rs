use leptos::*;
use serde::{Deserialize, Serialize};

use crate::nix::{
    config::ConfigVal,
    health::{
        check::ViewCheck,
        report::{Report, WithDetails},
        traits::Check,
    },
    info,
};

/// Check that [crate::nix::config::NixConfig::max_jobs] is set to a good value.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MaxJobs(ConfigVal<i32>);

impl Check for MaxJobs {
    fn check(info: &info::NixInfo) -> Self {
        MaxJobs(info.nix_config.max_jobs.clone())
    }
    fn name(&self) -> &'static str {
        "Max Jobs"
    }
    fn report(&self) -> Report<WithDetails> {
        if self.0.value > 1 {
            Report::Green
        } else {
            Report::Red(WithDetails {
                msg: "You are using only 1 core for nix builds",
                suggestion: "Try editing /etc/nix/nix.conf",
            })
        }
    }
}

impl IntoView for MaxJobs {
    fn into_view(self, cx: Scope) -> View {
        view! { cx,
            <ViewCheck check=self.clone()>
                <span>{self.0} " Cores"</span>
            </ViewCheck>
        }
    }
}
