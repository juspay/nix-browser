use serde::{Deserialize, Serialize};

/// The attribute output part of a [super::FlakeUrl]
///
/// Example: `foo` in `.#foo`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlakeAttr(pub Option<String>);

impl FlakeAttr {
    /// Get the attribute name.
    ///
    /// If attribute exists, then return "default".
    pub fn get_name(&self) -> String {
        self.0.clone().unwrap_or_else(|| "default".to_string())
    }

    /// Whether an explicit attribute is set
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Return nested attrs if the user specified one is separated by '.'
    pub fn as_list(&self) -> Vec<String> {
        self.0
            .clone()
            .map(|s| s.split('.').map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }
}
