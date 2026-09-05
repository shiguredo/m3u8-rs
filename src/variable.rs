//! `#EXT-X-DEFINE` の変数定義型を提供する

/// `#EXT-X-DEFINE` の定義
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableDefinition {
    /// `NAME` / `VALUE`
    Name {
        /// 変数名
        name: String,
        /// 変数の値
        value: String,
    },
    /// `IMPORT`
    Import {
        /// 取り込む変数名
        name: String,
        /// 取り込んだ変数の値
        value: String,
    },
    /// `QUERYPARAM`
    QueryParam {
        /// クエリパラメータ名
        name: String,
        /// 解決したクエリパラメータの値
        value: String,
    },
}

impl VariableDefinition {
    /// 変数名を返す
    pub fn name(&self) -> &str {
        match self {
            Self::Name { name, .. } | Self::Import { name, .. } | Self::QueryParam { name, .. } => {
                name
            }
        }
    }

    /// 変数の値を返す
    pub fn value(&self) -> &str {
        match self {
            Self::Name { value, .. }
            | Self::Import { value, .. }
            | Self::QueryParam { value, .. } => value,
        }
    }
}
