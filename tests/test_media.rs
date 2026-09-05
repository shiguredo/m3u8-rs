use shiguredo_m3u8::{
    error::ErrorKind, multivariant::EncryptionMethod, parse_media_playlist,
    parse_media_playlist_with_context, parse_multivariant_playlist, write_media_playlist,
};

#[test]
fn parse_media_playlist_keeps_playlist_tail_ll_hls_tags_in_playlist() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:10\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
        "#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"next.part\"\n",
        "#EXT-X-RENDITION-REPORT:URI=\"low.m3u8\",LAST-MSN=10,LAST-PART=1\n",
    );

    let playlist = parse_media_playlist(input).expect("メディアプレイリストのパースに成功すること");

    assert_eq!(playlist.preload_hints.len(), 1);
    assert_eq!(playlist.rendition_reports.len(), 1);
}

#[test]
fn parse_media_playlist_keeps_multiple_dateranges_per_segment() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:00Z\"\n",
        "#EXT-X-DATERANGE:ID=\"ad-2\",START-DATE=\"2026-03-18T00:00:01Z\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let playlist = parse_media_playlist(input).expect("メディアプレイリストのパースに成功すること");

    assert_eq!(playlist.segments.len(), 1);
    assert_eq!(playlist.segments[0].date_ranges.len(), 2);
}

#[test]
fn parse_media_playlist_keeps_daterange_extra_attributes() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:00Z\",SCTE35-OUT=0xFC30,X-ASSET-LIST=\"ads.json\",CUE=\"PRE,ONCE\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let playlist = parse_media_playlist(input).expect("拡張 DATERANGE のパースに成功すること");
    let date_range = &playlist.segments[0].date_ranges[0];

    assert_eq!(date_range.extra_attributes.len(), 3);
    assert_eq!(date_range.extra_attributes[0].name, "SCTE35-OUT");
    assert_eq!(date_range.extra_attributes[0].value, "0xFC30");
    assert!(!date_range.extra_attributes[0].quoted);
    assert_eq!(date_range.extra_attributes[1].name, "X-ASSET-LIST");
    assert!(date_range.extra_attributes[1].quoted);
    assert_eq!(date_range.extra_attributes[2].name, "CUE");
}

#[test]
fn parse_media_playlist_rejects_truncated_segment_at_eof() {
    let input = concat!("#EXTM3U\n", "#EXT-X-TARGETDURATION:4\n", "#EXTINF:4.0,\n",);

    let error = parse_media_playlist(input).expect_err("truncated playlist must fail");

    assert_eq!(error.kind(), &ErrorKind::UnexpectedEof);
}

#[test]
fn parse_media_playlist_rejects_duplicate_attribute_name() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PART:DURATION=0.5,DURATION=0.4,URI=\"part.0.m4s\"\n",
        "#EXTINF:4.0,\n",
        "segment.m4s\n",
    );

    let error = parse_media_playlist(input).expect_err("duplicate attribute must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "attribute-list",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_parts_without_part_inf() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PART:DURATION=0.5,URI=\"part.0.m4s\"\n",
        "#EXTINF:4.0,\n",
        "segment.m4s\n",
    );

    let error = parse_media_playlist(input).expect_err("missing part-inf must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::MissingTag {
            tag: "EXT-X-PART-INF",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_part_inf_without_part_hold_back() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-SERVER-CONTROL:HOLD-BACK=12.0\n",
        "#EXT-X-PART-INF:PART-TARGET=0.5\n",
        "#EXT-X-PART:DURATION=0.5,URI=\"part.0.m4s\"\n",
        "#EXTINF:4.0,\n",
        "segment.m4s\n",
    );

    let error = parse_media_playlist(input).expect_err("missing part-hold-back must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::MissingTag {
            tag: "PART-HOLD-BACK",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_can_skip_dateranges_without_can_skip_until() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-SERVER-CONTROL:CAN-SKIP-DATERANGES=YES,HOLD-BACK=12.0\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("missing can-skip-until must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::MissingTag {
            tag: "CAN-SKIP-UNTIL",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_endlist_with_preload_hint() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
        "#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"next.part\"\n",
        "#EXT-X-ENDLIST\n",
    );

    let error = parse_media_playlist(input).expect_err("endlist with preload hint must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-PRELOAD-HINT",
        }
    );
}

