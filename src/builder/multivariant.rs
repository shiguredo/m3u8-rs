//! Multivariant Playlist の書き出し処理を提供する

use crate::variable::VariableDefinition;
use crate::{
    attribute::write_quoted_string,
    multivariant::{
        ClosedCaptions, ContentSteering, EncryptionMethod, HdcpLevel, IFrameStream, Key, MediaType,
        MultivariantPlaylist, Rendition, SessionData, SessionDataValue, StartPoint, VariantStream,
        VideoRange,
    },
};

/// Multivariant Playlist を M3U8 テキストに書き出す
pub fn write(playlist: &MultivariantPlaylist) -> String {
    let mut out = String::from("#EXTM3U\n");

    if let Some(v) = playlist.version {
        out.push_str(&format!("#EXT-X-VERSION:{v}\n"));
    }
    if playlist.independent_segments {
        out.push_str("#EXT-X-INDEPENDENT-SEGMENTS\n");
    }
    if let Some(start) = &playlist.start {
        out.push_str(&write_start(start));
    }
    for definition in &playlist.variable_definitions {
        out.push_str(&write_variable_definition(definition));
    }
    if let Some(content_steering) = &playlist.content_steering {
        out.push_str(&write_content_steering(content_steering));
    }

    for rendition in &playlist.renditions {
        out.push_str(&write_rendition(rendition));
    }
    for stream in &playlist.variant_streams {
        out.push_str(&write_variant_stream(stream));
    }
    for i_frame in &playlist.i_frame_streams {
        out.push_str(&write_i_frame_stream(i_frame));
    }
    for session in &playlist.session_data {
        out.push_str(&write_session_data(session));
    }
    for key in &playlist.session_keys {
        out.push_str(&format!("#EXT-X-SESSION-KEY:{}\n", write_key_attrs(key)));
    }

    out
}

fn write_start(start: &StartPoint) -> String {
    format!("#EXT-X-START:{}\n", write_start_attrs(start))
}

pub(crate) fn write_start_attrs(start: &StartPoint) -> String {
    let mut attrs = format!("TIME-OFFSET={:.5}", start.time_offset);
    if start.precise {
        attrs.push_str(",PRECISE=YES");
    }
    attrs
}

