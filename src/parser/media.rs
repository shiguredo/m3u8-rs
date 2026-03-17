use crate::{
    attribute::{
        get_attribute, parse_attribute_entries, parse_bool_attribute, parse_byterange,
        parse_f64_attribute, parse_u64_attribute, require_attribute,
    },
    error::{Error, ErrorKind, Result},
    media::{
        ByteRange, DateRange, DateRangeAttribute, Map, MediaPlaylist, Part, PartInf, PlaylistType,
        PreloadHint, PreloadHintType, RenditionReport, Segment, ServerControl, Skip,
    },
    multivariant::MultivariantPlaylist,
    parser::multivariant::{
        PlaylistKind, parse_key, parse_variable_definition, substitute_variables_in_line,
        validate_key_attributes,
    },
};
use std::collections::{HashMap, HashSet};

/// Media Playlist をパースする
pub fn parse(
    input: &str,
    playlist_uri: Option<&str>,
    multivariant_playlist: Option<&MultivariantPlaylist>,
) -> Result<MediaPlaylist> {
    if !crate::parser::multivariant::is_valid_extm3u_header(input) {
        return Err(Error::new(
            ErrorKind::MissingHeader,
            "playlist must start with #EXTM3U",
        ));
    }

    let mut playlist = MediaPlaylist {
        version: None,
        target_duration: 0,
        media_sequence: None,
        discontinuity_sequence: None,
        playlist_type: None,
        i_frames_only: false,
        independent_segments: false,
        start: None,
        server_control: None,
        part_inf: None,
        variable_definitions: Vec::new(),
        segments: Vec::new(),
        skip: None,
        preload_hints: Vec::new(),
        rendition_reports: Vec::new(),
        end_list: false,
    };
    let mut variables = multivariant_playlist
        .map(|playlist| {
            playlist
                .variable_definitions
                .iter()
                .map(|definition| (definition.name().to_owned(), definition.value().to_owned()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let mut local_variable_names = HashSet::new();
    let mut has_target_duration = false;
    let mut has_first_segment = false;

    // セグメントごとに蓄積する状態
    let mut pending_duration: Option<f64> = None;
    let mut pending_title: Option<String> = None;
    let mut pending_byte_range: Option<ByteRange> = None;
    let mut pending_discontinuity = false;
    let mut pending_key: Option<crate::multivariant::Key> = None;
    let mut pending_map: Option<Map> = None;
    let mut pending_pdt: Option<String> = None;
    let mut pending_date_ranges: Vec<DateRange> = Vec::new();
    let mut pending_gap = false;
    let mut pending_bitrate: Option<u64> = None;
    let mut pending_parts: Vec<Part> = Vec::new();

    let lines = input.lines();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line == "#EXTM3U" {
            continue;
        }

        if let Some(attrs) = line.strip_prefix("#EXT-X-DEFINE:") {
            let definition =
                parse_variable_definition(attrs, PlaylistKind::Media, &variables, playlist_uri)?;
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
        reject_multivariant_only_tag(&line)?;
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
        } else if let Some(value) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            if has_target_duration {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-TARGETDURATION",
                    },
                    "EXT-X-TARGETDURATION must not appear more than once",
                ));
            }
            let parsed = value.trim().parse::<u32>().map_err(|_| {
                Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-TARGETDURATION",
                    },
                    value,
                )
            })?;
            if parsed == 0 {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-TARGETDURATION",
                    },
                    "EXT-X-TARGETDURATION value must be at least 1",
                ));
            }
            playlist.target_duration = parsed;
            has_target_duration = true;
        } else if let Some(value) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            if playlist.media_sequence.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-MEDIA-SEQUENCE",
                    },
                    "EXT-X-MEDIA-SEQUENCE must not appear more than once",
                ));
            }
            if has_first_segment {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-MEDIA-SEQUENCE",
                    },
                    "EXT-X-MEDIA-SEQUENCE must appear before the first Media Segment",
                ));
            }
            playlist.media_sequence = Some(value.trim().parse::<u64>().map_err(|_| {
                Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-MEDIA-SEQUENCE",
                    },
                    value,
                )
            })?);
        } else if let Some(value) = line.strip_prefix("#EXT-X-DISCONTINUITY-SEQUENCE:") {
            if playlist.discontinuity_sequence.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-DISCONTINUITY-SEQUENCE",
                    },
                    "EXT-X-DISCONTINUITY-SEQUENCE must not appear more than once",
                ));
            }
            if has_first_segment {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-DISCONTINUITY-SEQUENCE",
                    },
                    "EXT-X-DISCONTINUITY-SEQUENCE must appear before the first Media Segment",
                ));
            }
            playlist.discontinuity_sequence = Some(value.trim().parse::<u64>().map_err(|_| {
                Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-DISCONTINUITY-SEQUENCE",
                    },
                    value,
                )
            })?);
        } else if let Some(value) = line.strip_prefix("#EXT-X-PLAYLIST-TYPE:") {
            if playlist.playlist_type.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-PLAYLIST-TYPE",
                    },
                    "EXT-X-PLAYLIST-TYPE must not appear more than once",
                ));
            }
            playlist.playlist_type = match value.trim() {
                "EVENT" => Some(PlaylistType::Event),
                "VOD" => Some(PlaylistType::Vod),
                v => {
                    return Err(Error::new(
                        ErrorKind::InvalidTagValue {
                            tag: "EXT-X-PLAYLIST-TYPE",
                        },
                        v,
                    ));
                }
            };
        } else if line == "#EXT-X-I-FRAMES-ONLY" {
            if playlist.i_frames_only {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-I-FRAMES-ONLY",
                    },
                    "EXT-X-I-FRAMES-ONLY must not appear more than once",
                ));
            }
            playlist.i_frames_only = true;
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
            playlist.start = Some(crate::parser::multivariant::parse_start_inner(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-SERVER-CONTROL:") {
            if playlist.server_control.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-SERVER-CONTROL",
                    },
                    "EXT-X-SERVER-CONTROL must not appear more than once",
                ));
            }
            playlist.server_control = Some(parse_server_control(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-PART-INF:") {
            if playlist.part_inf.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-PART-INF",
                    },
                    "EXT-X-PART-INF must not appear more than once",
                ));
            }
            playlist.part_inf = Some(parse_part_inf(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-SKIP:") {
            if playlist.skip.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue { tag: "EXT-X-SKIP" },
                    "EXT-X-SKIP must not appear more than once",
                ));
            }
            playlist.skip = Some(parse_skip(attrs)?);
        } else if line == "#EXT-X-ENDLIST" {
            if playlist.end_list {
                return Err(Error::new(
                    ErrorKind::InvalidTagValue {
                        tag: "EXT-X-ENDLIST",
                    },
                    "EXT-X-ENDLIST must not appear more than once",
                ));
            }
            playlist.end_list = true;
        } else if line == "#EXT-X-DISCONTINUITY" {
            pending_discontinuity = true;
        } else if line == "#EXT-X-GAP" {
            pending_gap = true;
        } else if let Some(value) = line.strip_prefix("#EXT-X-BITRATE:") {
            // 0 は reset を意味する (builder が None → 0 で書き出すため)
            pending_bitrate = value.trim().parse::<u64>().ok().filter(|&v| v > 0);
        } else if let Some(attrs) = line.strip_prefix("#EXTINF:") {
            let (duration, title) = parse_extinf(attrs)?;
            pending_duration = Some(duration);
            pending_title = title;
        } else if let Some(value) = line.strip_prefix("#EXT-X-BYTERANGE:") {
            pending_byte_range = Some(parse_byterange(value.trim())?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-KEY:") {
            let key = parse_key(attrs)?;
            validate_key_attributes(&key, "EXT-X-KEY")?;
            // METHOD=NONE は暗号化なしへの reset を意味するため None に正規化する
            if key.method == crate::multivariant::EncryptionMethod::None {
                pending_key = None;
            } else {
                pending_key = Some(key);
            }
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-MAP:") {
            pending_map = Some(parse_map(attrs)?);
        } else if let Some(value) = line.strip_prefix("#EXT-X-PROGRAM-DATE-TIME:") {
            pending_pdt = Some(value.trim().to_owned());
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-DATERANGE:") {
            pending_date_ranges.push(parse_date_range(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-PART:") {
            pending_parts.push(parse_part(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-PRELOAD-HINT:") {
            playlist.preload_hints.push(parse_preload_hint(attrs)?);
        } else if let Some(attrs) = line.strip_prefix("#EXT-X-RENDITION-REPORT:") {
            playlist
                .rendition_reports
                .push(parse_rendition_report(attrs)?);
        } else if !line.starts_with('#') && !line.is_empty() {
            // URI 行: セグメントを確定させる
            let duration = pending_duration.ok_or_else(|| {
                Error::new(
                    ErrorKind::MissingTag { tag: "EXTINF" },
                    "segment URI found without preceding #EXTINF",
                )
            })?;
            playlist.segments.push(Segment {
                duration,
                title: pending_title.take(),
                uri: line.to_owned(),
                byte_range: pending_byte_range.take(),
                discontinuity: std::mem::replace(&mut pending_discontinuity, false),
                key: pending_key.clone(),
                map: pending_map.clone(),
                program_date_time: pending_pdt.take(),
                date_ranges: std::mem::take(&mut pending_date_ranges),
                gap: std::mem::replace(&mut pending_gap, false),
                bitrate: pending_bitrate,
                parts: std::mem::take(&mut pending_parts),
            });
            has_first_segment = true;
            pending_duration = None;
        }
        // 未知のタグは無視する
    }

    let has_pending_segment_data = pending_duration.is_some()
        || pending_title.is_some()
        || pending_byte_range.is_some()
        || pending_discontinuity
        || pending_pdt.is_some()
        || !pending_date_ranges.is_empty()
        || pending_gap
        || !pending_parts.is_empty();

    if has_pending_segment_data {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "playlist ended before pending segment data was completed",
        ));
    }

    if !has_target_duration {
        return Err(Error::new(
            ErrorKind::MissingTag {
                tag: "EXT-X-TARGETDURATION",
            },
            "EXT-X-TARGETDURATION is required",
        ));
    }

    validate_segment_duration(&playlist)?;
    validate_ll_hls_constraints(&playlist)?;
    validate_byterange_state(&playlist)?;
    validate_media_key_constraints(&playlist)?;
    validate_media_version(&playlist)?;
    validate_fmp4_and_i_frames_only_rules(&playlist)?;
    validate_date_range_constraints(&playlist)?;

    Ok(playlist)
}

fn parse_extinf(attrs: &str) -> Result<(f64, Option<String>)> {
    // `duration,title` または `duration`
    let (duration_str, title) = match attrs.split_once(',') {
        Some((d, t)) => (
            d,
            if t.is_empty() {
                None
            } else {
                Some(t.to_owned())
            },
        ),
        None => (attrs, None),
    };
    let duration = duration_str
        .trim()
        .parse::<f64>()
        .map_err(|_| Error::new(ErrorKind::InvalidTagValue { tag: "EXTINF" }, duration_str))?;
    Ok((duration, title))
}

fn parse_map(attrs: &str) -> Result<Map> {
    let uri = require_attribute(attrs, "URI")?.to_owned();
    let byte_range = get_attribute(attrs, "BYTERANGE")?
        .map(parse_byterange)
        .transpose()?;
    Ok(Map { uri, byte_range })
}

fn parse_date_range(attrs: &str) -> Result<DateRange> {
    let extra_attributes = parse_attribute_entries(attrs)?
        .into_iter()
        .filter(|entry| {
            !matches!(
                entry.name,
                "ID" | "CLASS"
                    | "START-DATE"
                    | "END-DATE"
                    | "DURATION"
                    | "PLANNED-DURATION"
                    | "END-ON-NEXT"
            )
        })
        .map(|entry| DateRangeAttribute {
            name: entry.name.to_owned(),
            value: entry.value.to_owned(),
            quoted: entry.quoted,
        })
        .collect();
    Ok(DateRange {
        id: require_attribute(attrs, "ID")?.to_owned(),
        class: get_attribute(attrs, "CLASS")?.map(str::to_owned),
        start_date: require_attribute(attrs, "START-DATE")?.to_owned(),
        end_date: get_attribute(attrs, "END-DATE")?.map(str::to_owned),
        duration: parse_f64_attribute(attrs, "DURATION")?,
        planned_duration: parse_f64_attribute(attrs, "PLANNED-DURATION")?,
        end_on_next: parse_bool_attribute(attrs, "END-ON-NEXT")?,
        extra_attributes,
    })
}

fn parse_server_control(attrs: &str) -> Result<ServerControl> {
    Ok(ServerControl {
        can_skip_until: parse_f64_attribute(attrs, "CAN-SKIP-UNTIL")?,
        can_skip_dateranges: parse_bool_attribute(attrs, "CAN-SKIP-DATERANGES")?,
        hold_back: parse_f64_attribute(attrs, "HOLD-BACK")?,
        part_hold_back: parse_f64_attribute(attrs, "PART-HOLD-BACK")?,
        can_block_reload: parse_bool_attribute(attrs, "CAN-BLOCK-RELOAD")?,
    })
}

fn parse_part_inf(attrs: &str) -> Result<PartInf> {
    let part_target = parse_f64_attribute(attrs, "PART-TARGET")?.ok_or_else(|| {
        Error::new(
            ErrorKind::MissingTag { tag: "PART-TARGET" },
            "PART-TARGET is required in EXT-X-PART-INF",
        )
    })?;
    Ok(PartInf { part_target })
}

fn parse_part(attrs: &str) -> Result<Part> {
    let uri = require_attribute(attrs, "URI")?.to_owned();
    let duration = parse_f64_attribute(attrs, "DURATION")?.ok_or_else(|| {
        Error::new(
            ErrorKind::MissingTag { tag: "DURATION" },
            "DURATION is required in EXT-X-PART",
        )
    })?;
    let byte_range = get_attribute(attrs, "BYTERANGE")?
        .map(parse_byterange)
        .transpose()?;
    Ok(Part {
        uri,
        duration,
        independent: parse_bool_attribute(attrs, "INDEPENDENT")?,
        byte_range,
        gap: parse_bool_attribute(attrs, "GAP")?,
    })
}

fn parse_preload_hint(attrs: &str) -> Result<PreloadHint> {
    let hint_type = match require_attribute(attrs, "TYPE")? {
        "PART" => PreloadHintType::Part,
        "MAP" => PreloadHintType::Map,
        v => {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue { attribute: "TYPE" },
                v,
            ));
        }
    };
    Ok(PreloadHint {
        hint_type,
        uri: require_attribute(attrs, "URI")?.to_owned(),
        byterange_start: parse_u64_attribute(attrs, "BYTERANGE-START")?,
        byterange_length: parse_u64_attribute(attrs, "BYTERANGE-LENGTH")?,
    })
}

