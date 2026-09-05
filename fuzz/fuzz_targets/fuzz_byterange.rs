#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // 任意のバイトレンジ文字列を #EXT-X-BYTERANGE に埋め込み、公開 API 経由でパースする
        let input = format!(
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-BYTERANGE:{s}\n#EXT-X-ENDLIST\n"
        );
        let _ = shiguredo_m3u8::parse_media_playlist(&input);
    }
});
