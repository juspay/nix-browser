use serde::{Deserialize, Serialize};

/// Types that can do specific "health check" for Nix
#[cfg(feature = "ssr")]
pub trait Checkable {
    /// Run and create the health check
    ///
    /// NOTE: Some checks may perform impure actions (IO, etc.). Returning an
    /// empty vector indicates that the check is skipped on this environment.
    fn check(&self, nix_info: &nix_rs::info::NixInfo, nix_env: &nix_rs::env::NixEnv) -> Vec<Check>;
}

/// A health check
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Check {
    /// A user-facing title of this check
    ///
    /// This value is expected to be unique across all checks.
    pub title: String,

    /// The user-facing information used to conduct this check
    /// TODO: Use Markdown
    pub info: String,

    /// The result of running this check
    pub result: CheckResult,

    /// Whether this check is mandatory
    ///
    /// Failures are considered non-critical if this is false.
    pub required: bool,
}

/// The result of a health [Check]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum CheckResult {
    /// The check passed
    Green,

    /// The check failed
    Red {
        /// TODO: Use markdown
        msg: String,
        /// TODO: Use markdown
        suggestion: String,
    },
}

impl CheckResult {
    /// When the check is green (ie., healthy)
    pub fn green(&self) -> bool {
        matches!(self, Self::Green)
    }

    /// Chain another [CheckResult]
    pub fn chain(&self, other: Self) -> Self {
        match (self, &other) {
            (Self::Green, Self::Green) => Self::Green,
            (Self::Green, Self::Red { .. }) => other.clone(),
            (Self::Red { .. }, Self::Green) => self.clone(),
            (
                Self::Red { msg, suggestion },
                Self::Red {
                    msg: msg2,
                    suggestion: suggestion2,
                },
            ) => Self::Red {
                // TODO: Once we have Markdown, format this accordingly
                msg: format!("{}. {}", msg, msg2),
                suggestion: format!("{}. {}", suggestion, suggestion2),
            },
        }
    }
}