fn parse_rendition_report(attrs: &str) -> Result<RenditionReport> {
    let uri = require_attribute(attrs, "URI")?;
    if looks_like_absolute_uri(uri) {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue { attribute: "URI" },
            "EXT-X-RENDITION-REPORT URI must be relative",
        ));
    }
    let last_msn = parse_u64_attribute(attrs, "LAST-MSN")?.ok_or_else(|| {
        Error::new(
            ErrorKind::MissingTag { tag: "LAST-MSN" },
            "LAST-MSN is required in EXT-X-RENDITION-REPORT",
        )
    })?;
    Ok(RenditionReport {
        uri: uri.to_owned(),
        last_msn: Some(last_msn),
        last_part: parse_u64_attribute(attrs, "LAST-PART")?,
    })
}

// この機能は draft-pantos-hls-rfc8216bis-20.txt 4.4.5.2 由来。
// 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
fn parse_skip(attrs: &str) -> Result<Skip> {
    let skipped_segments = parse_u64_attribute(attrs, "SKIPPED-SEGMENTS")?.ok_or_else(|| {
        Error::new(
            ErrorKind::MissingTag {
                tag: "SKIPPED-SEGMENTS",
            },
            "SKIPPED-SEGMENTS is required in EXT-X-SKIP",
        )
    })?;
    let recently_removed_dateranges = get_attribute(attrs, "RECENTLY-REMOVED-DATERANGES")?
        .map(|value| {
            if value.is_empty() {
                Vec::new()
            } else {
                value.split('\t').map(str::to_owned).collect()
            }
        })
        .unwrap_or_default();
    Ok(Skip {
        skipped_segments,
        recently_removed_dateranges,
    })
}

