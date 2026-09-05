//! Multivariant Playlist のパース処理を提供する

use crate::{
    attribute::{
        get_attribute, parse_bool_attribute, parse_f64_attribute, parse_resolution,
        parse_u64_attribute, require_attribute,
    },
    error::{Error, ErrorKind, Result},
    multivariant::{
        ClosedCaptions, ContentSteering, EncryptionMethod, HdcpLevel, IFrameStream, Key, MediaType,
        MultivariantPlaylist, Rendition, SessionData, SessionDataValue, StartPoint, VariantStream,
        VideoRange,
    },
    variable::VariableDefinition,
};
use std::collections::{HashMap, HashSet};

/// Multivariant Playlist をパースする
pub fn parse(input: &str, playlist_uri: Option<&str>) -> Result<MultivariantPlaylist> {
    if !is_valid_extm3u_header(input) {
        return Err(Error::new(
            ErrorKind::MissingHeader,
            "playlist must start with #EXTM3U",
        ));
    }

    let mut playlist = MultivariantPlaylist {
        version: None,
        independent_segments: false,
        start: None,
        variable_definitions: Vec::new(),
        content_steering: None,
        variant_streams: Vec::new(),
        renditions: Vec::new(),
        i_frame_streams: Vec::new(),
        session_data: Vec::new(),
        session_keys: Vec::new(),
    };
    let mut variables = HashMap::new();
    let mut local_variable_names = HashSet::new();

    let mut lines = input.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() || line == "#EXTM3U" {
            continue;
        }
        if let Some(attrs) = line.strip_prefix("#EXT-X-DEFINE:") {
            let definition = parse_variable_definition(
                attrs,
                PlaylistKind::Multivariant,
                &variables,
                playlist_uri,
            )?;
            if !local_variable_names.insert(definition.name().to_owned()) {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-DEFINE",
                    },
                    format!("duplicate variable definition: {}", definition.name()),
                ));
            }
            variables.insert(definition.name().to_owned(), definition.value().to_owned());
            playlist.variable_definitions.push(definition);
            continue;
        }
        let line = substitute_variables_in_line(line, &variables)?;
        reject_media_only_tag(&line)?;
        if let Some(value) = line.strip_prefix("#EXT-X-VERSION:") {
            if playlist.version.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-VERSION",
                    },
                    "EXT-X-VERSION must not appear more than once",
                ));
            }
            playlist.version = Some(value.trim().parse::<u8>().map_err(|_| {
                Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-VERSION",
                    },
                    value,
                )
            })?);
        } else if line == "#EXT-X-INDEPENDENT-SEGMENTS" {
            if playlist.independent_segments {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-INDEPENDENT-SEGMENTS",
                    },
                    "EXT-X-INDEPENDENT-SEGMENTS must not appear more than once",
                ));
            }
            playlist.independent_segments = true;
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-START:") {
            if playlist.start.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue { tag: "EXT-X-START" },
                    "EXT-X-START must not appear more than once",
                ));
            }
            playlist.start = Some(parse_start(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-CONTENT-STEERING:") {
            if playlist.content_steering.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-CONTENT-STEERING",
                    },
                    "EXT-X-CONTENT-STEERING must not appear more than once",
                ));
            }
            playlist.content_steering = Some(parse_content_steering(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-MEDIA:") {
            playlist.renditions.push(parse_rendition(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let uri = lines
                .next()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::UnexpectedEof,
                        "expected URI after #EXT-X-STREAM-INF",
                    )
                })?;
            let uri = substitute_variables_in_line(uri, &variables)?;
            playlist
                .variant_streams
                .push(parse_variant_stream(attrs, &uri)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-I-FRAME-STREAM-INF:") {
            playlist.i_frame_streams.push(parse_i_frame_stream(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-SESSION-DATA:") {
            playlist.session_data.push(parse_session_data(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-SESSION-KEY:") {
            playlist.session_keys.push(parse_key(attrs)?);
        }
        // 未知のタグは無視する
    }

    validate_multivariant_playlist(&playlist)?;
    validate_content_steering(&playlist)?;
    validate_multivariant_version(&playlist)?;

    Ok(playlist)
}

fn reject_media_only_tag(line: &str) -> Result<()> {
    let tag = if line.starts_with("#EXTINF:") {
        Some("EXTINF")
    } else if line.starts_with("#EXT-X-TARGETDURATION:") {
        Some("EXT-X-TARGETDURATION")
    } else if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") {
        Some("EXT-X-MEDIA-SEQUENCE")
    } else if line.starts_with("#EXT-X-DISCONTINUITY-SEQUENCE:") {
        Some("EXT-X-DISCONTINUITY-SEQUENCE")
    } else if line.starts_with("#EXT-X-PLAYLIST-TYPE:") {
        Some("EXT-X-PLAYLIST-TYPE")
    } else if line == "#EXT-X-I-FRAMES-ONLY" {
        Some("EXT-X-I-FRAMES-ONLY")
    } else if line.starts_with("#EXT-X-SERVER-CONTROL:") {
        Some("EXT-X-SERVER-CONTROL")
    } else if line.starts_with("#EXT-X-PART-INF:") {
        Some("EXT-X-PART-INF")
    } else if line.starts_with("#EXT-X-SKIP:") {
        Some("EXT-X-SKIP")
    } else if line == "#EXT-X-ENDLIST" {
        Some("EXT-X-ENDLIST")
    } else if line == "#EXT-X-DISCONTINUITY" {
        Some("EXT-X-DISCONTINUITY")
    } else if line == "#EXT-X-GAP" {
        Some("EXT-X-GAP")
    } else if line.starts_with("#EXT-X-BITRATE:") {
        Some("EXT-X-BITRATE")
    } else if line.starts_with("#EXT-X-BYTERANGE:") {
        Some("EXT-X-BYTERANGE")
    } else if line.starts_with("#EXT-X-KEY:") {
        Some("EXT-X-KEY")
    } else if line.starts_with("#EXT-X-MAP:") {
        Some("EXT-X-MAP")
    } else if line.starts_with("#EXT-X-PROGRAM-DATE-TIME:") {
        Some("EXT-X-PROGRAM-DATE-TIME")
    } else if line.starts_with("#EXT-X-DATERANGE:") {
        Some("EXT-X-DATERANGE")
    } else if line.starts_with("#EXT-X-PART:") {
        Some("EXT-X-PART")
    } else if line.starts_with("#EXT-X-PRELOAD-HINT:") {
        Some("EXT-X-PRELOAD-HINT")
    } else if line.starts_with("#EXT-X-RENDITION-REPORT:") {
        Some("EXT-X-RENDITION-REPORT")
    } else {
        None
    };

    if let Some(tag) = tag {
        return Err(Error::new(
            ErrorKind::InvalidTagValue { tag },
            format!("{tag} must not appear in a Multivariant Playlist"),
        ));
    }

    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) enum PlaylistKind {
    Multivariant,
    Media,
}

fn parse_start(attrs: &str) -> Result<StartPoint> {
    parse_start_inner(attrs)
}

/// Media Playlist パーサーからも参照できるよう pub(crate) で公開する
pub(crate) fn parse_start_inner(attrs: &str) -> Result<StartPoint> {
    let time_offset = parse_f64_attribute(attrs, "TIME-OFFSET")?.ok_or_else(|| {
        Error::new(
            ErrorKind::MissingTag { tag: "TIME-OFFSET" },
            "TIME-OFFSET is required in EXT-X-START",
        )
    })?;
    Ok(StartPoint {
        time_offset,
        precise: parse_bool_attribute(attrs, "PRECISE")?,
    })
}

fn parse_variant_stream(attrs: &str, uri: &str) -> Result<VariantStream> {
    let bandwidth = parse_u64_attribute(attrs, "BANDWIDTH")?.ok_or_else(|| {
        Error::new(
            ErrorKind::MissingTag { tag: "BANDWIDTH" },
            "BANDWIDTH is required in EXT-X-STREAM-INF",
        )
    })?;
    let resolution = get_attribute(attrs, "RESOLUTION")?
        .map(parse_resolution)
        .transpose()?;
    let closed_captions = match get_attribute(attrs, "CLOSED-CAPTIONS")? {
        Some("NONE") => Some(ClosedCaptions::None),
        Some(v) => Some(ClosedCaptions::GroupId(v.to_owned())),
        None => None,
    };
    let hdcp_level = match get_attribute(attrs, "HDCP-LEVEL")? {
        Some("NONE") => Some(HdcpLevel::None),
        Some("TYPE-0") => Some(HdcpLevel::Type0),
        Some("TYPE-1") => Some(HdcpLevel::Type1),
        Some(v) => {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "HDCP-LEVEL",
                },
                v,
            ));
        }
        None => None,
    };
    let video_range = match get_attribute(attrs, "VIDEO-RANGE")? {
        Some("SDR") => Some(VideoRange::Sdr),
        Some("HLG") => Some(VideoRange::Hlg),
        Some("PQ") => Some(VideoRange::Pq),
        Some(v) => {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "VIDEO-RANGE",
                },
                v,
            ));
        }
        None => None,
    };
    Ok(VariantStream {
        bandwidth,
        average_bandwidth: parse_u64_attribute(attrs, "AVERAGE-BANDWIDTH")?,
        codecs: get_attribute(attrs, "CODECS")?.map(str::to_owned),
        supplemental_codecs: get_attribute(attrs, "SUPPLEMENTAL-CODECS")?.map(str::to_owned),
        resolution,
        frame_rate: parse_f64_attribute(attrs, "FRAME-RATE")?,
        hdcp_level,
        allowed_cpc: get_attribute(attrs, "ALLOWED-CPC")?.map(str::to_owned),
        video_range,
        audio: get_attribute(attrs, "AUDIO")?.map(str::to_owned),
        video: get_attribute(attrs, "VIDEO")?.map(str::to_owned),
        subtitles: get_attribute(attrs, "SUBTITLES")?.map(str::to_owned),
        closed_captions,
        name: get_attribute(attrs, "NAME")?.map(str::to_owned),
        stable_variant_id: get_attribute(attrs, "STABLE-VARIANT-ID")?.map(str::to_owned),
        pathway_id: get_attribute(attrs, "PATHWAY-ID")?.map(str::to_owned),
        uri: uri.to_owned(),
    })
}

