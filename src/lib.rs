mod attribute;
mod builder;
mod error;
pub mod media;
pub mod multivariant;
mod parser;
pub mod variable;

pub use error::{Error, ErrorKind, Result};

/// fuzzing 用に内部パーサ関数を公開するモジュール
#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzz_helpers {
    use crate::error::Result;
    use std::collections::HashMap;

    /// `attribute::parse_attribute_entries()` のラッパー
    pub fn parse_attribute_entries(attrs: &str) -> Result<()> {
        let _ = crate::attribute::parse_attribute_entries(attrs)?;
        Ok(())
    }

    /// `attribute::parse_resolution()` のラッパー
    pub fn parse_resolution(s: &str) -> Result<crate::multivariant::Resolution> {
        crate::attribute::parse_resolution(s)
    }

    /// `attribute::parse_byterange()` のラッパー
    pub fn parse_byterange(s: &str) -> Result<crate::media::ByteRange> {
        crate::attribute::parse_byterange(s)
    }

    /// `parser::multivariant::substitute_variables_in_line()` のラッパー
    pub fn substitute_variables_in_line(
        line: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String> {
        crate::parser::multivariant::substitute_variables_in_line(line, variables)
    }
}

/// Multivariant Playlist (Master Playlist) をパースする
pub fn parse_multivariant_playlist(input: &str) -> Result<multivariant::MultivariantPlaylist> {
    parser::multivariant::parse(input, None)
}

/// URI を使って Multivariant Playlist をパースする
pub fn parse_multivariant_playlist_with_uri(
    input: &str,
    playlist_uri: &str,
) -> Result<multivariant::MultivariantPlaylist> {
    parser::multivariant::parse(input, Some(playlist_uri))
}

/// Media Playlist をパースする
pub fn parse_media_playlist(input: &str) -> Result<media::MediaPlaylist> {
    parser::media::parse(input, None, None)
}

/// 文脈付きで Media Playlist をパースする
pub fn parse_media_playlist_with_context(
    input: &str,
    playlist_uri: Option<&str>,
    multivariant_playlist: Option<&multivariant::MultivariantPlaylist>,
) -> Result<media::MediaPlaylist> {
    parser::media::parse(input, playlist_uri, multivariant_playlist)
}

/// Multivariant Playlist を M3U8 テキストに書き出す
pub fn write_multivariant_playlist(playlist: &multivariant::MultivariantPlaylist) -> String {
    builder::multivariant::write(playlist)
}

/// Media Playlist を M3U8 テキストに書き出す
pub fn write_media_playlist(playlist: &media::MediaPlaylist) -> String {
    builder::media::write(playlist)
}