// この検証は draft-pantos-hls-rfc8216bis-20.txt 4.4.3.7, 4.4.3.8,
// 4.4.5.3 由来。最終 RFC で変更される可能性がある。
fn validate_ll_hls_constraints(playlist: &MediaPlaylist) -> Result<()> {
    let has_parts = playlist
        .segments
        .iter()
        .any(|segment| !segment.parts.is_empty());
    if has_parts && playlist.part_inf.is_none() {
        return Err(Error::new(
            ErrorKind::MissingTag {
                tag: "EXT-X-PART-INF",
            },
            "EXT-X-PART-INF is required when EXT-X-PART tags are present",
        ));
    }

    if playlist.part_inf.is_some()
        && playlist
            .server_control
            .as_ref()
            .and_then(|control| control.part_hold_back)
            .is_none()
    {
        return Err(Error::new(
            ErrorKind::MissingTag {
                tag: "PART-HOLD-BACK",
            },
            "PART-HOLD-BACK is required when EXT-X-PART-INF is present",
        ));
    }

    if playlist
        .server_control
        .as_ref()
        .is_some_and(|control| control.can_skip_dateranges && control.can_skip_until.is_none())
    {
        return Err(Error::new(
            ErrorKind::MissingTag {
                tag: "CAN-SKIP-UNTIL",
            },
            "CAN-SKIP-UNTIL is required when CAN-SKIP-DATERANGES is YES",
        ));
    }

    // HOLD-BACK は TARGETDURATION の 3 倍以上でなければならない
    if let Some(hold_back) = playlist
        .server_control
        .as_ref()
        .and_then(|control| control.hold_back)
    {
        let min_hold_back = f64::from(playlist.target_duration) * 3.0;
        if hold_back < min_hold_back {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "HOLD-BACK",
                },
                format!(
                    "HOLD-BACK must be at least three times EXT-X-TARGETDURATION ({min_hold_back:.5}), got {hold_back:.5}"
                ),
            ));
        }
    }

    // PART-HOLD-BACK は PART-TARGET 以上でなければならない
    if let (Some(part_hold_back), Some(part_inf)) = (
        playlist
            .server_control
            .as_ref()
            .and_then(|control| control.part_hold_back),
        &playlist.part_inf,
    ) && part_hold_back < part_inf.part_target
    {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "PART-HOLD-BACK",
            },
            format!(
                "PART-HOLD-BACK must be at least PART-TARGET ({:.5}), got {part_hold_back:.5}",
                part_inf.part_target
            ),
        ));
    }

    // プレイリストに EXT-X-PART がある場合、RENDITION-REPORT には LAST-PART が必須
    if has_parts {
        for report in &playlist.rendition_reports {
            if report.last_part.is_none() {
                return Err(Error::new(
                    ErrorKind::MissingTag { tag: "LAST-PART" },
                    "LAST-PART is required in EXT-X-RENDITION-REPORT when EXT-X-PART tags are present",
                ));
            }
        }
    }

    if playlist.end_list && !playlist.preload_hints.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidTagValue {
                tag: "EXT-X-PRELOAD-HINT",
            },
            "EXT-X-PRELOAD-HINT must not appear in playlists with EXT-X-ENDLIST",
        ));
    }

    Ok(())
}