fn parse_rendition(attrs: &str) -> Result<Rendition> {
    let media_type = match require_attribute(attrs, "TYPE")? {
        "AUDIO" => MediaType::Audio,
        "VIDEO" => MediaType::Video,
        "SUBTITLES" => MediaType::Subtitles,
        "CLOSED-CAPTIONS" => MediaType::ClosedCaptions,
        v => {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue { attribute: "TYPE" },
                v,
            ));
        }
    };
    let uri = get_attribute(attrs, "URI")?.map(str::to_owned);
    let autoselect = parse_bool_attribute(attrs, "AUTOSELECT")?;
    let default = parse_bool_attribute(attrs, "DEFAULT")?;
    let forced = parse_bool_attribute(attrs, "FORCED")?;
    let instream_id = get_attribute(attrs, "INSTREAM-ID")?.map(str::to_owned);

    if default && get_attribute(attrs, "AUTOSELECT")? == Some("NO") {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "AUTOSELECT",
            },
            "AUTOSELECT=NO is invalid when DEFAULT=YES",
        ));
    }
    if media_type != MediaType::Subtitles && get_attribute(attrs, "FORCED")?.is_some() {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "FORCED",
            },
            "FORCED is only allowed for SUBTITLES renditions",
        ));
    }
    if media_type == MediaType::ClosedCaptions {
        if uri.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue { attribute: "URI" },
                "URI is not allowed for CLOSED-CAPTIONS renditions",
            ));
        }
        let Some(id) = instream_id.as_deref() else {
            return Err(Error::new(
                ErrorKind::MissingTag { tag: "INSTREAM-ID" },
                "INSTREAM-ID is required for CLOSED-CAPTIONS renditions",
            ));
        };
        validate_closed_captions_instream_id(id)?;
    }
    if media_type == MediaType::Subtitles && uri.is_none() {
        return Err(Error::new(
            ErrorKind::MissingTag { tag: "URI" },
            "URI is required for SUBTITLES renditions",
        ));
    }
    if media_type != MediaType::Audio && get_attribute(attrs, "BIT-DEPTH")?.is_some() {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "BIT-DEPTH",
            },
            "BIT-DEPTH is only allowed for AUDIO renditions",
        ));
    }
    if media_type != MediaType::Audio && get_attribute(attrs, "SAMPLE-RATE")?.is_some() {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "SAMPLE-RATE",
            },
            "SAMPLE-RATE is only allowed for AUDIO renditions",
        ));
    }
    if media_type != MediaType::Audio && get_attribute(attrs, "CHANNELS")?.is_some() {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "CHANNELS",
            },
            "CHANNELS is only allowed for AUDIO renditions",
        ));
    }

    Ok(Rendition {
        media_type,
        group_id: require_attribute(attrs, "GROUP-ID")?.to_owned(),
        name: require_attribute(attrs, "NAME")?.to_owned(),
        uri,
        language: get_attribute(attrs, "LANGUAGE")?.map(str::to_owned),
        assoc_language: get_attribute(attrs, "ASSOC-LANGUAGE")?.map(str::to_owned),
        default,
        autoselect,
        forced,
        instream_id,
        characteristics: get_attribute(attrs, "CHARACTERISTICS")?.map(str::to_owned),
        channels: get_attribute(attrs, "CHANNELS")?.map(str::to_owned),
        bit_depth: parse_u64_attribute(attrs, "BIT-DEPTH")?,
        sample_rate: parse_u64_attribute(attrs, "SAMPLE-RATE")?,
        stable_rendition_id: get_attribute(attrs, "STABLE-RENDITION-ID")?.map(str::to_owned),
    })
}

