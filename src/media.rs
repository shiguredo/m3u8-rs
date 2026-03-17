use crate::multivariant::{Key, StartPoint};
use crate::variable::VariableDefinition;

/// Media Playlist の全体
#[derive(Debug, Clone, PartialEq)]
pub struct MediaPlaylist {
    /// `#EXT-X-VERSION`
    pub version: Option<u8>,
    /// `#EXT-X-TARGETDURATION` (必須、値は 1 以上)
    pub target_duration: u32,
    /// `#EXT-X-MEDIA-SEQUENCE` (省略時はデフォルト 0)
    pub media_sequence: Option<u64>,
    /// `#EXT-X-DISCONTINUITY-SEQUENCE` (省略時はデフォルト 0)
    pub discontinuity_sequence: Option<u64>,
    /// `#EXT-X-PLAYLIST-TYPE`
    pub playlist_type: Option<PlaylistType>,
    /// `#EXT-X-I-FRAMES-ONLY`
    pub i_frames_only: bool,
    /// `#EXT-X-INDEPENDENT-SEGMENTS`
    pub independent_segments: bool,
    /// `#EXT-X-START`
    pub start: Option<StartPoint>,
    /// `#EXT-X-SERVER-CONTROL` (LL-HLS)
    pub server_control: Option<ServerControl>,
    /// `#EXT-X-PART-INF` (LL-HLS)
    pub part_inf: Option<PartInf>,
    /// `#EXT-X-DEFINE`
    pub variable_definitions: Vec<VariableDefinition>,
    /// セグメントのリスト
    pub segments: Vec<Segment>,
    /// `#EXT-X-SKIP` (LL-HLS Delta Update)
    pub skip: Option<Skip>,
    /// プレイリスト末尾の `#EXT-X-PRELOAD-HINT` (LL-HLS)
    pub preload_hints: Vec<PreloadHint>,
    /// プレイリスト末尾の `#EXT-X-RENDITION-REPORT` (LL-HLS)
    pub rendition_reports: Vec<RenditionReport>,
    /// `#EXT-X-ENDLIST`
    pub end_list: bool,
}

/// `#EXT-X-PLAYLIST-TYPE`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistType {
    /// `EVENT`
    Event,
    /// `VOD`
    Vod,
}

/// `#EXT-X-SERVER-CONTROL` (LL-HLS)
#[derive(Debug, Clone, PartialEq)]
pub struct ServerControl {
    /// `CAN-SKIP-UNTIL`
    pub can_skip_until: Option<f64>,
    /// `CAN-SKIP-DATERANGES`
    pub can_skip_dateranges: bool,
    /// `HOLD-BACK`
    pub hold_back: Option<f64>,
    /// `PART-HOLD-BACK`
    pub part_hold_back: Option<f64>,
    /// `CAN-BLOCK-RELOAD`
    pub can_block_reload: bool,
}

/// `#EXT-X-PART-INF` (LL-HLS)
#[derive(Debug, Clone, PartialEq)]
pub struct PartInf {
    /// `PART-TARGET` (必須)
    pub part_target: f64,
}

/// `#EXT-X-SKIP` (LL-HLS Delta Update)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    /// `SKIPPED-SEGMENTS` (必須)
    pub skipped_segments: u64,
    /// `RECENTLY-REMOVED-DATERANGES`
    pub recently_removed_dateranges: Vec<String>,
}

/// セグメント (`#EXTINF` + URI + 関連タグのまとまり)
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    /// `#EXTINF` の duration (必須)
    pub duration: f64,
    /// `#EXTINF` の title
    pub title: Option<String>,
    /// セグメント URI (必須)
    pub uri: String,
    /// `#EXT-X-BYTERANGE`
    pub byte_range: Option<ByteRange>,
    /// `#EXT-X-DISCONTINUITY`
    pub discontinuity: bool,
    /// `#EXT-X-KEY`
    pub key: Option<Key>,
    /// `#EXT-X-MAP`
    pub map: Option<Map>,
    /// `#EXT-X-PROGRAM-DATE-TIME`
    pub program_date_time: Option<String>,
    /// `#EXT-X-DATERANGE`
    pub date_ranges: Vec<DateRange>,
    /// `#EXT-X-GAP`
    pub gap: bool,
    /// `#EXT-X-BITRATE`
    pub bitrate: Option<u64>,
    /// `#EXT-X-PART` のリスト (LL-HLS)
    pub parts: Vec<Part>,
}

/// `#EXT-X-BYTERANGE`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    /// バイト数
    pub length: u64,
    /// オフセット (省略時は前セグメントの末尾)
    pub offset: Option<u64>,
}

/// `#EXT-X-MAP`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    /// `URI` (必須)
    pub uri: String,
    /// `BYTERANGE`
    pub byte_range: Option<ByteRange>,
}

/// `#EXT-X-DATERANGE`
#[derive(Debug, Clone, PartialEq)]
pub struct DateRange {
    /// `ID` (必須)
    pub id: String,
    /// `CLASS`
    pub class: Option<String>,
    /// `START-DATE` (必須)
    pub start_date: String,
    /// `END-DATE`
    pub end_date: Option<String>,
    /// `DURATION`
    pub duration: Option<f64>,
    /// `PLANNED-DURATION`
    pub planned_duration: Option<f64>,
    /// `END-ON-NEXT`
    pub end_on_next: bool,
    /// `X-*` / `SCTE35-*` / bis 拡張属性
    pub extra_attributes: Vec<DateRangeAttribute>,
}

/// `#EXT-X-DATERANGE` の拡張属性
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateRangeAttribute {
    /// 属性名
    pub name: String,
    /// 属性値
    pub value: String,
    /// quoted-string だったかどうか
    pub quoted: bool,
}

/// `#EXT-X-PART` (LL-HLS)
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// `URI` (必須)
    pub uri: String,
    /// `DURATION` (必須)
    pub duration: f64,
    /// `INDEPENDENT`
    pub independent: bool,
    /// `BYTERANGE`
    pub byte_range: Option<ByteRange>,
    /// `GAP`
    pub gap: bool,
}

/// `#EXT-X-PRELOAD-HINT` (LL-HLS)
#[derive(Debug, Clone, PartialEq)]
pub struct PreloadHint {
    /// `TYPE` (必須)
    pub hint_type: PreloadHintType,
    /// `URI` (必須)
    pub uri: String,
    /// `BYTERANGE-START`
    pub byterange_start: Option<u64>,
    /// `BYTERANGE-LENGTH`
    pub byterange_length: Option<u64>,
}

/// `#EXT-X-PRELOAD-HINT` の `TYPE`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreloadHintType {
    Part,
    Map,
}

/// `#EXT-X-RENDITION-REPORT` (LL-HLS)
#[derive(Debug, Clone, PartialEq)]
pub struct RenditionReport {
    /// `URI` (必須)
    pub uri: String,
    /// `LAST-MSN`
    pub last_msn: Option<u64>,
    /// `LAST-PART`
    pub last_part: Option<u64>,
}