fn validate_segment_duration(playlist: &MediaPlaylist) -> Result<()> {
    let target = playlist.target_duration;
    for segment in &playlist.segments {
        // 最も近い整数に丸めた値が TARGETDURATION 以下でなければならない
        let rounded = segment.duration.round() as u64;
        if rounded > u64::from(target) {
            return Err(Error::new(
                ErrorKind::InvalidTagValue { tag: "EXTINF" },
                format!(
                    "EXTINF duration {:.5} (rounded to {rounded}) exceeds EXT-X-TARGETDURATION:{target}",
                    segment.duration
                ),
            ));
        }
    }
    Ok(())
}

fn validate_byterange_state(playlist: &MediaPlaylist) -> Result<()> {
    let mut previous_ranged_segment: Option<(&str, ByteRange)> = None;

    for segment in &playlist.segments {
        if let Some(byte_range) = segment.byte_range {
            if byte_range.offset.is_none() {
                let Some((previous_uri, _)) = previous_ranged_segment else {
                    return Err(Error::new(
                        ErrorKind::InvalidTagValue {
                            tag: "EXT-X-BYTERANGE",
                        },
                        "BYTERANGE without offset requires a previous ranged segment",
                    ));
                };
                if previous_uri != segment.uri {
                    return Err(Error::new(
                        ErrorKind::InvalidTagValue {
                            tag: "EXT-X-BYTERANGE",
                        },
                        "BYTERANGE without offset requires the same segment URI as the previous ranged segment",
                    ));
                }
            }
            previous_ranged_segment = Some((segment.uri.as_str(), byte_range));
        } else {
            previous_ranged_segment = None;
        }
    }

    Ok(())
}