fn parse_i_frame_stream(attrs: &str) -> Result<IFrameStream> {
    let bandwidth = parse_u64_attribute(attrs, "BANDWIDTH")?.ok_or_else(|| {
        Error::new(
            ErrorKind::MissingTag { tag: "BANDWIDTH" },
            "BANDWIDTH is required in EXT-X-I-FRAME-STREAM-INF",
        )
    })?;
    let resolution = get_attribute(attrs, "RESOLUTION")?
        .map(parse_resolution)
        .transpose()?;
    let hdcp_level = match get_attribute(attrs, "HDCP-LEVEL")? {
        Some("NONE") => Some(HdcpLevel::None),
        Some("TYPE-0") => Some(HdcpLevel::Type0),
        Some("TYPE-1") => Some(HdcpLevel::Type1),
        Some(v) => {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "HDCP-LEVEL",
                },
                v,
            ));
        }
        None => None,
    };
    let video_range = match get_attribute(attrs, "VIDEO-RANGE")? {
        Some("SDR") => Some(VideoRange::Sdr),
        Some("HLG") => Some(VideoRange::Hlg),
        Some("PQ") => Some(VideoRange::Pq),
        Some(v) => {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "VIDEO-RANGE",
                },
                v,
            ));
        }
        None => None,
    };
    Ok(IFrameStream {
        bandwidth,
        average_bandwidth: parse_u64_attribute(attrs, "AVERAGE-BANDWIDTH")?,
        codecs: get_attribute(attrs, "CODECS")?.map(str::to_owned),
        supplemental_codecs: get_attribute(attrs, "SUPPLEMENTAL-CODECS")?.map(str::to_owned),
        resolution,
        hdcp_level,
        video_range,
        video: get_attribute(attrs, "VIDEO")?.map(str::to_owned),
        pathway_id: get_attribute(attrs, "PATHWAY-ID")?.map(str::to_owned),
        uri: require_attribute(attrs, "URI")?.to_owned(),
    })
}

