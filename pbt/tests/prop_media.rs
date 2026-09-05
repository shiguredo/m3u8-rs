use noprop::{TestCaseContext, TestResult};
use shiguredo_m3u8::{
    media::{
        ByteRange, DateRange, Map, MediaPlaylist, Part, PartInf, PlaylistType, PreloadHint,
        PreloadHintType, RenditionReport, Segment, ServerControl, Skip,
    },
    multivariant::{EncryptionMethod, Key, StartPoint},
    parse_media_playlist, write_media_playlist,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::ops::RangeInclusive;

/// quoted-string に安全な文字に限定した文字プール (`[a-zA-Z0-9_./-]`)
const SAFE_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_./-";

/// 文字プールから 1〜指定長の文字列を生成する
fn sample_string_from_pool(ctx: &mut TestCaseContext, len_range: RangeInclusive<usize>) -> String {
    let len = noprop::sample_usize_in(ctx, len_range);
    let mut s = String::new();
    for _ in 0..len {
        let index = noprop::sample_usize_in(ctx, 0..SAFE_CHARS.len());
        s.push(SAFE_CHARS[index] as char);
    }
    s
}

/// `[a-zA-Z0-9_./-]{1,32}` 相当の文字列を生成する
fn sample_safe_string(ctx: &mut TestCaseContext) -> String {
    sample_string_from_pool(ctx, 1..=32)
}

/// `[a-zA-Z0-9_./-]{1,64}` 相当の URI を生成する
fn sample_uri(ctx: &mut TestCaseContext) -> String {
    sample_string_from_pool(ctx, 1..=64)
}

/// `[0-9a-fA-F]{32}` 相当の IV (16 バイトの 16 進表記) を生成する
fn sample_hex_iv(ctx: &mut TestCaseContext) -> String {
    const HEX_CHARS: &[u8] = b"0123456789abcdef";
    let mut s = String::new();
    for _ in 0..16 {
        let byte = noprop::sample_u8(ctx);
        s.push(HEX_CHARS[(byte >> 4) as usize] as char);
        s.push(HEX_CHARS[(byte & 0x0f) as usize] as char);
    }
    s
}

/// `[0-9/]{1,8}` 相当の KEYFORMATVERSIONS を生成する
fn sample_keyformat_versions(ctx: &mut TestCaseContext) -> String {
    const VERSION_CHARS: &[u8] = b"0123456789/";
    let len = noprop::sample_usize_in(ctx, 1..=8);
    let mut s = String::new();
    for _ in 0..len {
        let index = noprop::sample_usize_in(ctx, 0..VERSION_CHARS.len());
        s.push(VERSION_CHARS[index] as char);
    }
    s
}

/// `YYYY-MM-DDTHH:MM:SSZ` 形式の日時文字列を生成する
fn sample_timestamp(ctx: &mut TestCaseContext) -> String {
    let year = noprop::sample_usize_in(ctx, 2000..=2030);
    let month = noprop::sample_usize_in(ctx, 1..=12);
    let day = noprop::sample_usize_in(ctx, 1..=28);
    let hour = noprop::sample_usize_in(ctx, 0..=23);
    let minute = noprop::sample_usize_in(ctx, 0..=59);
    let second = noprop::sample_usize_in(ctx, 0..=59);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// 小数第 5 位に丸める (書き出し側の `{:.5}` と一致させるため)
fn round5(value: f64) -> f64 {
    (value * 1e5).round() / 1e5
}

/// 3/4 の確率で `Some` を生成する
fn sample_option<T>(
    ctx: &mut TestCaseContext,
    sampler: impl Fn(&mut TestCaseContext) -> T,
) -> Option<T> {
    if noprop::sample_weighted_index(ctx, &[3, 1]) == 0 {
        Some(sampler(ctx))
    } else {
        None
    }
}

/// 長さを先に引き、その長さ分の要素を生成する
fn sample_vec<T>(
    ctx: &mut TestCaseContext,
    len_range: RangeInclusive<usize>,
    sampler: impl Fn(&mut TestCaseContext) -> T,
) -> Vec<T> {
    let len = noprop::sample_usize_in(ctx, len_range);
    let mut vec = Vec::new();
    for _ in 0..len {
        vec.push(sampler(ctx));
    }
    vec
}

/// `n@o` または `n` で表されるバイトレンジを生成する
fn sample_byte_range(ctx: &mut TestCaseContext) -> ByteRange {
    ByteRange {
        length: noprop::sample_u64_in(ctx, 1..=10_000_000),
        offset: sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 0..=10_000_000)),
    }
}

/// Media Playlist 用の `#EXT-X-MAP` を生成する
fn sample_map(ctx: &mut TestCaseContext) -> Map {
    Map {
        uri: sample_uri(ctx),
        byte_range: sample_option(ctx, sample_byte_range),
    }
}

