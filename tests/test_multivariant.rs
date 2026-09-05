use shiguredo_m3u8::{
    error::ErrorKind, multivariant::EncryptionMethod, parse_multivariant_playlist,
    parse_multivariant_playlist_with_uri, write_multivariant_playlist,
};

#[test]
fn parse_multivariant_playlist_rejects_session_data_with_value_and_uri() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-SESSION-DATA:DATA-ID=\"id\",VALUE=\"value\",URI=\"data.json\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("invalid session data must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-SESSION-DATA",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_invalid_yes_no_attribute() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\",DEFAULT=MAYBE\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("invalid bool must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "DEFAULT",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_invalid_version() {
    let input = concat!("#EXTM3U\n", "#EXT-X-VERSION:abc\n",);

    let error = parse_multivariant_playlist(input).expect_err("invalid version must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-VERSION",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_duplicate_ext_x_version() {
    let input = concat!("#EXTM3U\n", "#EXT-X-VERSION:6\n", "#EXT-X-VERSION:7\n",);

    let error = parse_multivariant_playlist(input).expect_err("duplicate version must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-VERSION",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_duplicate_attribute_name() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,TYPE=VIDEO,GROUP-ID=\"audio\",NAME=\"ja\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("duplicate attribute must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "attribute-list",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_unterminated_quoted_string() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio,NAME=\"ja\"\n",
    );

    let error =
        parse_multivariant_playlist(input).expect_err("unterminated quoted string must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "attribute-list",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_forced_on_non_subtitles() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\",FORCED=YES\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("invalid forced must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "FORCED",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_closed_captions_without_instream_id() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"cc\",NAME=\"cc\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("missing instream-id must fail");

    assert_eq!(error.kind(), &ErrorKind::MissingTag { tag: "INSTREAM-ID" });
}

#[test]
fn parse_multivariant_playlist_rejects_closed_captions_with_uri() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"cc\",NAME=\"cc\",URI=\"cc.m3u8\",INSTREAM-ID=\"CC1\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("closed captions URI must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue { attribute: "URI" }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_subtitles_without_uri() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"ja\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("subtitles URI must fail");

    assert_eq!(error.kind(), &ErrorKind::MissingTag { tag: "URI" });
}

#[test]
fn parse_multivariant_playlist_keeps_bis_media_attributes() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\",URI=\"audio.m3u8\",CHANNELS=\"16/JOC\",BIT-DEPTH=24,SAMPLE-RATE=48000\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000,AUDIO=\"audio\"\n",
        "main.m3u8\n",
    );

    let playlist =
        parse_multivariant_playlist(input).expect("bis 版メディア属性のパースに成功すること");
    let rendition = &playlist.renditions[0];

    assert_eq!(rendition.channels.as_deref(), Some("16/JOC"));
    assert_eq!(rendition.bit_depth, Some(24));
    assert_eq!(rendition.sample_rate, Some(48_000));
}

#[test]
fn parse_multivariant_playlist_rejects_bit_depth_on_non_audio() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"ja\",URI=\"sub.m3u8\",BIT-DEPTH=24\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("non-audio bit-depth must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "BIT-DEPTH",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_invalid_closed_captions_instream_id() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"cc\",NAME=\"cc\",INSTREAM-ID=\"SERVICE64\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("invalid instream-id must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "INSTREAM-ID",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_duplicate_rendition_name_in_group() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\"\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("duplicate name must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue { attribute: "NAME" }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_multiple_default_renditions_in_group() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\",DEFAULT=YES,AUTOSELECT=YES\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"en\",DEFAULT=YES,AUTOSELECT=YES\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("multiple defaults must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "DEFAULT",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_session_key_with_method_none() {
    let input = concat!("#EXTM3U\n", "#EXT-X-SESSION-KEY:METHOD=NONE\n",);

    let error = parse_multivariant_playlist(input).expect_err("session key none must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "METHOD",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_duplicate_session_keys() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-SESSION-KEY:METHOD=AES-128,URI=\"key.bin\"\n",
        "#EXT-X-SESSION-KEY:METHOD=AES-128,URI=\"key.bin\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("duplicate session key must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-SESSION-KEY",
        }
    );
}

#[test]
fn parse_multivariant_playlist_keeps_bis_session_key_method() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-SESSION-KEY:METHOD=AES-256-GCM,URI=\"key.bin\"\n",
    );

    let playlist = parse_multivariant_playlist(input)
        .expect("bis 版 SESSION-KEY メソッドのパースに成功すること");

    assert_eq!(playlist.session_keys[0].method, EncryptionMethod::Aes256Gcm);
}