fn parse_content_steering(attrs: &str) -> Result<ContentSteering> {
    Ok(ContentSteering {
        server_uri: require_attribute(attrs, "SERVER-URI")?.to_owned(),
        pathway_id: get_attribute(attrs, "PATHWAY-ID")?.map(str::to_owned),
    })
}

fn parse_session_data(attrs: &str) -> Result<SessionData> {
    let data_id = require_attribute(attrs, "DATA-ID")?.to_owned();
    let value = match (get_attribute(attrs, "VALUE")?, get_attribute(attrs, "URI")?) {
        (Some(v), None) => SessionDataValue::Value(v.to_owned()),
        (None, Some(u)) => SessionDataValue::Uri(u.to_owned()),
        (Some(_), Some(_)) => {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-SESSION-DATA",
                },
                "VALUE and URI are mutually exclusive",
            ));
        }
        (None, None) => {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-SESSION-DATA",
                },
                "either VALUE or URI is required",
            ));
        }
    };
    Ok(SessionData {
        data_id,
        value,
        language: get_attribute(attrs, "LANGUAGE")?.map(str::to_owned),
    })
}

pub(crate) fn parse_key(attrs: &str) -> Result<Key> {
    let method = match require_attribute(attrs, "METHOD")? {
        "NONE" => EncryptionMethod::None,
        "AES-128" => EncryptionMethod::Aes128,
        "AES-256-GCM" => EncryptionMethod::Aes256Gcm,
        "SAMPLE-AES" => EncryptionMethod::SampleAes,
        "SAMPLE-AES-CTR" => EncryptionMethod::SampleAesCtr,
        v => {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "METHOD",
                },
                v,
            ));
        }
    };
    Ok(Key {
        method,
        uri: get_attribute(attrs, "URI")?.map(str::to_owned),
        iv: get_attribute(attrs, "IV")?.map(str::to_owned),
        keyformat: get_attribute(attrs, "KEYFORMAT")?.map(str::to_owned),
        keyformat_versions: get_attribute(attrs, "KEYFORMATVERSIONS")?.map(str::to_owned),
    })
}

