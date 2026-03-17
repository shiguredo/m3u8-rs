use crate::variable::VariableDefinition;

/// Multivariant Playlist (Master Playlist) の全体
#[derive(Debug, Clone, PartialEq)]
pub struct MultivariantPlaylist {
    /// `#EXT-X-VERSION`
    pub version: Option<u8>,
    /// `#EXT-X-INDEPENDENT-SEGMENTS`
    pub independent_segments: bool,
    /// `#EXT-X-START`
    pub start: Option<StartPoint>,
    /// `#EXT-X-DEFINE`
    pub variable_definitions: Vec<VariableDefinition>,
    /// `#EXT-X-CONTENT-STEERING`
    ///
    /// draft-pantos-hls-rfc8216bis-20.txt 4.4.6.6 由来。
    /// 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
    pub content_steering: Option<ContentSteering>,
    /// `#EXT-X-STREAM-INF` + URI のリスト
    pub variant_streams: Vec<VariantStream>,
    /// `#EXT-X-MEDIA` のリスト
    pub renditions: Vec<Rendition>,
    /// `#EXT-X-I-FRAME-STREAM-INF` のリスト
    pub i_frame_streams: Vec<IFrameStream>,
    /// `#EXT-X-SESSION-DATA` のリスト
    pub session_data: Vec<SessionData>,
    /// `#EXT-X-SESSION-KEY` のリスト
    pub session_keys: Vec<Key>,
}

/// `#EXT-X-STREAM-INF` + URI
#[derive(Debug, Clone, PartialEq)]
pub struct VariantStream {
    /// `BANDWIDTH` (必須)
    pub bandwidth: u64,
    /// `AVERAGE-BANDWIDTH`
    pub average_bandwidth: Option<u64>,
    /// `CODECS`
    pub codecs: Option<String>,
    /// `SUPPLEMENTAL-CODECS`
    ///
    /// draft-pantos-hls-rfc8216bis-20.txt 4.4.6.2 由来。
    /// 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
    pub supplemental_codecs: Option<String>,
    /// `RESOLUTION`
    pub resolution: Option<Resolution>,
    /// `FRAME-RATE`
    pub frame_rate: Option<f64>,
    /// `HDCP-LEVEL`
    pub hdcp_level: Option<HdcpLevel>,
    /// `ALLOWED-CPC`
    pub allowed_cpc: Option<String>,
    /// `VIDEO-RANGE`
    pub video_range: Option<VideoRange>,
    /// `AUDIO` — `#EXT-X-MEDIA` の GROUP-ID への参照
    pub audio: Option<String>,
    /// `VIDEO` — `#EXT-X-MEDIA` の GROUP-ID への参照
    pub video: Option<String>,
    /// `SUBTITLES` — `#EXT-X-MEDIA` の GROUP-ID への参照
    pub subtitles: Option<String>,
    /// `CLOSED-CAPTIONS` — `#EXT-X-MEDIA` の GROUP-ID への参照
    pub closed_captions: Option<ClosedCaptions>,
    /// `NAME`
    pub name: Option<String>,
    /// `STABLE-VARIANT-ID`
    pub stable_variant_id: Option<String>,
    /// `PATHWAY-ID`
    ///
    /// draft-pantos-hls-rfc8216bis-20.txt 4.4.6.2 由来。
    /// 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
    pub pathway_id: Option<String>,
    /// プレイリスト URI
    pub uri: String,
}

/// `#EXT-X-MEDIA`
#[derive(Debug, Clone, PartialEq)]
pub struct Rendition {
    /// `TYPE` (必須)
    pub media_type: MediaType,
    /// `GROUP-ID` (必須)
    pub group_id: String,
    /// `NAME` (必須)
    pub name: String,
    /// `URI`
    pub uri: Option<String>,
    /// `LANGUAGE`
    pub language: Option<String>,
    /// `ASSOC-LANGUAGE`
    pub assoc_language: Option<String>,
    /// `DEFAULT`
    pub default: bool,
    /// `AUTOSELECT`
    pub autoselect: bool,
    /// `FORCED`
    pub forced: bool,
    /// `INSTREAM-ID`
    pub instream_id: Option<String>,
    /// `CHARACTERISTICS`
    pub characteristics: Option<String>,
    /// `CHANNELS`
    pub channels: Option<String>,
    /// `BIT-DEPTH`
    pub bit_depth: Option<u64>,
    /// `SAMPLE-RATE`
    pub sample_rate: Option<u64>,
    /// `STABLE-RENDITION-ID`
    pub stable_rendition_id: Option<String>,
}