fn validate_media_version(playlist: &MediaPlaylist) -> Result<()> {
    let mut required_version = 1u8;

    if playlist.i_frames_only {
        required_version = required_version.max(4);
    }

    for segment in &playlist.segments {
        if segment.duration.fract() != 0.0 {
            required_version = required_version.max(3);
        }
        if segment.byte_range.is_some() {
            required_version = required_version.max(4);
        }
        if let Some(key) = &segment.key {
            if key.iv.is_some() {
                required_version = required_version.max(2);
            }
            if key.keyformat.is_some() || key.keyformat_versions.is_some() {
                required_version = required_version.max(5);
            }
        }
        if segment.map.is_some() {
            required_version = required_version.max(if playlist.i_frames_only { 5 } else { 6 });
        }
    }

    validate_required_version(
        playlist.version,
        required_version,
        "Media Playlist",
        "EXT-X-VERSION",
    )
}

fn validate_required_version(
    version: Option<u8>,
    required_version: u8,
    playlist_kind: &str,
    tag: &'static str,
) -> Result<()> {
    if required_version <= 1 {
        return Ok(());
    }

    let Some(version) = version else {
        return Err(Error::new(
            ErrorKind::MissingTag { tag },
            format!("{playlist_kind} requires EXT-X-VERSION:{required_version} or higher"),
        ));
    };

    if version < required_version {
        return Err(Error::new(
            ErrorKind::InvalidTagValue { tag },
            format!(
                "{playlist_kind} requires EXT-X-VERSION:{required_version} or higher, got {version}"
            ),
        ));
    }

    Ok(())
}