pub(crate) fn validate_key_attributes(key: &Key, tag: &'static str) -> Result<()> {
    match key.method {
        EncryptionMethod::None => {
            if key.uri.is_some()
                || key.iv.is_some()
                || key.keyformat.is_some()
                || key.keyformat_versions.is_some()
            {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue { tag },
                    format!("{tag} with METHOD=NONE must not contain other attributes"),
                ));
            }
        }
        EncryptionMethod::Aes128
        | EncryptionMethod::Aes256Gcm
        | EncryptionMethod::SampleAes
        | EncryptionMethod::SampleAesCtr => {
            if key.uri.is_none() {
                return Err(Error::new(
                    ErrorKind::MissingTag { tag: "URI" },
                    format!(
                        "{tag} with METHOD={} requires URI",
                        method_name(&key.method)
                    ),
                ));
            }
        }
    }

    if matches!(
        key.method,
        EncryptionMethod::Aes256Gcm | EncryptionMethod::SampleAesCtr
    ) && key.iv.is_some()
    {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue { attribute: "IV" },
            format!(
                "{} with METHOD={} must not contain IV",
                tag,
                method_name(&key.method)
            ),
        ));
    }

    Ok(())
}

fn method_name(method: &EncryptionMethod) -> &'static str {
    match method {
        EncryptionMethod::None => "NONE",
        EncryptionMethod::Aes128 => "AES-128",
        EncryptionMethod::Aes256Gcm => "AES-256-GCM",
        EncryptionMethod::SampleAes => "SAMPLE-AES",
        EncryptionMethod::SampleAesCtr => "SAMPLE-AES-CTR",
    }
}