#[test]
fn parse_multivariant_playlist_rejects_session_key_without_uri() {
    let input = concat!("#EXTM3U\n", "#EXT-X-SESSION-KEY:METHOD=AES-128\n",);

    let error = parse_multivariant_playlist(input).expect_err("session key without uri must fail");

    assert_eq!(error.kind(), &ErrorKind::MissingTag { tag: "URI" });
}

#[test]
fn parse_multivariant_playlist_rejects_session_key_method_none_with_other_attributes() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-SESSION-KEY:METHOD=NONE,URI=\"key.bin\"\n",
    );

    let error =
        parse_multivariant_playlist(input).expect_err("session key method none with uri must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-SESSION-KEY",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_session_key_method_sample_aes_ctr_with_iv() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-SESSION-KEY:METHOD=SAMPLE-AES-CTR,URI=\"key.bin\",IV=0x1\n",
    );

    let error = parse_multivariant_playlist(input)
        .expect_err("session key sample-aes-ctr with iv must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue { attribute: "IV" }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_duplicate_session_data_language_pair() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-SESSION-DATA:DATA-ID=\"com.example.title\",VALUE=\"one\",LANGUAGE=\"ja\"\n",
        "#EXT-X-SESSION-DATA:DATA-ID=\"com.example.title\",VALUE=\"two\",LANGUAGE=\"ja\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("duplicate session-data must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-SESSION-DATA",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_unknown_audio_group_reference() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000,AUDIO=\"audio\"\n",
        "main.m3u8\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("unknown audio group must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue { attribute: "AUDIO" }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_duplicate_ext_x_start() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-START:TIME-OFFSET=0.0\n",
        "#EXT-X-START:TIME-OFFSET=1.0\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("duplicate start must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue { tag: "EXT-X-START" }
    );
}

#[test]
fn parse_multivariant_playlist_keeps_content_steering_and_pathway_id() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-CONTENT-STEERING:SERVER-URI=\"/steering.json\",PATHWAY-ID=\"CDN-A\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000,PATHWAY-ID=\"CDN-A\"\n",
        "main-a.m3u8\n",
    );

    let playlist =
        parse_multivariant_playlist(input).expect("CONTENT-STEERING のパースに成功すること");

    assert_eq!(
        playlist
            .content_steering
            .as_ref()
            .expect("CONTENT-STEERING が存在すること")
            .server_uri,
        "/steering.json"
    );
    assert_eq!(
        playlist.variant_streams[0].pathway_id.as_deref(),
        Some("CDN-A")
    );
}

#[test]
fn parse_multivariant_playlist_rejects_unknown_content_steering_pathway() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-CONTENT-STEERING:SERVER-URI=\"/steering.json\",PATHWAY-ID=\"CDN-A\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000,PATHWAY-ID=\"CDN-B\"\n",
        "main-b.m3u8\n",
    );

    let error =
        parse_multivariant_playlist(input).expect_err("unknown content steering pathway must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "PATHWAY-ID",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_media_tag_mixture() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000\n",
        "main.m3u8\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("mixed playlist tags must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-TARGETDURATION",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_unknown_closed_captions_group_reference() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000,CLOSED-CAPTIONS=\"cc\"\n",
        "main.m3u8\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("unknown cc group must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "CLOSED-CAPTIONS",
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_service_instream_id_without_version_7() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"cc\",NAME=\"cc\",INSTREAM-ID=\"SERVICE1\"\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("service version must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::MissingTag {
            tag: "EXT-X-VERSION"
        }
    );
}

#[test]
fn parse_multivariant_playlist_rejects_subtitles_group_without_wvtt_codec() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"ja\",URI=\"sub-ja.m3u8\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000,CODECS=\"avc1.64001f,mp4a.40.2\",SUBTITLES=\"subs\"\n",
        "main.m3u8\n",
    );

    let error = parse_multivariant_playlist(input).expect_err("missing wvtt must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "CODECS",
        }
    );
}

#[test]
fn parse_multivariant_playlist_substitutes_named_variables() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-DEFINE:NAME=\"host\",VALUE=\"cdn.example.com\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000\n",
        "https://{$host}/main.m3u8\n",
    );

    let playlist = parse_multivariant_playlist(input).expect("名前付き変数のパースに成功すること");

    assert_eq!(playlist.variable_definitions.len(), 1);
    assert_eq!(
        playlist.variant_streams[0].uri,
        "https://cdn.example.com/main.m3u8"
    );
}

