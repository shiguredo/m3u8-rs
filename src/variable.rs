/// `#EXT-X-DEFINE` の定義
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VariableDefinition {
    /// `NAME` / `VALUE`
    Name { name: String, value: String },
    /// `IMPORT`
    Import { name: String, value: String },
    /// `QUERYPARAM`
    QueryParam { name: String, value: String },
}

impl VariableDefinition {
    pub fn name(&self) -> &str {
        match self {
            Self::Name { name, .. } | Self::Import { name, .. } | Self::QueryParam { name, .. } => {
                name
            }
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Self::Name { value, .. }
            | Self::Import { value, .. }
            | Self::QueryParam { value, .. } => value,
        }
    }
}
