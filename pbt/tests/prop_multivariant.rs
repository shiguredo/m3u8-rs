use noprop::{TestCaseContext, TestResult};
use shiguredo_m3u8::{
    multivariant::{
        ClosedCaptions, ContentSteering, EncryptionMethod, HdcpLevel, IFrameStream, Key, MediaType,
        MultivariantPlaylist, Rendition, Resolution, SessionData, SessionDataValue, StartPoint,
        VariantStream, VideoRange,
    },
    parse_multivariant_playlist, write_multivariant_playlist,
};
use std::cell::Cell;
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

/// `WxH` 形式の解像度を生成する
fn sample_resolution(ctx: &mut TestCaseContext) -> Resolution {
    Resolution {
        width: noprop::sample_usize_in(ctx, 1..=7680) as u32,
        height: noprop::sample_usize_in(ctx, 1..=4320) as u32,
    }
}

/// HDCP レベルを生成する
fn sample_hdcp_level(ctx: &mut TestCaseContext) -> HdcpLevel {
    match noprop::sample_weighted_index(ctx, &[1, 1, 1]) {
        0 => HdcpLevel::None,
        1 => HdcpLevel::Type0,
        _ => HdcpLevel::Type1,
    }
}

/// ビデオレンジを生成する
fn sample_video_range(ctx: &mut TestCaseContext) -> VideoRange {
    match noprop::sample_weighted_index(ctx, &[1, 1, 1]) {
        0 => VideoRange::Sdr,
        1 => VideoRange::Hlg,
        _ => VideoRange::Pq,
    }
}

/// `#EXT-X-SESSION-KEY` を生成する
///
/// `EncryptionMethod::None` は SESSION-KEY で許可されないため除外する
fn sample_session_key(ctx: &mut TestCaseContext) -> Key {
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
        // {:.5} で書き出すため 5 桁に丸めてラウンドトリップを保証する
        time_offset: round5(noprop::sample_f64_in(ctx, -3600.0, 3600.0)),
        precise: noprop::sample_bool(ctx),
    }
}

/// CLOSED-CAPTIONS 属性値を生成する
fn sample_closed_captions(ctx: &mut TestCaseContext) -> ClosedCaptions {
    match noprop::sample_weighted_index(ctx, &[1, 1]) {
        0 => ClosedCaptions::None,
        _ => ClosedCaptions::GroupId(sample_safe_string(ctx)),
    }
}

/// `#EXT-X-STREAM-INF` を生成する
fn sample_variant_stream(ctx: &mut TestCaseContext) -> VariantStream {
    VariantStream {
        bandwidth: noprop::sample_u64_in(ctx, 1..=50_000_000),
        average_bandwidth: sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 1..=50_000_000)),
        codecs: sample_option(ctx, sample_safe_string),
        supplemental_codecs: sample_option(ctx, sample_safe_string),
        resolution: sample_option(ctx, sample_resolution),
        frame_rate: sample_option(ctx, |ctx| round5(noprop::sample_f64_in(ctx, 1.0, 120.0))),
        hdcp_level: sample_option(ctx, sample_hdcp_level),
        allowed_cpc: sample_option(ctx, sample_safe_string),
        video_range: sample_option(ctx, sample_video_range),
        audio: sample_option(ctx, sample_safe_string),
        video: sample_option(ctx, sample_safe_string),
        subtitles: sample_option(ctx, sample_safe_string),
        closed_captions: sample_option(ctx, sample_closed_captions),
        name: sample_option(ctx, sample_safe_string),
        stable_variant_id: sample_option(ctx, sample_safe_string),
        pathway_id: sample_option(ctx, sample_safe_string),
        uri: sample_uri(ctx),
    }
}

/// メディア種別を生成する
fn sample_media_type(ctx: &mut TestCaseContext) -> MediaType {
    match noprop::sample_weighted_index(ctx, &[1, 1, 1, 1]) {
        0 => MediaType::Audio,
        1 => MediaType::Video,
        2 => MediaType::Subtitles,
        _ => MediaType::ClosedCaptions,
    }
}

