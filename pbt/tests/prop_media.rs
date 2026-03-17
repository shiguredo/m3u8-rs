use proptest::prelude::*;
use shiguredo_m3u8::{
    media::{
        ByteRange, DateRange, Map, MediaPlaylist, Part, PartInf, PlaylistType, PreloadHint,
        PreloadHintType, RenditionReport, Segment, ServerControl, Skip,
    },
    multivariant::{EncryptionMethod, Key, StartPoint},
    parse_media_playlist, write_media_playlist,
};
use std::collections::HashMap;

fn safe_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_./-]{1,32}".prop_map(|s| s)
}

fn uri_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_./-]{1,64}".prop_map(|s| s)
}

fn byterange_strategy() -> impl Strategy<Value = ByteRange> {
    (
        1u64..=10_000_000u64,
        proptest::option::of(0u64..=10_000_000u64),
    )
        .prop_map(|(length, offset)| ByteRange { length, offset })
}

prop_compose! {
    fn map_strategy()(
        uri in uri_strategy(),
        byte_range in proptest::option::of(byterange_strategy()),
    ) -> Map {
        Map { uri, byte_range }
    }
}

prop_compose! {
    fn key_strategy()(
        // EncryptionMethod::None は parser が key: None に正規化するため除外する
        method in prop_oneof![
            Just(EncryptionMethod::Aes128),
            Just(EncryptionMethod::Aes256Gcm),
            Just(EncryptionMethod::SampleAes),
            Just(EncryptionMethod::SampleAesCtr),
        ],
        uri in proptest::option::of(uri_strategy()),
        iv in proptest::option::of("[0-9a-fA-F]{32}".prop_map(|s| s)),
        keyformat in proptest::option::of(safe_string()),
        keyformat_versions in proptest::option::of("[0-9/]{1,8}".prop_map(|s| s)),
    ) -> Key {
        let uri = Some(uri.unwrap_or_else(|| String::from("key.bin")));
        let iv = match method {
            EncryptionMethod::Aes256Gcm | EncryptionMethod::SampleAesCtr => None,
            _ => iv,
        };
        Key { method, uri, iv, keyformat, keyformat_versions }
    }
}

prop_compose! {
    fn start_point_strategy()(
        time_offset in -3600.0f64..3600.0f64,
        precise in proptest::bool::ANY,
    ) -> StartPoint {
        let time_offset = (time_offset * 1e5).round() / 1e5;
        StartPoint { time_offset, precise }
    }
}

prop_compose! {
    fn date_range_strategy()(
        id in safe_string(),
        class in proptest::option::of(safe_string()),
        start_date in "[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|1[0-9]|2[0-8])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z".prop_map(|s| s),
        end_date in proptest::option::of(
            "[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|1[0-9]|2[0-8])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z".prop_map(|s| s),
        ),
        duration in proptest::option::of((0.0f64..=86400.0f64).prop_map(|v| (v * 1e5).round() / 1e5)),
        planned_duration in proptest::option::of((0.0f64..=86400.0f64).prop_map(|v| (v * 1e5).round() / 1e5)),
        end_on_next in proptest::bool::ANY,
    ) -> DateRange {
        let class = if end_on_next {
            Some(class.unwrap_or_else(|| String::from("class")))
        } else {
            class
        };
        let (end_date, duration) = if end_on_next {
            (None, None)
        } else if let (Some(end_date), Some(_duration)) = (end_date.clone(), duration) {
            if end_date < start_date {
                (Some(start_date.clone()), None)
            } else {
                (Some(end_date), None)
            }
        } else if let Some(end_date) = end_date.clone() {
            if end_date < start_date {
                (Some(start_date.clone()), duration)
            } else {
                (Some(end_date), duration)
            }
        } else {
            (end_date, duration)
        };
        DateRange {
            id,
            class,
            start_date,
            end_date,
            duration,
            planned_duration,
            end_on_next,
            extra_attributes: Vec::new(),
        }
    }
}

prop_compose! {
    fn server_control_strategy()(
        can_skip_until in proptest::option::of((0.0f64..=60.0f64).prop_map(|v| (v * 1e5).round() / 1e5)),
        can_skip_dateranges in proptest::bool::ANY,
        hold_back in proptest::option::of((0.0f64..=30.0f64).prop_map(|v| (v * 1e5).round() / 1e5)),
        part_hold_back in proptest::option::of((0.0f64..=10.0f64).prop_map(|v| (v * 1e5).round() / 1e5)),
        can_block_reload in proptest::bool::ANY,
    ) -> ServerControl {
        ServerControl {
            can_skip_until, can_skip_dateranges,
            hold_back, part_hold_back, can_block_reload,
        }
    }
}

