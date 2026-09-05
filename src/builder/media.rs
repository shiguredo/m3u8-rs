//! Media Playlist の書き出し処理を提供する

use crate::{attribute::write_quoted_string, variable::VariableDefinition};
use crate::{
    builder::multivariant::{write_key_attrs, write_start_attrs},
    media::{
        ByteRange, DateRange, Map, MediaPlaylist, Part, PartInf, PlaylistType, PreloadHint,
        PreloadHintType, RenditionReport, Segment, ServerControl, Skip,
    },
};

/// Media Playlist を M3U8 テキストに書き出す
pub fn write(playlist: &MediaPlaylist) -> String {
    let mut out = String::from("#EXTM3U\n");

    if let Some(v) = playlist.version {
        out.push_str(&format!("#EXT-X-VERSION:{v}\n"));
    }
    out.push_str(&format!(
        "#EXT-X-TARGETDURATION:{}\n",
        playlist.target_duration
    ));
    if let Some(v) = playlist.media_sequence {
        out.push_str(&format!("#EXT-X-MEDIA-SEQUENCE:{v}\n"));
    }
    if let Some(v) = playlist.discontinuity_sequence {
        out.push_str(&format!("#EXT-X-DISCONTINUITY-SEQUENCE:{v}\n"));
    }
    if let Some(v) = &playlist.playlist_type {
        let s = match v {
            PlaylistType::Event => "EVENT",
            PlaylistType::Vod => "VOD",
        };
        out.push_str(&format!("#EXT-X-PLAYLIST-TYPE:{s}\n"));
    }
    if playlist.i_frames_only {
        out.push_str("#EXT-X-I-FRAMES-ONLY\n");
    }
    if playlist.independent_segments {
        out.push_str("#EXT-X-INDEPENDENT-SEGMENTS\n");
    }
    if let Some(start) = &playlist.start {
        out.push_str(&format!("#EXT-X-START:{}\n", write_start_attrs(start)));
    }
    if let Some(sc) = &playlist.server_control {
        out.push_str(&write_server_control(sc));
    }
    if let Some(pi) = &playlist.part_inf {
        out.push_str(&write_part_inf(pi));
    }
    for definition in &playlist.variable_definitions {
        out.push_str(&write_variable_definition(definition));
    }
    if let Some(skip) = &playlist.skip {
        out.push_str(&write_skip(skip));
    }

    // sticky タグ (#EXT-X-KEY / #EXT-X-MAP / #EXT-X-BITRATE) の現在値を追跡する
    let mut current_key: Option<crate::multivariant::Key> = None;
    let mut current_map: Option<Map> = None;
    let mut current_bitrate: Option<u64> = None;
    for segment in &playlist.segments {
        out.push_str(&write_segment(
            segment,
            &mut current_key,
            &mut current_map,
            &mut current_bitrate,
        ));
    }
    for hint in &playlist.preload_hints {
        out.push_str(&write_preload_hint(hint));
    }
    for report in &playlist.rendition_reports {
        out.push_str(&write_rendition_report(report));
    }

    if playlist.end_list {
        out.push_str("#EXT-X-ENDLIST\n");
    }

    out
}

fn write_segment(
    segment: &Segment,
    current_key: &mut Option<crate::multivariant::Key>,
    current_map: &mut Option<Map>,
    current_bitrate: &mut Option<u64>,
) -> String {
    let mut out = String::new();

    if segment.discontinuity {
        out.push_str("#EXT-X-DISCONTINUITY\n");
    }
    // key が変化したときのみ出力する。None への遷移は METHOD=NONE で reset する
    if segment.key != *current_key {
        match &segment.key {
            Some(key) => out.push_str(&format!("#EXT-X-KEY:{}\n", write_key_attrs(key))),
            None => out.push_str("#EXT-X-KEY:METHOD=NONE\n"),
        }
        *current_key = segment.key.clone();
    }
    // map が変化したときのみ出力する (M3U8 仕様上 unset 不可のため None → Some のみ想定)
    if segment.map != *current_map {
        if let Some(map) = &segment.map {
            out.push_str(&write_map(map));
        }
        *current_map = segment.map.clone();
    }
    if let Some(pdt) = &segment.program_date_time {
        out.push_str(&format!("#EXT-X-PROGRAM-DATE-TIME:{pdt}\n"));
    }
    for dr in &segment.date_ranges {
        out.push_str(&write_date_range(dr));
    }
    if segment.gap {
        out.push_str("#EXT-X-GAP\n");
    }
    // bitrate が変化したときのみ出力する。None への遷移は 0 で reset する
    if segment.bitrate != *current_bitrate {
        match segment.bitrate {
            Some(b) => out.push_str(&format!("#EXT-X-BITRATE:{b}\n")),
            None => out.push_str("#EXT-X-BITRATE:0\n"),
        }
        *current_bitrate = segment.bitrate;
    }
    for part in &segment.parts {
        out.push_str(&write_part(part));
    }
    if let Some(br) = &segment.byte_range {
        out.push_str(&format!("#EXT-X-BYTERANGE:{}\n", write_byterange(br)));
    }
    match &segment.title {
        Some(title) => out.push_str(&format!("#EXTINF:{:.5},{title}\n", segment.duration)),
        None => out.push_str(&format!("#EXTINF:{:.5},\n", segment.duration)),
    }
    out.push_str(&format!("{}\n", segment.uri));

    out
}

fn write_byterange(br: &ByteRange) -> String {
    match br.offset {
        Some(offset) => format!("{}@{}", br.length, offset),
        None => format!("{}", br.length),
    }
}

