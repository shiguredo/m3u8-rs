//! M3U8 パーサーとビルダーを提供するクレート

mod attribute;
mod builder;
pub mod error;
pub mod media;
pub mod multivariant;
mod parser;
pub mod variable;

use error::Result;

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