/// `#EXT-X-MEDIA` を生成する
fn sample_rendition(ctx: &mut TestCaseContext) -> Rendition {
    let media_type = sample_media_type(ctx);
    let group_id = sample_safe_string(ctx);
    let name = sample_safe_string(ctx);
    let uri = sample_option(ctx, sample_uri);
    let language = sample_option(ctx, sample_safe_string);
    let assoc_language = sample_option(ctx, sample_safe_string);
    let default = noprop::sample_bool(ctx);
    let autoselect = noprop::sample_bool(ctx);
    let forced = noprop::sample_bool(ctx);
    let instream_id = sample_option(ctx, sample_safe_string);
    let characteristics = sample_option(ctx, sample_safe_string);
    let channels = sample_option(ctx, sample_safe_string);
    let bit_depth = sample_option(ctx, |ctx| noprop::sample_usize_in(ctx, 0..=64) as u64);
    let sample_rate = sample_option(ctx, |ctx| noprop::sample_usize_in(ctx, 1..=384_000) as u64);
    let stable_rendition_id = sample_option(ctx, sample_safe_string);
    let autoselect = if default { true } else { autoselect };
    match media_type {
        MediaType::ClosedCaptions => Rendition {
            media_type,
            group_id,
            name,
            uri: None,
            language,
            assoc_language,
            default,
            autoselect,
            forced: false,
            instream_id: Some(String::from("CC1")),
            characteristics,
            channels: None,
            bit_depth: None,
            sample_rate: None,
            stable_rendition_id,
        },
        MediaType::Subtitles => Rendition {
            media_type,
            group_id,
            name,
            uri: Some(uri.unwrap_or_else(|| String::from("subtitles.m3u8"))),
            language,
            assoc_language,
            default,
            autoselect,
            forced,
            instream_id,
            characteristics,
            channels: None,
            bit_depth: None,
            sample_rate: None,
            stable_rendition_id,
        },
        MediaType::Audio => Rendition {
            media_type,
            group_id,
            name,
            uri,
            language,
            assoc_language,
            default,
            autoselect,
            forced: false,
            instream_id,
            characteristics,
            channels,
            bit_depth,
            sample_rate,
            stable_rendition_id,
        },
        MediaType::Video => Rendition {
            media_type,
            group_id,
            name,
            uri,
            language,
            assoc_language,
            default,
            autoselect,
            forced: false,
            instream_id,
            characteristics,
            channels: None,
            bit_depth: None,
            sample_rate: None,
            stable_rendition_id,
        },
    }
}

/// `#EXT-X-I-FRAME-STREAM-INF` を生成する
fn sample_i_frame_stream(ctx: &mut TestCaseContext) -> IFrameStream {
    IFrameStream {
        bandwidth: noprop::sample_u64_in(ctx, 1..=50_000_000),
        average_bandwidth: sample_option(ctx, |ctx| noprop::sample_u64_in(ctx, 1..=50_000_000)),
        codecs: sample_option(ctx, sample_safe_string),
        supplemental_codecs: sample_option(ctx, sample_safe_string),
        resolution: sample_option(ctx, sample_resolution),
        hdcp_level: sample_option(ctx, sample_hdcp_level),
        video_range: sample_option(ctx, sample_video_range),
        video: sample_option(ctx, sample_safe_string),
        pathway_id: sample_option(ctx, sample_safe_string),
        uri: sample_uri(ctx),
    }
}

/// `#EXT-X-SESSION-DATA` を生成する
fn sample_session_data(ctx: &mut TestCaseContext) -> SessionData {
    let data_id = sample_safe_string(ctx);
    let value = if noprop::sample_bool(ctx) {
        SessionDataValue::Value(sample_safe_string(ctx))
    } else {
        SessionDataValue::Uri(sample_uri(ctx))
    };
    let language = sample_option(ctx, sample_safe_string);
    SessionData {
        data_id,
        value,
        language,
    }
}