fn write_map(map: &Map) -> String {
    let mut attrs = format!("URI={}", write_quoted_string(&map.uri));
    if let Some(br) = &map.byte_range {
        attrs.push_str(&format!(
            ",BYTERANGE={}",
            write_quoted_string(&write_byterange(br))
        ));
    }
    format!("#EXT-X-MAP:{attrs}\n")
}

fn write_date_range(dr: &DateRange) -> String {
    let mut attrs = format!(
        "ID={},START-DATE={}",
        write_quoted_string(&dr.id),
        write_quoted_string(&dr.start_date)
    );
    if let Some(v) = &dr.class {
        attrs.push_str(&format!(",CLASS={}", write_quoted_string(v)));
    }
    if let Some(v) = &dr.end_date {
        attrs.push_str(&format!(",END-DATE={}", write_quoted_string(v)));
    }
    if let Some(v) = dr.duration {
        attrs.push_str(&format!(",DURATION={v:.5}"));
    }
    if let Some(v) = dr.planned_duration {
        attrs.push_str(&format!(",PLANNED-DURATION={v:.5}"));
    }
    if dr.end_on_next {
        attrs.push_str(",END-ON-NEXT=YES");
    }
    for attribute in &dr.extra_attributes {
        attrs.push(',');
        attrs.push_str(&attribute.name);
        attrs.push('=');
        if attribute.quoted {
            attrs.push_str(&write_quoted_string(&attribute.value));
        } else {
            attrs.push_str(&attribute.value);
        }
    }
    format!("#EXT-X-DATERANGE:{attrs}\n")
}

fn write_server_control(sc: &ServerControl) -> String {
    let mut attrs = String::new();
    if let Some(v) = sc.can_skip_until {
        attrs.push_str(&format!("CAN-SKIP-UNTIL={v:.5}"));
    }
    if sc.can_skip_dateranges {
        if !attrs.is_empty() {
            attrs.push(',');
        }
        attrs.push_str("CAN-SKIP-DATERANGES=YES");
    }
    if let Some(v) = sc.hold_back {
        if !attrs.is_empty() {
            attrs.push(',');
        }
        attrs.push_str(&format!("HOLD-BACK={v:.5}"));
    }
    if let Some(v) = sc.part_hold_back {
        if !attrs.is_empty() {
            attrs.push(',');
        }
        attrs.push_str(&format!("PART-HOLD-BACK={v:.5}"));
    }
    if sc.can_block_reload {
        if !attrs.is_empty() {
            attrs.push(',');
        }
        attrs.push_str("CAN-BLOCK-RELOAD=YES");
    }
    format!("#EXT-X-SERVER-CONTROL:{attrs}\n")
}

fn write_part_inf(pi: &PartInf) -> String {
    format!("#EXT-X-PART-INF:PART-TARGET={:.5}\n", pi.part_target)
}

// この機能は draft-pantos-hls-rfc8216bis-20.txt 4.4.5.2 由来。
// 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
fn write_skip(skip: &Skip) -> String {
    let mut attrs = format!("SKIPPED-SEGMENTS={}", skip.skipped_segments);
    if !skip.recently_removed_dateranges.is_empty() {
        attrs.push_str(&format!(
            ",RECENTLY-REMOVED-DATERANGES=\"{}\"",
            skip.recently_removed_dateranges.join("\t")
        ));
    }
    format!("#EXT-X-SKIP:{attrs}\n")
}

fn write_part(part: &Part) -> String {
    let mut attrs = format!(
        "URI={},DURATION={:.5}",
        write_quoted_string(&part.uri),
        part.duration
    );
    if part.independent {
        attrs.push_str(",INDEPENDENT=YES");
    }
    if let Some(br) = &part.byte_range {
        attrs.push_str(&format!(
            ",BYTERANGE={}",
            write_quoted_string(&write_byterange(br))
        ));
    }
    if part.gap {
        attrs.push_str(",GAP=YES");
    }
    format!("#EXT-X-PART:{attrs}\n")
}

fn write_preload_hint(hint: &PreloadHint) -> String {
    let type_str = match hint.hint_type {
        PreloadHintType::Part => "PART",
        PreloadHintType::Map => "MAP",
    };
    let mut attrs = format!("TYPE={type_str},URI={}", write_quoted_string(&hint.uri));
    if let Some(v) = hint.byterange_start {
        attrs.push_str(&format!(",BYTERANGE-START={v}"));
    }
    if let Some(v) = hint.byterange_length {
        attrs.push_str(&format!(",BYTERANGE-LENGTH={v}"));
    }
    format!("#EXT-X-PRELOAD-HINT:{attrs}\n")
}

fn write_rendition_report(report: &RenditionReport) -> String {
    let mut attrs = format!("URI={}", write_quoted_string(&report.uri));
    if let Some(v) = report.last_msn {
        attrs.push_str(&format!(",LAST-MSN={v}"));
    }
    if let Some(v) = report.last_part {
        attrs.push_str(&format!(",LAST-PART={v}"));
    }
    format!("#EXT-X-RENDITION-REPORT:{attrs}\n")
}

fn write_variable_definition(definition: &VariableDefinition) -> String {
    match definition {
        VariableDefinition::Name { name, value } => {
            format!(
                "#EXT-X-DEFINE:NAME={},VALUE={}\n",
                write_quoted_string(name),
                write_quoted_string(value)
            )
        }
        VariableDefinition::Import { name, .. } => {
            format!("#EXT-X-DEFINE:IMPORT={}\n", write_quoted_string(name))
        }
        VariableDefinition::QueryParam { name, .. } => {
            format!("#EXT-X-DEFINE:QUERYPARAM={}\n", write_quoted_string(name))
        }
    }
}
