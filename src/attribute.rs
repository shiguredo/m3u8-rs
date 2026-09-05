//! 属性リスト (`KEY=VALUE`) のパース・生成を提供する

use crate::error::{Error, ErrorKind, Result};
use std::collections::HashSet;

/// 属性リスト文字列 (`KEY=VALUE,KEY="VALUE",...`) をパースして
/// キーに対応する値の文字列スライスを返す
pub(crate) fn get_attribute<'a>(attrs: &'a str, key: &'static str) -> Result<Option<&'a str>> {
    for entry in parse_attribute_list(attrs)? {
        if entry.name == key {
            return Ok(Some(entry.value));
        }
    }
    Ok(None)
}

/// 属性リストから必須属性を取得し、なければエラーを返す
pub(crate) fn require_attribute<'a>(attrs: &'a str, key: &'static str) -> Result<&'a str> {
    get_attribute(attrs, key)?.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidAttributeValue { attribute: key },
            format!("missing required attribute: {key}"),
        )
    })
}

/// 属性リストから `u64` 値を取得する
pub(crate) fn parse_u64_attribute(attrs: &str, key: &'static str) -> Result<Option<u64>> {
    let Some(val) = get_attribute(attrs, key)? else {
        return Ok(None);
    };
    val.parse::<u64>()
        .map(Some)
        .map_err(|_| Error::new(ErrorKind::InvalidAttributeValue { attribute: key }, val))
}

/// 属性リストから `f64` 値を取得する
pub(crate) fn parse_f64_attribute(attrs: &str, key: &'static str) -> Result<Option<f64>> {
    let Some(val) = get_attribute(attrs, key)? else {
        return Ok(None);
    };
    val.parse::<f64>()
        .map(Some)
        .map_err(|_| Error::new(ErrorKind::InvalidAttributeValue { attribute: key }, val))
}

/// 属性リストから `YES`/`NO` の bool 値を取得する
pub(crate) fn parse_bool_attribute(attrs: &str, key: &'static str) -> Result<bool> {
    match get_attribute(attrs, key)? {
        Some("YES") => Ok(true),
        Some("NO") => Ok(false),
        Some(value) => Err(Error::new(
            ErrorKind::InvalidAttributeValue { attribute: key },
            value,
        )),
        None => Ok(false),
    }
}

/// quoted-string に安全な文字列へ正規化する
pub(crate) fn write_quoted_string(value: &str) -> String {
    let sanitized = value
        .chars()
        .filter(|c| !matches!(c, '"' | '\n' | '\r'))
        .collect::<String>();
    format!("\"{sanitized}\"")
}

/// 属性リストを name / value / quoted 付きでパースする
pub(crate) fn parse_attribute_entries(attrs: &str) -> Result<Vec<AttributeEntry<'_>>> {
    parse_attribute_list(attrs)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AttributeEntry<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub quoted: bool,
}

/// `WxH` 形式の解像度文字列をパースする
pub(crate) fn parse_resolution(s: &str) -> Result<crate::multivariant::Resolution> {
    let mut parts = s.splitn(2, 'x');
    let w = parts
        .next()
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "RESOLUTION",
                },
                s,
            )
        })?;
    let h = parts
        .next()
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "RESOLUTION",
                },
                s,
            )
        })?;
    Ok(crate::multivariant::Resolution {
        width: w,
        height: h,
    })
}

/// `n@o` または `n` 形式のバイトレンジをパースする
pub(crate) fn parse_byterange(s: &str) -> Result<crate::media::ByteRange> {
    if let Some((length, offset)) = s.split_once('@') {
        let length = length.parse::<u64>().map_err(|_| {
            Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-BYTERANGE",
                },
                s,
            )
        })?;
        let offset = offset.parse::<u64>().map_err(|_| {
            Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-BYTERANGE",
                },
                s,
            )
        })?;
        Ok(crate::media::ByteRange {
            length,
            offset: Some(offset),
        })
    } else {
        let length = s.parse::<u64>().map_err(|_| {
            Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-BYTERANGE",
                },
                s,
            )
        })?;
        Ok(crate::media::ByteRange {
            length,
            offset: None,
        })
    }
}

fn parse_attribute_list(attrs: &str) -> Result<Vec<AttributeEntry<'_>>> {
    let mut pairs = Vec::new();
    let mut seen = HashSet::new();
    let bytes = attrs.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let name_start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_uppercase() || bytes[i].is_ascii_digit() || bytes[i] == b'-')
        {
            i += 1;
        }
        if i == name_start {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "attribute-list",
                },
                "invalid attribute name",
            ));
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "attribute-list",
                },
                "expected '=' in attribute list",
            ));
        }
        let name = &attrs[name_start..i];
        if !seen.insert(name) {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "attribute-list",
                },
                format!("duplicate attribute name: {name}"),
            ));
        }
        i += 1;
        if i >= bytes.len() {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "attribute-list",
                },
                format!("missing value for attribute: {name}"),
            ));
        }

        let (value, quoted) = if bytes[i] == b'"' {
            i += 1;
            let value_start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                if matches!(bytes[i], b'\n' | b'\r') {
                    return Err(Error::new(
                        ErrorKind::InvalidTagValue {
                            tag: "attribute-list",
                        },
                        format!("quoted-string contains invalid character in attribute: {name}"),
                    ));
                }
                i += 1;
            }
            if i >= bytes.len() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "attribute-list",
                    },
                    format!("unterminated quoted-string in attribute: {name}"),
                ));
            }
            let value = &attrs[value_start..i];
            i += 1;
            (value, true)
        } else {
            let value_start = i;
            while i < bytes.len() && bytes[i] != b',' {
                if bytes[i].is_ascii_whitespace() {
                    return Err(Error::new(
                        ErrorKind::InvalidTagValue {
                            tag: "attribute-list",
                        },
                        format!("attribute value contains whitespace: {name}"),
                    ));
                }
                i += 1;
            }
            if i == value_start {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "attribute-list",
                    },
                    format!("missing value for attribute: {name}"),
                ));
            }
            (&attrs[value_start..i], false)
        };

        pairs.push(AttributeEntry {
            name,
            value,
            quoted,
        });

        if i == bytes.len() {
            break;
        }
        if bytes[i] != b',' {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "attribute-list",
                },
                "expected ',' in attribute list",
            ));
        }
        i += 1;
        if i >= bytes.len() {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "attribute-list",
                },
                "attribute list must not end with ','",
            ));
        }
    }
    Ok(pairs)
}
