//! M3U8 のパースおよびビルド時のエラー型を提供する

use std::fmt;

/// M3U8 のパースおよびビルド時のエラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// エラーの種別を返す
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for Error {}

/// エラーの種別
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// `#EXTM3U` ヘッダーがない
    MissingHeader,
    /// 必須タグが欠けている
    MissingTag {
        /// 欠けているタグ名
        tag: &'static str,
    },
    /// タグの値が不正
    InvalidTagValue {
        /// 値が不正なタグ名
        tag: &'static str,
    },
    /// 属性の値が不正
    InvalidAttributeValue {
        /// 値が不正な属性名
        attribute: &'static str,
    },
    /// 不正な URI
    InvalidUri,
    /// 予期しない EOF
    UnexpectedEof,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => write!(f, "missing #EXTM3U header"),
            Self::MissingTag { tag } => write!(f, "missing required tag: {tag}"),
            Self::InvalidTagValue { tag } => write!(f, "invalid value for tag: {tag}"),
            Self::InvalidAttributeValue { attribute } => {
                write!(f, "invalid value for attribute: {attribute}")
            }
            Self::InvalidUri => write!(f, "invalid URI"),
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
        }
    }
}

/// `Result` 型エイリアス
pub type Result<T> = std::result::Result<T, Error>;