prop_compose! {
    fn part_strategy()(
        uri in uri_strategy(),
        duration in (0.001f64..=10.0f64).prop_map(|v| (v * 1e5).round() / 1e5),
        independent in proptest::bool::ANY,
        byte_range in proptest::option::of(byterange_strategy()),
        gap in proptest::bool::ANY,
    ) -> Part {
        Part { uri, duration, independent, byte_range, gap }
    }
}

prop_compose! {
    fn skip_strategy()(
        skipped_segments in 1u64..=100u64,
        recently_removed_dateranges in proptest::collection::vec(safe_string(), 0..=3),
    ) -> Skip {
        Skip { skipped_segments, recently_removed_dateranges }
    }
}

prop_compose! {
    fn preload_hint_strategy()(
        hint_type in prop_oneof![Just(PreloadHintType::Part), Just(PreloadHintType::Map)],
        uri in uri_strategy(),
        byterange_start in proptest::option::of(0u64..=10_000_000u64),
        byterange_length in proptest::option::of(1u64..=10_000_000u64),
    ) -> PreloadHint {
        PreloadHint { hint_type, uri, byterange_start, byterange_length }
    }
}

prop_compose! {
    fn rendition_report_strategy()(
        uri in uri_strategy(),
        last_msn in 0u64..=100_000u64,
        last_part in proptest::option::of(0u64..=100u64),
    ) -> RenditionReport {
        RenditionReport { uri, last_msn: Some(last_msn), last_part }
    }
}

fn segment_strategy() -> impl Strategy<Value = Segment> {
    // フィールドが多いためネストしたタプルで 12 要素の上限を回避する
    (
        (
            (0.001f64..=60.0f64).prop_map(|v| (v * 1e5).round() / 1e5),
            proptest::option::of(safe_string()),
            uri_strategy(),
            proptest::option::of(byterange_strategy()),
            proptest::bool::ANY,
            proptest::option::of(key_strategy()),
        ),
        (
            proptest::option::of(map_strategy()),
            proptest::option::of(
                "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z".prop_map(|s| s),
            ),
            proptest::collection::vec(date_range_strategy(), 0..=2),
            proptest::bool::ANY,
            proptest::option::of(1u64..=50_000_000u64),
        ),
        (proptest::collection::vec(part_strategy(), 0..=3),),
    )
        .prop_map(
            |(
                (duration, title, uri, byte_range, discontinuity, key),
                (map, program_date_time, date_ranges, gap, bitrate),
                (parts,),
            )| Segment {
                duration,
                title,
                uri,
                byte_range,
                discontinuity,
                key,
                map,
                program_date_time,
                date_ranges,
                gap,
                bitrate,
                parts,
            },
        )
}

