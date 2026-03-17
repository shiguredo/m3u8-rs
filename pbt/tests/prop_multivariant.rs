use proptest::prelude::*;
use shiguredo_m3u8::{
    multivariant::{
        ClosedCaptions, EncryptionMethod, HdcpLevel, IFrameStream, Key, MediaType,
        MultivariantPlaylist, Rendition, Resolution, SessionData, SessionDataValue, StartPoint,
        VariantStream, VideoRange,
    },
    parse_multivariant_playlist, write_multivariant_playlist,
};

fn safe_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_./-]{1,32}".prop_map(|s| s)
}

fn uri_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_./-]{1,64}".prop_map(|s| s)
}

fn resolution_strategy() -> impl Strategy<Value = Resolution> {
    (1u32..=7680u32, 1u32..=4320u32).prop_map(|(w, h)| Resolution {
        width: w,
        height: h,
    })
}

fn hdcp_level_strategy() -> impl Strategy<Value = HdcpLevel> {
    prop_oneof![
        Just(HdcpLevel::None),
        Just(HdcpLevel::Type0),
        Just(HdcpLevel::Type1),
    ]
}

fn video_range_strategy() -> impl Strategy<Value = VideoRange> {
    prop_oneof![
        Just(VideoRange::Sdr),
        Just(VideoRange::Hlg),
        Just(VideoRange::Pq),
    ]
}