fn validate_media_key_constraints(playlist: &MediaPlaylist) -> Result<()> {
    for segment in &playlist.segments {
        if let Some(key) = &segment.key {
            validate_key_attributes(key, "EXT-X-KEY")?;
        }
    }

    Ok(())
}

fn validate_fmp4_and_i_frames_only_rules(playlist: &MediaPlaylist) -> Result<()> {
    for segment in &playlist.segments {
        let has_fmp4_media = is_fmp4_like_uri(&segment.uri)
            || segment.parts.iter().any(|part| is_fmp4_like_uri(&part.uri));
        if has_fmp4_media && segment.map.is_none() {
            return Err(Error::new(
                ErrorKind::MissingTag { tag: "EXT-X-MAP" },
                "fMP4 segments require EXT-X-MAP",
            ));
        }
    }

    Ok(())
}

fn is_fmp4_like_uri(uri: &str) -> bool {
    // クエリパラメータとフラグメントを除去してパス部分のみを取得する
    let without_query = uri.split_once('?').map_or(uri, |(path, _)| path);
    let path = without_query
        .split_once('#')
        .map_or(without_query, |(path, _)| path)
        .to_ascii_lowercase();
    [".m4s", ".mp4", ".cmfa", ".cmfv", ".cmft", ".cmfm"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
}

fn looks_like_absolute_uri(uri: &str) -> bool {
    let Some((scheme, _)) = uri.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        && scheme.as_bytes()[0].is_ascii_alphabetic()
}

fn reject_multivariant_only_tag(line: &str) -> Result<()> {
    let tag = if line.starts_with("#EXT-X-MEDIA:") {
        Some("EXT-X-MEDIA")
    } else if line.starts_with("#EXT-X-STREAM-INF:") {
        Some("EXT-X-STREAM-INF")
    } else if line.starts_with("#EXT-X-I-FRAME-STREAM-INF:") {
        Some("EXT-X-I-FRAME-STREAM-INF")
    } else if line.starts_with("#EXT-X-SESSION-DATA:") {
        Some("EXT-X-SESSION-DATA")
    } else if line.starts_with("#EXT-X-SESSION-KEY:") {
        Some("EXT-X-SESSION-KEY")
    } else if line.starts_with("#EXT-X-CONTENT-STEERING:") {
        Some("EXT-X-CONTENT-STEERING")
    } else {
        None
    };

    if let Some(tag) = tag {
        return Err(Error::new(
            ErrorKind::InvalidTagValue { tag },
            format!("{tag} must not appear in a Media Playlist"),
        ));
    }

    Ok(())
}

fn validate_date_range_constraints(playlist: &MediaPlaylist) -> Result<()> {
    let has_date_range = playlist
        .segments
        .iter()
        .any(|segment| !segment.date_ranges.is_empty());
    if !has_date_range {
        return Ok(());
    }

    let has_program_date_time = playlist
        .segments
        .iter()
        .any(|segment| segment.program_date_time.is_some());
    if !has_program_date_time {
        return Err(Error::new(
            ErrorKind::MissingTag {
                tag: "EXT-X-PROGRAM-DATE-TIME",
            },
            "EXT-X-DATERANGE requires at least one EXT-X-PROGRAM-DATE-TIME tag",
        ));
    }

    let mut seen_ids: HashMap<&str, &DateRange> = HashMap::new();
    for segment in &playlist.segments {
        for date_range in &segment.date_ranges {
            validate_single_date_range(date_range)?;
            if let Some(previous) = seen_ids.insert(date_range.id.as_str(), date_range) {
                validate_matching_date_range(previous, date_range)?;
            }
        }
    }

    Ok(())
}

fn validate_single_date_range(date_range: &DateRange) -> Result<()> {
    if date_range.duration.is_some_and(|duration| duration < 0.0) {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "DURATION",
            },
            "DURATION must not be negative",
        ));
    }
    if date_range
        .planned_duration
        .is_some_and(|duration| duration < 0.0)
    {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "PLANNED-DURATION",
            },
            "PLANNED-DURATION must not be negative",
        ));
    }
    if date_range.end_on_next {
        if date_range.class.is_none() {
            return Err(Error::new(
                ErrorKind::MissingTag { tag: "CLASS" },
                "END-ON-NEXT requires CLASS",
            ));
        }
        if date_range.duration.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "DURATION",
                },
                "END-ON-NEXT forbids DURATION",
            ));
        }
        if date_range.end_date.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "END-DATE",
                },
                "END-ON-NEXT forbids END-DATE",
            ));
        }
    }

    let start = parse_iso8601_datetime(&date_range.start_date)?;
    if let Some(end_date) = &date_range.end_date {
        let end = parse_iso8601_datetime(end_date)?;
        if end < start {
            return Err(Error::new(
                ErrorKind::InvalidAttributeValue {
                    attribute: "END-DATE",
                },
                "END-DATE must be equal to or later than START-DATE",
            ));
        }
        if let Some(duration) = date_range.duration {
            let expected = add_duration_seconds(start, duration)?;
            if end != expected {
                return Err(Error::new(
                    ErrorKind::InvalidAttributeValue {
                        attribute: "END-DATE",
                    },
                    "END-DATE must equal START-DATE plus DURATION",
                ));
            }
        }
    }

    Ok(())
}