#[test]
fn parse_media_playlist_keeps_ext_x_skip() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-SKIP:SKIPPED-SEGMENTS=3,RECENTLY-REMOVED-DATERANGES=\"ad-1\tad-2\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let playlist =
        parse_media_playlist(input).expect("SKIP 付きプレイリストのパースに成功すること");
    let skip = playlist.skip.expect("SKIP が存在すること");

    assert_eq!(skip.skipped_segments, 3);
    assert_eq!(
        skip.recently_removed_dateranges,
        vec![String::from("ad-1"), String::from("ad-2")]
    );
}

#[test]
fn parse_media_playlist_rejects_duplicate_ext_x_skip() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-SKIP:SKIPPED-SEGMENTS=1\n",
        "#EXT-X-SKIP:SKIPPED-SEGMENTS=2\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("duplicate skip must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue { tag: "EXT-X-SKIP" }
    );
}

#[test]
fn parse_media_playlist_rejects_duplicate_ext_x_version() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:6\n",
        "#EXT-X-VERSION:7\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("duplicate version must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-VERSION",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_duplicate_endlist() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
        "#EXT-X-ENDLIST\n",
        "#EXT-X-ENDLIST\n",
    );

    let error = parse_media_playlist(input).expect_err("duplicate endlist must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-ENDLIST",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_multivariant_tag_mixture() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"ja\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("mixed playlist tags must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue { tag: "EXT-X-MEDIA" }
    );
}

#[test]
fn parse_media_playlist_rejects_first_byterange_without_offset() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-BYTERANGE:1200\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("missing byterange base must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-BYTERANGE",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_byterange_without_offset_after_uri_change() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-BYTERANGE:1200@0\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
        "#EXT-X-BYTERANGE:1200\n",
        "#EXTINF:4.0,\n",
        "other.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("byterange URI change must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-BYTERANGE",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_byterange_without_required_version() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-BYTERANGE:1200@0\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("missing version must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::MissingTag {
            tag: "EXT-X-VERSION"
        }
    );
}

#[test]
fn parse_media_playlist_rejects_map_with_too_low_version() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:5\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-MAP:URI=\"init.mp4\"\n",
        "#EXTINF:4.0,\n",
        "segment.m4s\n",
    );

    let error = parse_media_playlist(input).expect_err("map version must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-VERSION",
        }
    );
}

#[test]
fn parse_media_playlist_keeps_bis_key_method() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:10\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI=\"key.bin\"\n",
        "#EXT-X-MAP:URI=\"init.mp4\"\n",
        "#EXTINF:4.0,\n",
        "segment.m4s\n",
    );

    let playlist = parse_media_playlist(input).expect("bis 版 KEY メソッドのパースに成功すること");

    assert_eq!(
        playlist.segments[0]
            .key
            .as_ref()
            .expect("キーが存在すること")
            .method,
        EncryptionMethod::SampleAesCtr
    );
}

#[test]
fn parse_media_playlist_rejects_key_without_uri() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-KEY:METHOD=AES-128\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("key without uri must fail");

    assert_eq!(error.kind(), &ErrorKind::MissingTag { tag: "URI" });
}

#[test]
fn parse_media_playlist_rejects_key_method_none_with_other_attributes() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-KEY:METHOD=NONE,URI=\"key.bin\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("method none with uri must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue { tag: "EXT-X-KEY" }
    );
}

#[test]
fn parse_media_playlist_rejects_key_method_aes_256_gcm_with_iv() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:10\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-KEY:METHOD=AES-256-GCM,URI=\"key.bin\",IV=0x1\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("aes-256-gcm with iv must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue { attribute: "IV" }
    );
}

#[test]
fn parse_media_playlist_rejects_key_method_sample_aes_ctr_with_iv() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:10\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI=\"key.bin\",IV=0x1\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("sample-aes-ctr with iv must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue { attribute: "IV" }
    );
}