#[test]
fn parse_multivariant_playlist_substitutes_query_params() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-DEFINE:QUERYPARAM=\"token\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000\n",
        "https://example.com/main.m3u8?token={$token}\n",
    );

    let playlist = parse_multivariant_playlist_with_uri(
        input,
        "https://playlist.example.com/master.m3u8?token=abc%20123",
    )
    .expect("QUERYPARAM のパースに成功すること");

    assert_eq!(
        playlist.variant_streams[0].uri,
        "https://example.com/main.m3u8?token=abc 123"
    );
}

#[test]
fn write_multivariant_playlist_snapshot() {
    let playlist = parse_multivariant_playlist_with_uri(
        concat!(
            "#EXTM3U\n",
            "#EXT-X-VERSION:10\n",
            "#EXT-X-INDEPENDENT-SEGMENTS\n",
            "#EXT-X-START:TIME-OFFSET=2.50000,PRECISE=YES\n",
            "#EXT-X-DEFINE:NAME=\"host\",VALUE=\"cdn.example.com\"\n",
            "#EXT-X-DEFINE:QUERYPARAM=\"token\"\n",
            "#EXT-X-CONTENT-STEERING:SERVER-URI=\"/steering.json\",PATHWAY-ID=\"CDN-A\"\n",
            "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\",URI=\"audio-ja.m3u8\",LANGUAGE=\"ja\",DEFAULT=YES,AUTOSELECT=YES,CHANNELS=\"2/JOC\",BIT-DEPTH=24,SAMPLE-RATE=48000,STABLE-RENDITION-ID=\"audio-ja\"\n",
            "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"ja\",URI=\"sub-ja.m3u8\",LANGUAGE=\"ja\",AUTOSELECT=YES,FORCED=YES,CHARACTERISTICS=\"public.accessibility.transcribes-spoken-dialog\",STABLE-RENDITION-ID=\"sub-ja\"\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=1500000,AVERAGE-BANDWIDTH=1200000,CODECS=\"avc1.64001f,mp4a.40.2,wvtt\",RESOLUTION=1280x720,FRAME-RATE=59.94000,HDCP-LEVEL=TYPE-0,ALLOWED-CPC=\"com.apple:ckc\",VIDEO-RANGE=PQ,AUDIO=\"audio\",SUBTITLES=\"subs\",CLOSED-CAPTIONS=NONE,NAME=\"main\",STABLE-VARIANT-ID=\"variant-main\",PATHWAY-ID=\"CDN-A\"\n",
            "https://{$host}/main.m3u8?token={$token}\n",
            "#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=350000,AVERAGE-BANDWIDTH=300000,CODECS=\"avc1.64001f\",RESOLUTION=1280x720,HDCP-LEVEL=TYPE-0,VIDEO=\"video-main\",PATHWAY-ID=\"CDN-A\",URI=\"iframe.m3u8\"\n",
            "#EXT-X-SESSION-DATA:DATA-ID=\"com.example.title\",VALUE=\"Example Title\",LANGUAGE=\"ja\"\n",
            "#EXT-X-SESSION-KEY:METHOD=AES-128,URI=\"https://{$host}/key.bin\",IV=0x1,KEYFORMAT=\"identity\",KEYFORMATVERSIONS=\"1\"\n",
        ),
        "https://playlist.example.com/master.m3u8?token=abc%20123",
    )
    .expect("スナップショット用 Multivariant Playlist のパースに成功すること");

    insta::assert_snapshot!(write_multivariant_playlist(&playlist));
}

#[test]
fn write_multivariant_playlist_sanitizes_quoted_string_values() {
    let playlist = parse_multivariant_playlist(concat!(
        "#EXTM3U\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000\n",
        "main.m3u8\n",
    ))
    .expect("プレイリストのパースに成功すること");

    let mut playlist = playlist;
    playlist.renditions[0].name = String::from("ja\"\nmain");
    playlist.variant_streams[0].name = Some(String::from("main\"\nname"));

    let text = write_multivariant_playlist(&playlist);

    assert!(!text.contains("ja\"\nmain"));
    assert!(text.contains("NAME=\"jamain\""));
    assert!(text.contains("NAME=\"mainname\""));
}