fn validate_matching_date_range(previous: &DateRange, current: &DateRange) -> Result<()> {
    if previous.start_date != current.start_date {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "START-DATE",
            },
            "DATERANGEs with the same ID must keep the same START-DATE",
        ));
    }
    if previous.class.is_some() && current.class.is_some() && previous.class != current.class {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue { attribute: "CLASS" },
            "DATERANGEs with the same ID must keep the same CLASS",
        ));
    }
    if previous.end_date.is_some()
        && current.end_date.is_some()
        && previous.end_date != current.end_date
    {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "END-DATE",
            },
            "DATERANGEs with the same ID must keep the same END-DATE",
        ));
    }
    if previous.duration.is_some()
        && current.duration.is_some()
        && previous.duration != current.duration
    {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "DURATION",
            },
            "DATERANGEs with the same ID must keep the same DURATION",
        ));
    }
    if previous.planned_duration.is_some()
        && current.planned_duration.is_some()
        && previous.planned_duration != current.planned_duration
    {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "PLANNED-DURATION",
            },
            "DATERANGEs with the same ID must keep the same PLANNED-DURATION",
        ));
    }

    Ok(())
}

fn add_duration_seconds(start: i128, duration: f64) -> Result<i128> {
    let nanos = duration * 1_000_000_000.0;
    if !nanos.is_finite() {
        return Err(Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "DURATION",
            },
            "invalid DURATION",
        ));
    }
    Ok(start + nanos.round() as i128)
}