pub(crate) fn parse_variable_definition(
    attrs: &str,
    kind: PlaylistKind,
    variables: &HashMap<String, String>,
    playlist_uri: Option<&str>,
) -> Result<VariableDefinition> {
    let name = get_attribute(attrs, "NAME")?.map(str::to_owned);
    let import = get_attribute(attrs, "IMPORT")?.map(str::to_owned);
    let queryparam = get_attribute(attrs, "QUERYPARAM")?.map(str::to_owned);

    let count = usize::from(name.is_some())
        + usize::from(import.is_some())
        + usize::from(queryparam.is_some());
    if count != 1 {
        return Err(Error::new(
            ErrorKind::InvalidTagValue {
                tag: "EXT-X-DEFINE",
            },
            "EXT-X-DEFINE must contain exactly one of NAME, IMPORT, or QUERYPARAM",
        ));
    }

    if let Some(name) = name {
        let value = require_attribute(attrs, "VALUE")?.to_owned();
        return Ok(VariableDefinition::Name { name, value });
    }

    if let Some(name) = import {
        if matches!(kind, PlaylistKind::Multivariant) {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-DEFINE",
                },
                "IMPORT is not allowed in Multivariant Playlists",
            ));
        }
        let value = variables.get(&name).cloned().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-DEFINE",
                },
                format!("imported variable is not defined: {name}"),
            )
        })?;
        return Ok(VariableDefinition::Import { name, value });
    }

    let name = queryparam.expect("queryparam must exist");
    let playlist_uri = playlist_uri.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidTagValue {
                tag: "EXT-X-DEFINE",
            },
            "playlist URI is required to resolve QUERYPARAM",
        )
    })?;
    let value = resolve_query_param(playlist_uri, &name).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidTagValue {
                tag: "EXT-X-DEFINE",
            },
            format!("query parameter is not defined: {name}"),
        )
    })?;
    Ok(VariableDefinition::QueryParam { name, value })
}

pub(crate) fn substitute_variables_in_line(
    line: &str,
    variables: &HashMap<String, String>,
) -> Result<String> {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find("{$") {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find('}') else {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-DEFINE",
                },
                "unterminated variable reference",
            ));
        };
        let name = &after_start[..end];
        let value = variables.get(name).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-DEFINE",
                },
                format!("undefined variable reference: {name}"),
            )
        })?;
        out.push_str(value);
        rest = &after_start[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn resolve_query_param(playlist_uri: &str, target: &str) -> Option<String> {
    let after_question = playlist_uri.split_once('?')?.1;
    // フラグメントを除去する
    let query = after_question
        .split_once('#')
        .map_or(after_question, |(q, _)| q);
    for pair in query.split('&') {
        // イコールなしのフラグパラメータはスキップする
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        if name == target {
            return percent_decode(value);
        }
    }
    None
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_value(bytes[i + 1])?;
                let lo = hex_value(bytes[i + 2])?;
                out.push((hi << 4) | lo);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// 入力が正確に `#EXTM3U` で始まるかを検証する
pub(crate) fn is_valid_extm3u_header(input: &str) -> bool {
    input == "#EXTM3U" || input.starts_with("#EXTM3U\n") || input.starts_with("#EXTM3U\r\n")
}

fn validate_closed_captions_instream_id(instream_id: &str) -> Result<()> {
    if matches!(instream_id, "CC1" | "CC2" | "CC3" | "CC4") {
        return Ok(());
    }
    if let Some(value) = instream_id.strip_prefix("SERVICE") {
        let service = value.parse::<u8>().map_err(|_| {
            Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "INSTREAM-ID",
                },
                instream_id,
            )
        })?;
        if (1..=63).contains(&service) {
            return Ok(());
        }
    }
    Err(Error::new(
        ErrorKind::InvalidAttributeValue {
            attribute: "INSTREAM-ID",
        },
        instream_id,
    ))
}

