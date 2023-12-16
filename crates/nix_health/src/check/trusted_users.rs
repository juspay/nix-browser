use std::collections::HashSet;

use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};

use crate::traits::*;

/// Check that [crate::nix::config::NixConfig::trusted_users] is set to a good value.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TrustedUsers {}

impl Checkable for TrustedUsers {
    fn check(
        &self,
        nix_info: &nix_rs::info::NixInfo,
        _: Option<nix_rs::flake::url::FlakeUrl>,
    ) -> Vec<Check> {
        let val = &nix_info.nix_config.trusted_users.value;
        let current_user = &nix_info.nix_env.current_user;
        let current_user_groups: HashSet<&String> =
            nix_info.nix_env.current_user_groups.iter().collect();
        let (val_groups, val_users): (Vec<String>, Vec<String>) =
            val.iter().partition_map(|x| match x.strip_prefix('@') {
                Some(x) => Either::Left(x.to_string()),
                None => Either::Right(x.clone()),
            });
        let result = if val_users.contains(current_user)
            || val_groups.iter().any(|x| current_user_groups.contains(&x))
        {
            CheckResult::Green
        } else {
            let msg = format!("User '{}' not present in trusted_users", current_user);
            let suggestion = match nix_info.nix_env.os.nix_system_config_label() {
                Some(conf_label) => format!(
                    r#"Add `nix.trustedUsers = [ "root" "{}" ];` to your {}"#,
                    current_user, conf_label,
                ),
                None => format!(
                    r#"Set `trusted-users = root {}` in /etc/nix/nix.conf and then restart the Nix daemon using `sudo pkill nix-daemon`"#,
                    current_user
                ),
            };
            CheckResult::Red { msg, suggestion }
        };
        let check = Check {
            title: "Trusted Users".to_string(),
            info: format!("trusted-users = {}", val.join(" ")),
            result,
            required: true,
        };
        vec![check]
    }
}