#[test]
fn parse_media_playlist_rejects_fmp4_segment_without_map() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:6\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "segment.m4s\n",
    );

    let error = parse_media_playlist(input).expect_err("fmp4 without map must fail");

    assert_eq!(error.kind(), &ErrorKind::MissingTag { tag: "EXT-X-MAP" });
}

#[test]
fn parse_media_playlist_rejects_fmp4_part_without_map() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:10\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-SERVER-CONTROL:HOLD-BACK=12.0,PART-HOLD-BACK=1.5\n",
        "#EXT-X-PART-INF:PART-TARGET=0.5\n",
        "#EXT-X-PART:DURATION=0.5,URI=\"part.0.m4s\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("fmp4 part without map must fail");

    assert_eq!(error.kind(), &ErrorKind::MissingTag { tag: "EXT-X-MAP" });
}

#[test]
fn parse_media_playlist_rejects_rendition_report_without_last_msn() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
        "#EXT-X-RENDITION-REPORT:URI=\"low.m3u8\"\n",
    );

    let error = parse_media_playlist(input).expect_err("missing last-msn must fail");

    assert_eq!(error.kind(), &ErrorKind::MissingTag { tag: "LAST-MSN" });
}

#[test]
fn parse_media_playlist_rejects_daterange_without_program_date_time() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:00Z\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("missing pdt must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::MissingTag {
            tag: "EXT-X-PROGRAM-DATE-TIME",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_end_on_next_without_class() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:00Z\",END-ON-NEXT=YES\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("missing class must fail");

    assert_eq!(error.kind(), &ErrorKind::MissingTag { tag: "CLASS" });
}

#[test]
fn parse_media_playlist_rejects_end_on_next_with_duration() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",CLASS=\"ad\",START-DATE=\"2026-03-18T00:00:00Z\",DURATION=4.0,END-ON-NEXT=YES\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("end-on-next duration must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "DURATION",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_daterange_end_date_before_start_date() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:10Z\",END-DATE=\"2026-03-18T00:00:09Z\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("end-date order must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "END-DATE",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_daterange_end_date_duration_mismatch() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:00Z\",END-DATE=\"2026-03-18T00:00:05Z\",DURATION=4.0\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("end-date duration mismatch must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "END-DATE",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_inconsistent_daterange_id() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:00Z\",CLASS=\"ad\"\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:01Z\",CLASS=\"ad\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    );

    let error = parse_media_playlist(input).expect_err("inconsistent id must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue {
            attribute: "START-DATE",
        }
    );
}

#[test]
fn parse_media_playlist_rejects_absolute_rendition_report_uri() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
        "#EXT-X-RENDITION-REPORT:URI=\"https://example.com/low.m3u8\",LAST-MSN=10\n",
    );

    let error = parse_media_playlist(input).expect_err("absolute URI must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidAttributeValue { attribute: "URI" }
    );
}

#[test]
fn parse_media_playlist_imports_variables_from_multivariant_playlist() {
    let multivariant = parse_multivariant_playlist(concat!(
        "#EXTM3U\n",
        "#EXT-X-DEFINE:NAME=\"host\",VALUE=\"cdn.example.com\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1000\n",
        "main.m3u8\n",
    ))
    .expect("Multivariant Playlist のパースに成功すること");

    let media = parse_media_playlist_with_context(
        concat!(
            "#EXTM3U\n",
            "#EXT-X-TARGETDURATION:4\n",
            "#EXT-X-DEFINE:IMPORT=\"host\"\n",
            "#EXTINF:4.0,\n",
            "https://{$host}/segment.ts\n",
        ),
        None,
        Some(&multivariant),
    )
    .expect("IMPORT 付きメディアプレイリストのパースに成功すること");

    assert_eq!(media.variable_definitions.len(), 1);
    assert_eq!(media.segments[0].uri, "https://cdn.example.com/segment.ts");
}

#[test]
fn parse_media_playlist_rejects_undefined_variable_reference() {
    let error = parse_media_playlist(concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "https://{$missing}/segment.ts\n",
    ))
    .expect_err("undefined variable must fail");

    assert_eq!(
        error.kind(),
        &ErrorKind::InvalidTagValue {
            tag: "EXT-X-DEFINE"
        }
    );
}