/// Multivariant Playlist を生成する
fn sample_multivariant_playlist(ctx: &mut TestCaseContext) -> MultivariantPlaylist {
    let version = sample_option(ctx, |ctx| noprop::sample_usize_in(ctx, 1..=12) as u8);
    let independent_segments = noprop::sample_bool(ctx);
    let start = sample_option(ctx, sample_start_point);
    let content_steering_server_uri = sample_option(ctx, sample_uri);
    let content_steering_pathway_id = sample_option(ctx, sample_safe_string);
    let mut variant_streams = sample_vec(ctx, 0..=4, sample_variant_stream);
    let renditions = sample_vec(ctx, 0..=4, sample_rendition);
    let mut i_frame_streams = sample_vec(ctx, 0..=2, sample_i_frame_stream);
    let session_data = sample_vec(ctx, 0..=2, sample_session_data);
    let session_keys = sample_vec(ctx, 0..=2, sample_session_key);

    // GROUP-ID は名前空間ごとに一意でなければならないため、インデックスを付けて重複を避ける
    let renditions = renditions
        .into_iter()
        .enumerate()
        .map(|(i, mut rendition)| {
            rendition.group_id = format!("{}-{i}", rendition.group_id);
            rendition
        })
        .collect::<Vec<_>>();
    let content_steering = if let Some(server_uri) = content_steering_server_uri {
        let pathway_id = if variant_streams.is_empty() && i_frame_streams.is_empty() {
            None
        } else {
            let pathway_id =
                content_steering_pathway_id.unwrap_or_else(|| String::from("pathway-0"));
            if let Some(stream) = variant_streams.first_mut() {
                stream.pathway_id = Some(pathway_id.clone());
            } else if let Some(stream) = i_frame_streams.first_mut() {
                stream.pathway_id = Some(pathway_id.clone());
            }
            Some(pathway_id)
        };
        Some(ContentSteering {
            server_uri,
            pathway_id,
        })
    } else {
        None
    };
    for stream in &mut variant_streams {
        stream.audio = first_group_id(&renditions, MediaType::Audio);
        stream.video = first_group_id(&renditions, MediaType::Video);
        stream.subtitles = first_group_id(&renditions, MediaType::Subtitles);
        stream.closed_captions =
            first_group_id(&renditions, MediaType::ClosedCaptions).map(ClosedCaptions::GroupId);
        if stream.subtitles.is_some() {
            let codecs = stream.codecs.get_or_insert_with(String::new);
            if !codecs
                .split(',')
                .map(str::trim)
                .any(|codec| codec.eq_ignore_ascii_case("wvtt"))
            {
                if !codecs.is_empty() {
                    codecs.push(',');
                }
                codecs.push_str("wvtt");
            }
        }
    }
    let mut deduped_session_keys = Vec::new();
    for key in session_keys {
        if !deduped_session_keys.contains(&key) {
            deduped_session_keys.push(key);
        }
    }
    let mut deduped_session_data = Vec::new();
    let mut seen_session_data = std::collections::HashSet::new();
    for session in session_data {
        let signature = (session.data_id.clone(), session.language.clone());
        if seen_session_data.insert(signature) {
            deduped_session_data.push(session);
        }
    }
    MultivariantPlaylist {
        version: match version {
            Some(version) => Some(version.max(required_multivariant_version(&renditions))),
            None if required_multivariant_version(&renditions) > 1 => {
                Some(required_multivariant_version(&renditions))
            }
            None => None,
        },
        independent_segments,
        start,
        variable_definitions: Vec::new(),
        content_steering,
        variant_streams,
        renditions,
        i_frame_streams,
        session_data: deduped_session_data,
        session_keys: deduped_session_keys,
    }
}

/// Rendition の内容から必要になる Multivariant Playlist のバージョン番号を求める
fn required_multivariant_version(renditions: &[Rendition]) -> u8 {
    if renditions
        .iter()
        .filter_map(|rendition| rendition.instream_id.as_deref())
        .any(|instream_id| instream_id.starts_with("SERVICE"))
    {
        7
    } else {
        1
    }
}

/// 指定したメディア種別の Rendition の GROUP-ID を返す
fn first_group_id(renditions: &[Rendition], media_type: MediaType) -> Option<String> {
    renditions
        .iter()
        .find(|rendition| rendition.media_type == media_type)
        .map(|rendition| rendition.group_id.clone())
}

/// Multivariant Playlist のラウンドトリップ: write → parse → 元と一致する
///
/// 生成が偏って特定の要素を一度も通らないまま成功することを防ぐため、
/// 主要な要素ごとの到達回数を数えてから検証する
#[test]
fn roundtrip_multivariant_playlist() -> TestResult {
    let seed = noprop::seed_from_env_or_time("M3U8_PBT_SEED")?;
    let with_variant_streams = Cell::new(0usize);
    let with_renditions = Cell::new(0usize);
    let with_i_frame_streams = Cell::new(0usize);
    let with_content_steering = Cell::new(0usize);
    let with_session_data = Cell::new(0usize);
    let mut runner = noprop::Runner::new(seed);

    runner.run(256, |ctx| {
        let playlist = sample_multivariant_playlist(ctx);
        if !playlist.variant_streams.is_empty() {
            with_variant_streams.set(with_variant_streams.get() + 1);
        }
        if !playlist.renditions.is_empty() {
            with_renditions.set(with_renditions.get() + 1);
        }
        if !playlist.i_frame_streams.is_empty() {
            with_i_frame_streams.set(with_i_frame_streams.get() + 1);
        }
        if playlist.content_steering.is_some() {
            with_content_steering.set(with_content_steering.get() + 1);
        }
        if !playlist.session_data.is_empty() {
            with_session_data.set(with_session_data.get() + 1);
        }
        let text = write_multivariant_playlist(&playlist);
        let parsed =
            parse_multivariant_playlist(&text).expect("write で生成した M3U8 は必ずパースできる");
        assert_eq!(playlist, parsed);
        Ok(())
    })?;

    assert!(
        with_variant_streams.get() > 0,
        "variant stream を生成するケースがなかった\n{runner}"
    );
    assert!(
        with_renditions.get() > 0,
        "rendition を生成するケースがなかった\n{runner}"
    );
    assert!(
        with_i_frame_streams.get() > 0,
        "I-frame stream を生成するケースがなかった\n{runner}"
    );
    assert!(
        with_content_steering.get() > 0,
        "content steering を生成するケースがなかった\n{runner}"
    );
    assert!(
        with_session_data.get() > 0,
        "session data を生成するケースがなかった\n{runner}"
    );
    Ok(())
}