// この検証は draft-pantos-hls-rfc8216bis-20.txt 4.4.6.1 と
// rfc8216.txt 4.3.4.5 由来。最終 RFC で変更される可能性がある。
fn validate_multivariant_playlist(playlist: &MultivariantPlaylist) -> Result<()> {
    let mut group_names: HashMap<(MediaType, String), HashSet<String>> = HashMap::new();
    let mut group_default_count: HashMap<(MediaType, String), usize> = HashMap::new();
    let mut subtitles_groups = HashSet::new();
    let mut groups_by_type: HashMap<MediaType, HashSet<String>> = HashMap::new();

    for rendition in &playlist.renditions {
        let group_key = (rendition.media_type.clone(), rendition.group_id.clone());
        let names = group_names.entry(group_key.clone()).or_default();
        groups_by_type
            .entry(rendition.media_type.clone())
            .or_default()
            .insert(rendition.group_id.clone());
        if !names.insert(rendition.name.clone()) {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue { attribute: "NAME" },
                format!(
                    "duplicate rendition NAME in group {}:{}",
                    rendition.group_id, rendition.name
                ),
            ));
        }
        if rendition.default {
            let count = group_default_count.entry(group_key).or_default();
            *count += 1;
            if *count > 1 {
                return Err(Error::new(
                    ErrorKind::InvalidAttributeValue {
                        attribute: "DEFAULT",
                    },
                    format!(
                        "multiple DEFAULT=YES renditions in group {}",
                        rendition.group_id
                    ),
                ));
            }
        }
        if rendition.media_type == MediaType::Subtitles {
            subtitles_groups.insert(rendition.group_id.clone());
        }
    }

    let mut seen_session_keys = HashSet::new();
    for key in &playlist.session_keys {
        validate_key_attributes(key, "EXT-X-SESSION-KEY")?;
        if key.method == EncryptionMethod::None {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "METHOD",
                },
                "EXT-X-SESSION-KEY must not use METHOD=NONE",
            ));
        }
        let signature = (
            key.method.clone(),
            key.uri.clone(),
            key.iv.clone(),
            key.keyformat.clone(),
            key.keyformat_versions.clone(),
        );
        if !seen_session_keys.insert(signature) {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-SESSION-KEY",
                },
                "duplicate EXT-X-SESSION-KEY definition",
            ));
        }
    }

    let mut seen_session_data = HashSet::new();
    for session in &playlist.session_data {
        let signature = (session.data_id.clone(), session.language.clone());
        if !seen_session_data.insert(signature) {
            return Err(Error::new(
                ErrorKind::InvalidTagValue {
                    tag: "EXT-X-SESSION-DATA",
                },
                "duplicate EXT-X-SESSION-DATA definition",
            ));
        }
    }

    // CLOSED-CAPTIONS=NONE と CLOSED-CAPTIONS=<group-id> の混在を禁止する
    let has_cc_none = playlist
        .variant_streams
        .iter()
        .any(|s| matches!(&s.closed_captions, Some(ClosedCaptions::None)));
    let has_cc_group = playlist
        .variant_streams
        .iter()
        .any(|s| matches!(&s.closed_captions, Some(ClosedCaptions::GroupId(_))));
    if has_cc_none && has_cc_group {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "CLOSED-CAPTIONS",
            },
            "if any Variant Stream has CLOSED-CAPTIONS=NONE, all must have CLOSED-CAPTIONS=NONE",
        ));
    }

    for stream in &playlist.variant_streams {
        validate_group_reference(&groups_by_type, &stream.audio, MediaType::Audio, "AUDIO")?;
        validate_group_reference(&groups_by_type, &stream.video, MediaType::Video, "VIDEO")?;
        validate_group_reference(
            &groups_by_type,
            &stream.subtitles,
            MediaType::Subtitles,
            "SUBTITLES",
        )?;
        if let Some(ClosedCaptions::GroupId(group_id)) = &stream.closed_captions {
            validate_group_reference(
                &groups_by_type,
                &Some(group_id.clone()),
                MediaType::ClosedCaptions,
                "CLOSED-CAPTIONS",
            )?;
        }
        let Some(subtitles_group) = stream.subtitles.as_deref() else {
            continue;
        };
        if !subtitles_groups.contains(subtitles_group) {
            continue;
        }
        let Some(codecs) = stream.codecs.as_deref() else {
            continue;
        };
        if !codecs_contains_sample_format(codecs, "wvtt") {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "CODECS",
                },
                format!(
                    "CODECS must include wvtt when SUBTITLES group {} is referenced",
                    subtitles_group
                ),
            ));
        }
    }

    Ok(())
}

