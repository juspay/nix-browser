use nix_rs::{config::ConfigVal, info, system};

use std::fmt::Display;

use os_info;
use serde::{Deserialize, Serialize};

use crate::{
    report::{Report, WithDetails},
    traits::Check,
};

/// Check that [crate::nix::config::NixConfig::trusted_users] is set to a good value.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrustedUsers {
    pub trusted_users: ConfigVal<Vec<String>>,
    sys_info: system::SysInfo,
}

impl Check for TrustedUsers {
    fn check(nix_info: &info::NixInfo, sys_info: &system::SysInfo) -> Self {
        TrustedUsers {
            trusted_users: nix_info.nix_config.trusted_users.clone(),
            sys_info: sys_info.clone(),
        }
    }
    fn name(&self) -> &'static str {
        "Trusted users"
    }
    fn report(&self) -> Report<WithDetails> {
        let trusted_users = &self.trusted_users.value;
        let current_user = &self.sys_info.current_user;
        let os = self.sys_info.os;
        let uses_nix_darwin = self.sys_info.uses_nix_darwin;
        if trusted_users.contains(current_user) {
            Report::Green
        } else if os == os_info::Type::NixOS || uses_nix_darwin {
            Report::Red(WithDetails {
                msg: format!("{} not present in trusted_users", current_user),
                suggestion: format!(
                    r#"Add `nix.trustedUsers = [ "root" "{}" ];` to your `configuration.nix`"#,
                    current_user
                ),
            })
        } else {
            Report::Red(WithDetails {
                msg: format!("{} not present in trusted_users", current_user),
                suggestion: format!(
                    r#"Run 'echo "trusted-users = root {}" | sudo tee -a /etc/nix/nix.conf && sudo pkill nix-daemon'"#,
                    current_user
                ),
            })
        }
    }
}

impl Display for TrustedUsers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "trusted_users = {}", self.trusted_users.value.join(" "))
    }
}