/// Media Playlist 用の `#EXT-X-KEY` を生成する
///
/// `EncryptionMethod::None` はパーサーが `key: None` に正規化するため除外する
fn sample_key(ctx: &mut TestCaseContext) -> Key {
    let method = match noprop::sample_weighted_index(ctx, &[1, 1, 1, 1]) {
        0 => EncryptionMethod::Aes128,
        1 => EncryptionMethod::Aes256Gcm,
        2 => EncryptionMethod::SampleAes,
        _ => EncryptionMethod::SampleAesCtr,
    };
    let uri = sample_option(ctx, sample_uri);
    let iv = sample_option(ctx, sample_hex_iv);
    let keyformat = sample_option(ctx, sample_safe_string);
    let keyformat_versions = sample_option(ctx, sample_keyformat_versions);
    let uri = Some(uri.unwrap_or_else(|| String::from("key.bin")));
    let iv = match method {
        EncryptionMethod::Aes256Gcm | EncryptionMethod::SampleAesCtr => None,
        _ => iv,
    };
    Key {
        method,
        uri,
        iv,
        keyformat,
        keyformat_versions,
    }
}

/// `#EXT-X-START` を生成する
fn sample_start_point(ctx: &mut TestCaseContext) -> StartPoint {
    StartPoint {
        time_offset: round5(noprop::sample_f64_in(ctx, -3600.0, 3600.0)),
        precise: noprop::sample_bool(ctx),
    }
}

/// `#EXT-X-DATERANGE` を生成する
fn sample_date_range(ctx: &mut TestCaseContext) -> DateRange {
    let id = sample_safe_string(ctx);
    let class = sample_option(ctx, sample_safe_string);
    let start_date = sample_timestamp(ctx);
    let end_date = sample_option(ctx, sample_timestamp);
    let duration = sample_option(ctx, |ctx| round5(noprop::sample_f64_in(ctx, 0.0, 86400.0)));
    let planned_duration =
        sample_option(ctx, |ctx| round5(noprop::sample_f64_in(ctx, 0.0, 86400.0)));
    let end_on_next = noprop::sample_bool(ctx);
    let class = if end_on_next {
        Some(class.unwrap_or_else(|| String::from("class")))
    } else {
        class
    };
    // END-DATE と DURATION は同時に指定しない。END-DATE は START-DATE 以上に正規化する
    let (end_date, duration) = if end_on_next {
        (None, None)
    } else if let Some(end_date) = end_date {
        let end_date = if end_date < start_date {
            start_date.clone()
        } else {
            end_date
        };
        (Some(end_date), None)
    } else {
        (None, duration)
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

/// `#EXT-X-SERVER-CONTROL` を生成する
fn sample_server_control(ctx: &mut TestCaseContext) -> ServerControl {
    ServerControl {
        can_skip_until: sample_option(ctx, |ctx| round5(noprop::sample_f64_in(ctx, 0.0, 60.0))),
        can_skip_dateranges: noprop::sample_bool(ctx),
        hold_back: sample_option(ctx, |ctx| round5(noprop::sample_f64_in(ctx, 0.0, 30.0))),
        part_hold_back: sample_option(ctx, |ctx| round5(noprop::sample_f64_in(ctx, 0.0, 10.0))),
        can_block_reload: noprop::sample_bool(ctx),
    }
}

/// `#EXT-X-PART` を生成する
fn sample_part(ctx: &mut TestCaseContext) -> Part {
    Part {
        uri: sample_uri(ctx),
        duration: round5(noprop::sample_f64_in(ctx, 0.001, 10.0)),
        independent: noprop::sample_bool(ctx),
        byte_range: sample_option(ctx, sample_byte_range),
        gap: noprop::sample_bool(ctx),
    }
}

/// `#EXT-X-SKIP` を生成する
fn sample_skip(ctx: &mut TestCaseContext) -> Skip {
    Skip {
        skipped_segments: noprop::sample_u64_in(ctx, 1..=100),
        recently_removed_dateranges: sample_vec(ctx, 0..=3, sample_safe_string),
    }
}

/// `#EXT-X-PRELOAD-HINT` を生成する
fn sample_preload_hint(ctx: &mut TestCaseContext) -> PreloadHint {
    let hint_type = if noprop::sample_bool(ctx) {
        PreloadHintType::Part
    } else {
        PreloadHintType::Map
    };
    PreloadHint {
        hint_type,
        uri: sample_uri(ctx),
        byterange_start: sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 0..=10_000_000)),
        byterange_length: sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 1..=10_000_000)),
    }
}

/// `#EXT-X-RENDITION-REPORT` を生成する
fn sample_rendition_report(ctx: &mut TestCaseContext) -> RenditionReport {
    RenditionReport {
        uri: sample_uri(ctx),
        last_msn: Some(noprop::sample_u64_in(ctx, 0..=100_000)),
        last_part: sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 0..=100)),
    }
}