fn write_variant_stream(stream: &VariantStream) -> String {
    let mut attrs = format!("BANDWIDTH={}", stream.bandwidth);
    if let Some(v) = stream.average_bandwidth {
        attrs.push_str(&format!(",AVERAGE-BANDWIDTH={v}"));
    }
    if let Some(v) = &stream.codecs {
        attrs.push_str(&format!(",CODECS={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.supplemental_codecs {
        attrs.push_str(&format!(",SUPPLEMENTAL-CODECS={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.resolution {
        attrs.push_str(&format!(",RESOLUTION={}x{}", v.width, v.height));
    }
    if let Some(v) = stream.frame_rate {
        attrs.push_str(&format!(",FRAME-RATE={v:.5}"));
    }
    if let Some(v) = &stream.hdcp_level {
        let s = match v {
            HdcpLevel::None => "NONE",
            HdcpLevel::Type0 => "TYPE-0",
            HdcpLevel::Type1 => "TYPE-1",
        };
        attrs.push_str(&format!(",HDCP-LEVEL={s}"));
    }
    if let Some(v) = &stream.allowed_cpc {
        attrs.push_str(&format!(",ALLOWED-CPC={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.video_range {
        let s = match v {
            VideoRange::Sdr => "SDR",
            VideoRange::Hlg => "HLG",
            VideoRange::Pq => "PQ",
        };
        attrs.push_str(&format!(",VIDEO-RANGE={s}"));
    }
    if let Some(v) = &stream.audio {
        attrs.push_str(&format!(",AUDIO={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.video {
        attrs.push_str(&format!(",VIDEO={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.subtitles {
        attrs.push_str(&format!(",SUBTITLES={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.closed_captions {
        match v {
            ClosedCaptions::None => attrs.push_str(",CLOSED-CAPTIONS=NONE"),
            ClosedCaptions::GroupId(g) => {
                attrs.push_str(&format!(",CLOSED-CAPTIONS={}", write_quoted_string(g)));
            }
        }
    }
    if let Some(v) = &stream.name {
        attrs.push_str(&format!(",NAME={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.stable_variant_id {
        attrs.push_str(&format!(",STABLE-VARIANT-ID={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.pathway_id {
        attrs.push_str(&format!(",PATHWAY-ID={}", write_quoted_string(v)));
    }
    format!("#EXT-X-STREAM-INF:{attrs}\n{}\n", stream.uri)
}

fn write_rendition(rendition: &Rendition) -> String {
    let type_str = match rendition.media_type {
        MediaType::Audio => "AUDIO",
        MediaType::Video => "VIDEO",
        MediaType::Subtitles => "SUBTITLES",
        MediaType::ClosedCaptions => "CLOSED-CAPTIONS",
    };
    let mut attrs = format!(
        "TYPE={},GROUP-ID={},NAME={}",
        type_str,
        write_quoted_string(&rendition.group_id),
        write_quoted_string(&rendition.name)
    );
    if let Some(v) = &rendition.uri {
        attrs.push_str(&format!(",URI={}", write_quoted_string(v)));
    }
    if let Some(v) = &rendition.language {
        attrs.push_str(&format!(",LANGUAGE={}", write_quoted_string(v)));
    }
    if let Some(v) = &rendition.assoc_language {
        attrs.push_str(&format!(",ASSOC-LANGUAGE={}", write_quoted_string(v)));
    }
    if rendition.default {
        attrs.push_str(",DEFAULT=YES");
    }
    if rendition.autoselect {
        attrs.push_str(",AUTOSELECT=YES");
    }
    if rendition.forced {
        attrs.push_str(",FORCED=YES");
    }
    if let Some(v) = &rendition.instream_id {
        attrs.push_str(&format!(",INSTREAM-ID={}", write_quoted_string(v)));
    }
    if let Some(v) = &rendition.characteristics {
        attrs.push_str(&format!(",CHARACTERISTICS={}", write_quoted_string(v)));
    }
    if let Some(v) = &rendition.channels {
        attrs.push_str(&format!(",CHANNELS={}", write_quoted_string(v)));
    }
    if let Some(v) = rendition.bit_depth {
        attrs.push_str(&format!(",BIT-DEPTH={v}"));
    }
    if let Some(v) = rendition.sample_rate {
        attrs.push_str(&format!(",SAMPLE-RATE={v}"));
    }
    if let Some(v) = &rendition.stable_rendition_id {
        attrs.push_str(&format!(",STABLE-RENDITION-ID={}", write_quoted_string(v)));
    }
    format!("#EXT-X-MEDIA:{attrs}\n")
}

fn write_i_frame_stream(stream: &IFrameStream) -> String {
    let mut attrs = format!("BANDWIDTH={}", stream.bandwidth);
    if let Some(v) = stream.average_bandwidth {
        attrs.push_str(&format!(",AVERAGE-BANDWIDTH={v}"));
    }
    if let Some(v) = &stream.codecs {
        attrs.push_str(&format!(",CODECS={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.supplemental_codecs {
        attrs.push_str(&format!(",SUPPLEMENTAL-CODECS={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.resolution {
        attrs.push_str(&format!(",RESOLUTION={}x{}", v.width, v.height));
    }
    if let Some(v) = &stream.hdcp_level {
        let s = match v {
            HdcpLevel::None => "NONE",
            HdcpLevel::Type0 => "TYPE-0",
            HdcpLevel::Type1 => "TYPE-1",
        };
        attrs.push_str(&format!(",HDCP-LEVEL={s}"));
    }
    if let Some(v) = &stream.video_range {
        let s = match v {
            VideoRange::Sdr => "SDR",
            VideoRange::Hlg => "HLG",
            VideoRange::Pq => "PQ",
        };
        attrs.push_str(&format!(",VIDEO-RANGE={s}"));
    }
    if let Some(v) = &stream.video {
        attrs.push_str(&format!(",VIDEO={}", write_quoted_string(v)));
    }
    if let Some(v) = &stream.pathway_id {
        attrs.push_str(&format!(",PATHWAY-ID={}", write_quoted_string(v)));
    }
    attrs.push_str(&format!(",URI={}", write_quoted_string(&stream.uri)));
    format!("#EXT-X-I-FRAME-STREAM-INF:{attrs}\n")
}

fn write_content_steering(content_steering: &ContentSteering) -> String {
    let mut attrs = format!(
        "SERVER-URI={}",
        write_quoted_string(&content_steering.server_uri)
    );
    if let Some(v) = &content_steering.pathway_id {
        attrs.push_str(&format!(",PATHWAY-ID={}", write_quoted_string(v)));
    }
    format!("#EXT-X-CONTENT-STEERING:{attrs}\n")
}

fn write_session_data(session: &SessionData) -> String {
    let mut attrs = format!("DATA-ID={}", write_quoted_string(&session.data_id));
    match &session.value {
        SessionDataValue::Value(v) => attrs.push_str(&format!(",VALUE={}", write_quoted_string(v))),
        SessionDataValue::Uri(u) => attrs.push_str(&format!(",URI={}", write_quoted_string(u))),
    }
    if let Some(v) = &session.language {
        attrs.push_str(&format!(",LANGUAGE={}", write_quoted_string(v)));
    }
    format!("#EXT-X-SESSION-DATA:{attrs}\n")
}

pub(crate) fn write_key_attrs(key: &Key) -> String {
    let method = match key.method {
        EncryptionMethod::None => "NONE",
        EncryptionMethod::Aes128 => "AES-128",
        EncryptionMethod::Aes256Gcm => "AES-256-GCM",
        EncryptionMethod::SampleAes => "SAMPLE-AES",
        EncryptionMethod::SampleAesCtr => "SAMPLE-AES-CTR",
    };
    let mut attrs = format!("METHOD={method}");
    if let Some(v) = &key.uri {
        attrs.push_str(&format!(",URI={}", write_quoted_string(v)));
    }
    if let Some(v) = &key.iv {
        attrs.push_str(&format!(",IV={v}"));
    }
    if let Some(v) = &key.keyformat {
        attrs.push_str(&format!(",KEYFORMAT={}", write_quoted_string(v)));
    }
    if let Some(v) = &key.keyformat_versions {
        attrs.push_str(&format!(",KEYFORMATVERSIONS={}", write_quoted_string(v)));
    }
    attrs
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