fn media_playlist_strategy() -> impl Strategy<Value = MediaPlaylist> {
    (
        (
            proptest::option::of(1u8..=12u8),
            1u32..=60u32,
            proptest::option::of(0u64..=100_000u64),
            proptest::option::of(0u64..=100u64),
            proptest::option::of(prop_oneof![
                Just(PlaylistType::Event),
                Just(PlaylistType::Vod),
            ]),
            proptest::bool::ANY,
        ),
        (
            proptest::bool::ANY,
            proptest::option::of(start_point_strategy()),
            proptest::option::of(server_control_strategy()),
            proptest::option::of((0.001f64..=10.0f64).prop_map(|v| PartInf {
                part_target: (v * 1e5).round() / 1e5,
            })),
            proptest::collection::vec(segment_strategy(), 1..=8),
            proptest::option::of(skip_strategy()),
            proptest::collection::vec(preload_hint_strategy(), 0..=2),
            proptest::collection::vec(rendition_report_strategy(), 0..=2),
            proptest::bool::ANY,
        ),
    )
        .prop_map(
            |(
                (
                    version,
                    target_duration,
                    media_sequence,
                    discontinuity_sequence,
                    playlist_type,
                    i_frames_only,
                ),
                (
                    independent_segments,
                    start,
                    server_control,
                    part_inf,
                    mut segments,
                    skip,
                    mut preload_hints,
                    rendition_reports,
                    end_list,
                ),
            )| {
                // #EXT-X-MAP は M3U8 仕様上 unset 不可のため、一度 Some になったら以降も維持する
                let mut last_map: Option<Map> = None;
                let mut previous_ranged_uri: Option<String> = None;
                let mut seen_date_ranges: HashMap<String, DateRange> = HashMap::new();
                for seg in &mut segments {
                    if seg.map.is_some() {
                        last_map = seg.map.clone();
                    } else if last_map.is_some() {
                        seg.map = last_map.clone();
                    }
                    if let Some(byte_range) = &mut seg.byte_range {
                        if byte_range.offset.is_none()
                            && previous_ranged_uri.as_deref() != Some(seg.uri.as_str())
                        {
                            byte_range.offset = Some(0);
                        }
                        previous_ranged_uri = Some(seg.uri.clone());
                    } else {
                        previous_ranged_uri = None;
                    }
                    if !seg.date_ranges.is_empty() && seg.program_date_time.is_none() {
                        seg.program_date_time = Some(String::from("2026-03-18T00:00:00Z"));
                    }
                    for date_range in &mut seg.date_ranges {
                        if let Some(previous) = seen_date_ranges.get(&date_range.id) {
                            date_range.start_date = previous.start_date.clone();
                            date_range.end_on_next = previous.end_on_next;
                            if previous.class.is_some() {
                                date_range.class = previous.class.clone();
                            }
                            if previous.end_date.is_some() {
                                date_range.end_date = previous.end_date.clone();
                            }
                            if previous.duration.is_some() {
                                date_range.duration = previous.duration;
                            }
                            if previous.planned_duration.is_some() {
                                date_range.planned_duration = previous.planned_duration;
                            }
                            if date_range.end_on_next {
                                date_range.end_date = None;
                                date_range.duration = None;
                                if date_range.class.is_none() {
                                    date_range.class = Some(String::from("class"));
                                }
                            }
                            // start_date を上書きした後、end_date >= start_date を再検証する
                            if let Some(ref end_date) = date_range.end_date
                                && *end_date < date_range.start_date
                            {
                                date_range.end_date = Some(date_range.start_date.clone());
                            }
                        } else {
                            seen_date_ranges.insert(date_range.id.clone(), date_range.clone());
                        }
                    }
                }
                let has_parts = segments.iter().any(|seg| !seg.parts.is_empty());
                let part_inf = if has_parts {
                    part_inf.or(Some(PartInf { part_target: 0.5 }))
                } else {
                    part_inf
                };
                let server_control = if let Some(mut control) = server_control {
                    if control.can_skip_dateranges && control.can_skip_until.is_none() {
                        control.can_skip_until = Some(24.0);
                    }
                    if part_inf.is_some() && control.part_hold_back.is_none() {
                        control.part_hold_back = Some(1.5);
                    }
                    Some(control)
                } else if part_inf.is_some() {
                    Some(ServerControl {
                        can_skip_until: None,
                        can_skip_dateranges: false,
                        hold_back: None,
                        part_hold_back: Some(1.5),
                        can_block_reload: false,
                    })
                } else {
                    None
                };
                if end_list {
                    preload_hints.clear();
                }
                // EXTINF duration を丸めた値が target_duration 以下になるよう調整する
                let max_rounded_duration = segments
                    .iter()
                    .map(|seg| seg.duration.round() as u32)
                    .max()
                    .unwrap_or(1);
                let target_duration = target_duration.max(max_rounded_duration);
                // HOLD-BACK は target_duration * 3 以上にする
                let server_control = server_control.map(|mut control| {
                    if let Some(hold_back) = &mut control.hold_back {
                        let min = f64::from(target_duration) * 3.0;
                        if *hold_back < min {
                            *hold_back = min;
                        }
                    }
                    if let (Some(part_hold_back), Some(pi)) =
                        (&mut control.part_hold_back, &part_inf)
                        && *part_hold_back < pi.part_target
                    {
                        *part_hold_back = pi.part_target;
                    }
                    control
                });
                // RENDITION-REPORT に LAST-PART を補完する
                let rendition_reports = if has_parts {
                    rendition_reports
                        .into_iter()
                        .map(|mut r| {
                            if r.last_part.is_none() {
                                r.last_part = Some(0);
                            }
                            r
                        })
                        .collect()
                } else {
                    rendition_reports
                };
                let required_version = required_media_version(i_frames_only, &segments);
                let version = match version {
                    Some(version) => Some(version.max(required_version)),
                    None if required_version > 1 => Some(required_version),
                    None => None,
                };
                MediaPlaylist {
                    version,
                    target_duration,
                    media_sequence,
                    discontinuity_sequence,
                    playlist_type,
                    i_frames_only,
                    independent_segments,
                    start,
                    server_control,
                    part_inf,
                    variable_definitions: Vec::new(),
                    segments,
                    skip,
                    preload_hints,
                    rendition_reports,
                    end_list,
                }
            },
        )
}

fn required_media_version(i_frames_only: bool, segments: &[Segment]) -> u8 {
    let mut required_version = if i_frames_only { 4 } else { 1 };

    for segment in segments {
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
            required_version = required_version.max(if i_frames_only { 5 } else { 6 });
        }
    }

    required_version
}

proptest! {
    /// Media Playlist のラウンドトリップ: write → parse → 元と一致する
    #[test]
    fn roundtrip_media_playlist(playlist in media_playlist_strategy()) {
        let text = write_media_playlist(&playlist);
        let parsed = parse_media_playlist(&text)
            .expect("write で生成した M3U8 は必ずパースできる");
        prop_assert_eq!(playlist, parsed);
    }
}
