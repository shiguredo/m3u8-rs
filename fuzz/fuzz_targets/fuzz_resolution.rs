#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // 任意の解像度文字列を #EXT-X-STREAM-INF の RESOLUTION に埋め込み、公開 API 経由でパースする
        let input = format!(
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000,RESOLUTION={s}\nindex.m3u8\n"
        );
        let _ = shiguredo_m3u8::parse_multivariant_playlist(&input);
    }
});