prop_compose! {
    fn key_strategy()(
        method in prop_oneof![
            Just(EncryptionMethod::None),
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
        let (uri, iv, keyformat, keyformat_versions) = match method {
            EncryptionMethod::None => (None, None, None, None),
            EncryptionMethod::Aes256Gcm | EncryptionMethod::SampleAesCtr => (
                Some(uri.unwrap_or_else(|| String::from("key.bin"))),
                None,
                keyformat,
                keyformat_versions,
            ),
            EncryptionMethod::Aes128 | EncryptionMethod::SampleAes => (
                Some(uri.unwrap_or_else(|| String::from("key.bin"))),
                iv,
                keyformat,
                keyformat_versions,
            ),
            _ => (None, None, None, None),
        };
        Key { method, uri, iv, keyformat, keyformat_versions }
    }
}

prop_compose! {
    fn session_key_strategy()(
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
        // {:.5} で書き出すため 5 桁に丸めてラウンドトリップを保証する
        let time_offset = (time_offset * 1e5).round() / 1e5;
        StartPoint { time_offset, precise }
    }
}

fn variant_stream_strategy() -> impl Strategy<Value = VariantStream> {
    // フィールドが多いためネストしたタプルで 12 要素の上限を回避する
    (
        (
            1u64..=50_000_000u64,
            proptest::option::of(1u64..=50_000_000u64),
            proptest::option::of(safe_string()),
            proptest::option::of(safe_string()),
            proptest::option::of(resolution_strategy()),
            proptest::option::of((1.0f64..=120.0f64).prop_map(|v| (v * 1e5).round() / 1e5)),
        ),
        (
            proptest::option::of(hdcp_level_strategy()),
            proptest::option::of(safe_string()),
            proptest::option::of(video_range_strategy()),
            proptest::option::of(safe_string()),
            proptest::option::of(safe_string()),
        ),
        (
            proptest::option::of(safe_string()),
            proptest::option::of(prop_oneof![
                Just(ClosedCaptions::None),
                safe_string().prop_map(ClosedCaptions::GroupId),
            ]),
            proptest::option::of(safe_string()),
            proptest::option::of(safe_string()),
            proptest::option::of(safe_string()),
            uri_strategy(),
        ),
    )
        .prop_map(
            |(
                (bandwidth, average_bandwidth, codecs, supplemental_codecs, resolution, frame_rate),
                (hdcp_level, allowed_cpc, video_range, audio, video),
                (subtitles, closed_captions, name, stable_variant_id, pathway_id, uri),
            )| VariantStream {
                bandwidth,
                average_bandwidth,
                codecs,
                supplemental_codecs,
                resolution,
                frame_rate,
                hdcp_level,
                allowed_cpc,
                video_range,
                audio,
                video,
                subtitles,
                closed_captions,
                name,
                stable_variant_id,
                pathway_id,
                uri,
            },
        )
}

fn media_type_strategy() -> impl Strategy<Value = MediaType> {
    prop_oneof![
        Just(MediaType::Audio),
        Just(MediaType::Video),
        Just(MediaType::Subtitles),
        Just(MediaType::ClosedCaptions),
    ]
}

fn rendition_strategy() -> impl Strategy<Value = Rendition> {
    (
        (
            media_type_strategy(),
            safe_string(),
            safe_string(),
            proptest::option::of(uri_strategy()),
            proptest::option::of(safe_string()),
            proptest::option::of(safe_string()),
        ),
        (
            proptest::bool::ANY,
            proptest::bool::ANY,
            proptest::bool::ANY,
            proptest::option::of(safe_string()),
            proptest::option::of(safe_string()),
            proptest::option::of(safe_string()),
            proptest::option::of(0u64..=64u64),
            proptest::option::of(1u64..=384_000u64),
            proptest::option::of(safe_string()),
        ),
    )
        .prop_map(
            |(
                (media_type, group_id, name, uri, language, assoc_language),
                (
                    default,
                    autoselect,
                    forced,
                    instream_id,
                    characteristics,
                    channels,
                    bit_depth,
                    sample_rate,
                    stable_rendition_id,
                ),
            )| {
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
            },
        )
}

prop_compose! {
    fn i_frame_stream_strategy()(
        bandwidth in 1u64..=50_000_000u64,
        average_bandwidth in proptest::option::of(1u64..=50_000_000u64),
        codecs in proptest::option::of(safe_string()),
        supplemental_codecs in proptest::option::of(safe_string()),
        resolution in proptest::option::of(resolution_strategy()),
        hdcp_level in proptest::option::of(hdcp_level_strategy()),
        video_range in proptest::option::of(video_range_strategy()),
        video in proptest::option::of(safe_string()),
        pathway_id in proptest::option::of(safe_string()),
        uri in uri_strategy(),
    ) -> IFrameStream {
        IFrameStream {
            bandwidth,
            average_bandwidth,
            codecs,
            supplemental_codecs,
            resolution,
            hdcp_level,
            video_range,
            video,
            pathway_id,
            uri,
        }
    }
}

prop_compose! {
    fn session_data_strategy()(
        data_id in safe_string(),
        value in prop_oneof![
            safe_string().prop_map(SessionDataValue::Value),
            uri_strategy().prop_map(SessionDataValue::Uri),
        ],
        language in proptest::option::of(safe_string()),
    ) -> SessionData {
        SessionData { data_id, value, language }
    }
}

prop_compose! {
    fn multivariant_playlist_strategy()(
        version in proptest::option::of(1u8..=12u8),
        independent_segments in proptest::bool::ANY,
        start in proptest::option::of(start_point_strategy()),
        content_steering_server_uri in proptest::option::of(uri_strategy()),
        content_steering_pathway_id in proptest::option::of(safe_string()),
        mut variant_streams in proptest::collection::vec(variant_stream_strategy(), 0..=4),
        renditions in proptest::collection::vec(rendition_strategy(), 0..=4),
        mut i_frame_streams in proptest::collection::vec(i_frame_stream_strategy(), 0..=2),
        session_data in proptest::collection::vec(session_data_strategy(), 0..=2),
        session_keys in proptest::collection::vec(session_key_strategy(), 0..=2),
    ) -> MultivariantPlaylist {
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
                        let pathway_id = content_steering_pathway_id
                            .unwrap_or_else(|| String::from("pathway-0"));
                        if let Some(stream) = variant_streams.first_mut() {
                            stream.pathway_id = Some(pathway_id.clone());
                        } else if let Some(stream) = i_frame_streams.first_mut() {
                            stream.pathway_id = Some(pathway_id.clone());
                        }
                        Some(pathway_id)
                    };
                    Some(shiguredo_m3u8::multivariant::ContentSteering {
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
                        first_group_id(&renditions, MediaType::ClosedCaptions)
                            .map(ClosedCaptions::GroupId);
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
            independent_segments, start,
            variable_definitions: Vec::new(),
            content_steering,
            variant_streams, renditions, i_frame_streams,
            session_data: deduped_session_data, session_keys: deduped_session_keys,
        }
    }
}

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

fn first_group_id(renditions: &[Rendition], media_type: MediaType) -> Option<String> {
    renditions
        .iter()
        .find(|rendition| rendition.media_type == media_type)
        .map(|rendition| rendition.group_id.clone())
}

proptest! {
    /// Multivariant Playlist のラウンドトリップ: write → parse → 元と一致する
    #[test]
    fn roundtrip_multivariant_playlist(playlist in multivariant_playlist_strategy()) {
        let text = write_multivariant_playlist(&playlist);
        let parsed = parse_multivariant_playlist(&text)
            .expect("write で生成した M3U8 は必ずパースできる");
        prop_assert_eq!(playlist, parsed);
    }
}