/// `#EXT-X-I-FRAME-STREAM-INF`
#[derive(Debug, Clone, PartialEq)]
pub struct IFrameStream {
    /// `BANDWIDTH` (必須)
    pub bandwidth: u64,
    /// `AVERAGE-BANDWIDTH`
    pub average_bandwidth: Option<u64>,
    /// `CODECS`
    pub codecs: Option<String>,
    /// `SUPPLEMENTAL-CODECS`
    ///
    /// draft-pantos-hls-rfc8216bis-20.txt 4.4.6.3 由来。
    /// 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
    pub supplemental_codecs: Option<String>,
    /// `RESOLUTION`
    pub resolution: Option<Resolution>,
    /// `HDCP-LEVEL`
    pub hdcp_level: Option<HdcpLevel>,
    /// `VIDEO-RANGE`
    pub video_range: Option<VideoRange>,
    /// `VIDEO`
    pub video: Option<String>,
    /// `PATHWAY-ID`
    ///
    /// draft-pantos-hls-rfc8216bis-20.txt 4.4.6.3 由来。
    /// 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
    pub pathway_id: Option<String>,
    /// `URI` (必須)
    pub uri: String,
}

/// `#EXT-X-CONTENT-STEERING`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentSteering {
    /// `SERVER-URI` (必須)
    pub server_uri: String,
    /// `PATHWAY-ID`
    pub pathway_id: Option<String>,
}

/// `#EXT-X-SESSION-DATA`
#[derive(Debug, Clone, PartialEq)]
pub struct SessionData {
    /// `DATA-ID` (必須)
    pub data_id: String,
    /// `VALUE` または `URI` のいずれか
    pub value: SessionDataValue,
    /// `LANGUAGE`
    pub language: Option<String>,
}

/// SESSION-DATA の値の種類
#[derive(Debug, Clone, PartialEq)]
pub enum SessionDataValue {
    /// `VALUE`
    Value(String),
    /// `URI`
    Uri(String),
}

/// `#EXT-X-KEY` / `#EXT-X-SESSION-KEY`
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    /// `METHOD` (必須)
    pub method: EncryptionMethod,
    /// `URI`
    pub uri: Option<String>,
    /// `IV`
    pub iv: Option<String>,
    /// `KEYFORMAT`
    pub keyformat: Option<String>,
    /// `KEYFORMATVERSIONS`
    pub keyformat_versions: Option<String>,
}

/// `#EXT-X-START`
#[derive(Debug, Clone, PartialEq)]
pub struct StartPoint {
    /// `TIME-OFFSET` (必須)
    pub time_offset: f64,
    /// `PRECISE`
    pub precise: bool,
}

/// `RESOLUTION`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

/// `TYPE` 属性の値
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MediaType {
    Audio,
    Video,
    Subtitles,
    ClosedCaptions,
}

/// `CLOSED-CAPTIONS` 属性の値
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClosedCaptions {
    /// GROUP-ID への参照
    GroupId(String),
    /// `NONE`
    None,
}

/// `HDCP-LEVEL` 属性の値
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HdcpLevel {
    None,
    Type0,
    Type1,
}

/// `VIDEO-RANGE` 属性の値
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VideoRange {
    Sdr,
    Hlg,
    Pq,
}

/// `METHOD` 属性の値
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EncryptionMethod {
    None,
    Aes128,
    // draft-pantos-hls-rfc8216bis-20.txt 4.4.4.4 由来。
    // 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
    Aes256Gcm,
    SampleAes,
    // draft-pantos-hls-rfc8216bis-20.txt 4.4.4.4 由来。
    // 最終 RFC で変更される可能性があるため、仕様変更時は追従が必要。
    SampleAesCtr,
}