#[test]
fn write_media_playlist_snapshot() {
    let playlist = parse_media_playlist(concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:10\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-MEDIA-SEQUENCE:7\n",
        "#EXT-X-DISCONTINUITY-SEQUENCE:3\n",
        "#EXT-X-PLAYLIST-TYPE:EVENT\n",
        "#EXT-X-INDEPENDENT-SEGMENTS\n",
        "#EXT-X-START:TIME-OFFSET=1.50000,PRECISE=YES\n",
        "#EXT-X-SERVER-CONTROL:CAN-SKIP-UNTIL=12.00000,CAN-SKIP-DATERANGES=YES,HOLD-BACK=12.00000,PART-HOLD-BACK=1.50000,CAN-BLOCK-RELOAD=YES\n",
        "#EXT-X-PART-INF:PART-TARGET=0.50000\n",
        "#EXT-X-DEFINE:NAME=\"host\",VALUE=\"cdn.example.com\"\n",
        "#EXT-X-SKIP:SKIPPED-SEGMENTS=2,RECENTLY-REMOVED-DATERANGES=\"old-1\told-2\"\n",
        "#EXT-X-KEY:METHOD=AES-128,URI=\"https://{$host}/key.bin\",IV=0x1,KEYFORMAT=\"identity\",KEYFORMATVERSIONS=\"1\"\n",
        "#EXT-X-MAP:URI=\"init.mp4\",BYTERANGE=\"1000@0\"\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",CLASS=\"ad\",START-DATE=\"2026-03-18T00:00:00Z\",END-DATE=\"2026-03-18T00:00:04Z\",DURATION=4.0,PLANNED-DURATION=4.0\n",
        "#EXT-X-GAP\n",
        "#EXT-X-BITRATE:900000\n",
        "#EXT-X-PART:URI=\"https://{$host}/segment-7.part0.m4s\",DURATION=0.5,INDEPENDENT=YES,BYTERANGE=\"200@0\"\n",
        "#EXT-X-PART:URI=\"https://{$host}/segment-7.part1.m4s\",DURATION=0.5,GAP=YES\n",
        "#EXT-X-BYTERANGE:1200@1000\n",
        "#EXTINF:4.0,segment-title\n",
        "https://{$host}/segment-7.m4s\n",
        "#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"https://{$host}/segment-8.part0.m4s\",BYTERANGE-START=0,BYTERANGE-LENGTH=200\n",
        "#EXT-X-RENDITION-REPORT:URI=\"low.m3u8\",LAST-MSN=8,LAST-PART=1\n",
    ))
    .expect("スナップショット用メディアプレイリストのパースに成功すること");

    insta::assert_snapshot!(write_media_playlist(&playlist));
}

#[test]
fn write_media_playlist_keeps_daterange_extra_attributes() {
    let playlist = parse_media_playlist(concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXT-X-PROGRAM-DATE-TIME:2026-03-18T00:00:00Z\n",
        "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2026-03-18T00:00:00Z\",SCTE35-OUT=0xFC30,X-ASSET-LIST=\"ads.json\",CUE=\"PRE,ONCE\"\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    ))
    .expect("拡張 DATERANGE のパースに成功すること");

    let text = write_media_playlist(&playlist);

    assert!(text.contains("SCTE35-OUT=0xFC30"));
    assert!(text.contains("X-ASSET-LIST=\"ads.json\""));
    assert!(text.contains("CUE=\"PRE,ONCE\""));
}

#[test]
fn write_media_playlist_sanitizes_quoted_string_values() {
    let playlist = parse_media_playlist(concat!(
        "#EXTM3U\n",
        "#EXT-X-TARGETDURATION:4\n",
        "#EXTINF:4.0,\n",
        "segment.ts\n",
    ))
    .expect("プレイリストのパースに成功すること");

    let mut playlist = playlist;
    playlist.segments[0].map = Some(shiguredo_m3u8::media::Map {
        uri: String::from("init\"\n.mp4"),
        byte_range: None,
    });

    let text = write_media_playlist(&playlist);

    assert!(text.contains("#EXT-X-MAP:URI=\"init.mp4\""));
    assert!(!text.contains("init\"\n.mp4"));
}