/// セグメント 1 つ分を生成する
fn sample_segment(ctx: &mut TestCaseContext) -> Segment {
    Segment {
        duration: round5(noprop::sample_f64_in(ctx, 0.001, 60.0)),
        title: sample_option(ctx, sample_safe_string),
        uri: sample_uri(ctx),
        byte_range: sample_option(ctx, sample_byte_range),
        discontinuity: noprop::sample_bool(ctx),
        key: sample_option(ctx, sample_key),
        map: sample_option(ctx, sample_map),
        program_date_time: sample_option(ctx, sample_timestamp),
        date_ranges: sample_vec(ctx, 0..=2, sample_date_range),
        gap: noprop::sample_bool(ctx),
        bitrate: sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 1..=50_000_000)),
        parts: sample_vec(ctx, 0..=3, sample_part),
    }
}

/// Media Playlist を生成する
fn sample_media_playlist(ctx: &mut TestCaseContext) -> MediaPlaylist {
    let version = sample_option(ctx, |ctx| noprop::sample_usize_in(ctx, 1..=12) as u8);
    let target_duration = noprop::sample_usize_in(ctx, 1..=60) as u32;
    let media_sequence = sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 0..=100_000));
    let discontinuity_sequence = sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 0..=100));
    let playlist_type = sample_option(ctx, |ctx| {
        match noprop::sample_weighted_index(ctx, &[1, 1]) {
            0 => PlaylistType::Event,
            _ => PlaylistType::Vod,
        }
    });
    let i_frames_only = noprop::sample_bool(ctx);
    let independent_segments = noprop::sample_bool(ctx);
    let start = sample_option(ctx, sample_start_point);
    let server_control = sample_option(ctx, sample_server_control);
    let part_inf = sample_option(ctx, |ctx| PartInf {
        part_target: round5(noprop::sample_f64_in(ctx, 0.001, 10.0)),
    });
    let mut segments = sample_vec(ctx, 1..=8, sample_segment);
    let skip = sample_option(ctx, sample_skip);
    let mut preload_hints = sample_vec(ctx, 0..=2, sample_preload_hint);
    let rendition_reports = sample_vec(ctx, 0..=2, sample_rendition_report);
    let end_list = noprop::sample_bool(ctx);

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
            // END-DATE と DURATION は同時に指定できない。
            // 同一 ID のマージで previous の DURATION がコピーされ、
            // END-DATE と共存してしまうケースがあるため、ここで必ず排除する
            if date_range.end_date.is_some() {
                date_range.duration = None;
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
        if let (Some(part_hold_back), Some(pi)) = (&mut control.part_hold_back, &part_inf)
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
}

/// 機能ごとに必要になる Media Playlist のバージョン番号を求める
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

/// Media Playlist のラウンドトリップ: write → parse → 元と一致する
///
/// 生成が偏って特定の機能を一度も通らないまま成功することを防ぐため、
/// 主要な機能ごとの到達回数を数えてから検証する
#[test]
fn roundtrip_media_playlist() -> TestResult {
    let seed = noprop::seed_from_env_or_time("M3U8_PBT_SEED")?;
    let segments_with_key = Cell::new(0usize);
    let segments_with_map = Cell::new(0usize);
    let segments_with_byte_range = Cell::new(0usize);
    let segments_with_parts = Cell::new(0usize);
    let mut runner = noprop::Runner::new(seed);

    runner.run(256, |ctx| {
        let playlist = sample_media_playlist(ctx);
        for segment in &playlist.segments {
            if segment.key.is_some() {
                segments_with_key.set(segments_with_key.get() + 1);
            }
            if segment.map.is_some() {
                segments_with_map.set(segments_with_map.get() + 1);
            }
            if segment.byte_range.is_some() {
                segments_with_byte_range.set(segments_with_byte_range.get() + 1);
            }
            if !segment.parts.is_empty() {
                segments_with_parts.set(segments_with_parts.get() + 1);
            }
        }
        let text = write_media_playlist(&playlist);
        let parsed = parse_media_playlist(&text).expect("write で生成した M3U8 は必ずパースできる");
        assert_eq!(playlist, parsed);
        Ok(())
    })?;

    assert!(
        segments_with_key.get() > 0,
        "暗号化キー付きセグメントを生成するケースがなかった\n{runner}"
    );
    assert!(
        segments_with_map.get() > 0,
        "マップ付きセグメントを生成するケースがなかった\n{runner}"
    );
    assert!(
        segments_with_byte_range.get() > 0,
        "バイトレンジ付きセグメントを生成するケースがなかった\n{runner}"
    );
    assert!(
        segments_with_parts.get() > 0,
        "パーツ付きセグメントを生成するケースがなかった\n{runner}"
    );
    Ok(())
}