fn parse_iso8601_datetime(value: &str) -> Result<i128> {
    let date_time = value.split_once('T').ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidAttributeValue {
                attribute: "START-DATE",
            },
            "invalid ISO-8601 date-time",
        )
    })?;
    let (date, time_and_offset) = date_time;
    let mut date_parts = date.split('-');
    let year = parse_i32_component(date_parts.next(), "START-DATE")?;
    let month = parse_u32_component(date_parts.next(), "START-DATE")?;
    let day = parse_u32_component(date_parts.next(), "START-DATE")?;

    let timezone_start = time_and_offset
        .find(['Z', '+', '-'])
        .ok_or_else(invalid_datetime_error)?;
    let time = &time_and_offset[..timezone_start];
    let timezone = &time_and_offset[timezone_start..];
    let mut time_parts = time.split(':');
    let hour = parse_u32_component(time_parts.next(), "START-DATE")?;
    let minute = parse_u32_component(time_parts.next(), "START-DATE")?;
    let second_part = time_parts.next().ok_or_else(invalid_datetime_error)?;
    let (second_str, fraction_str) = match second_part.split_once('.') {
        Some((second, fraction)) => (second, Some(fraction)),
        None => (second_part, None),
    };
    let second = second_str
        .parse::<u32>()
        .map_err(|_| invalid_datetime_error())?;
    let fraction = fraction_str.map_or(0u32, |fraction| {
        let mut digits = fraction.chars().take(9).collect::<String>();
        while digits.len() < 9 {
            digits.push('0');
        }
        digits.parse::<u32>().unwrap_or(0)
    });

    let offset_seconds = match timezone {
        "Z" => 0i32,
        _ => {
            let sign = if timezone.starts_with('+') {
                1
            } else if timezone.starts_with('-') {
                -1
            } else {
                return Err(invalid_datetime_error());
            };
            let offset = &timezone[1..];
            let (hours, minutes) = offset.split_once(':').ok_or_else(invalid_datetime_error)?;
            let hours = hours.parse::<i32>().map_err(|_| invalid_datetime_error())?;
            let minutes = minutes
                .parse::<i32>()
                .map_err(|_| invalid_datetime_error())?;
            sign * (hours * 3600 + minutes * 60)
        }
    };

    let days = days_from_civil(year, month, day);
    let seconds =
        days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second)
            - i64::from(offset_seconds);

    Ok(i128::from(seconds) * 1_000_000_000 + i128::from(fraction))
}

fn invalid_datetime_error() -> Error {
    Error::new(
        ErrorKind::InvalidAttributeValue {
            attribute: "START-DATE",
        },
        "invalid ISO-8601 date-time",
    )
}

fn parse_i32_component(value: Option<&str>, attribute: &'static str) -> Result<i32> {
    value
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidAttributeValue { attribute },
                "missing component",
            )
        })?
        .parse::<i32>()
        .map_err(|_| {
            Error::new(
                ErrorKind::InvalidAttributeValue { attribute },
                "invalid component",
            )
        })
}

fn parse_u32_component(value: Option<&str>, attribute: &'static str) -> Result<u32> {
    value
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidAttributeValue { attribute },
                "missing component",
            )
        })?
        .parse::<u32>()
        .map_err(|_| {
            Error::new(
                ErrorKind::InvalidAttributeValue { attribute },
                "invalid component",
            )
        })
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era * 146097 + doe - 719468)
}