fn validate_group_reference(
    groups_by_type: &HashMap<MediaType, HashSet<String>>,
    group_id: &Option<String>,
    media_type: MediaType,
    attribute: &'static str,
) -> Result<()> {
    let Some(group_id) = group_id.as_deref() else {
        return Ok(());
    };
    if groups_by_type
        .get(&media_type)
        .is_some_and(|groups| groups.contains(group_id))
    {
        return Ok(());
    }
    Err(Error::new(
        ErrorKind::InvalidAttributeValue { attribute },
        format!("unknown rendition group reference: {group_id}"),
    ))
}

fn codecs_contains_sample_format(codecs: &str, sample_format: &str) -> bool {
    codecs
        .split(',')
        .map(str::trim)
        .any(|codec| codec.eq_ignore_ascii_case(sample_format))
}

fn validate_content_steering(playlist: &MultivariantPlaylist) -> Result<()> {
    let Some(content_steering) = &playlist.content_steering else {
        return Ok(());
    };
    let Some(pathway_id) = content_steering.pathway_id.as_deref() else {
        return Ok(());
    };

    let has_pathway = playlist
        .variant_streams
        .iter()
        .filter_map(|stream| stream.pathway_id.as_deref())
        .any(|id| id == pathway_id)
        || playlist
            .i_frame_streams
            .iter()
            .filter_map(|stream| stream.pathway_id.as_deref())
            .any(|id| id == pathway_id);

    if has_pathway {
        return Ok(());
    }

    Err(Error::new(
        ErrorKind::InvalidAttributeValue {
            attribute: "PATHWAY-ID",
        },
        "EXT-X-CONTENT-STEERING PATHWAY-ID must reference a PATHWAY-ID in the playlist",
    ))
}

fn validate_multivariant_version(playlist: &MultivariantPlaylist) -> Result<()> {
    let required_version = if playlist
        .renditions
        .iter()
        .filter_map(|rendition| rendition.instream_id.as_deref())
        .any(|instream_id| instream_id.starts_with("SERVICE"))
    {
        7
    } else {
        1
    };

    if required_version <= 1 {
        return Ok(());
    }

    let Some(version) = playlist.version else {
        return Err(Error::new(
            ErrorKind::MissingTag {
                tag: "EXT-X-VERSION",
            },
            format!("Multivariant Playlist requires EXT-X-VERSION:{required_version} or higher"),
        ));
    };

    if version < required_version {
        return Err(Error::new(
            ErrorKind::InvalidTagValue {
                tag: "EXT-X-VERSION",
            },
            format!(
                "Multivariant Playlist requires EXT-X-VERSION:{required_version} or higher, got {version}"
            ),
        ));
    }

    Ok(())
}
