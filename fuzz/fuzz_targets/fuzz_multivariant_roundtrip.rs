#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(playlist) = shiguredo_m3u8::parse_multivariant_playlist(s)
    {
        // builder がパニックしないことを検証する
        let output = shiguredo_m3u8::write_multivariant_playlist(&playlist);
        // 書き出した結果を再パースしてパニックしないことを検証する
        let _ = shiguredo_m3u8::parse_multivariant_playlist(&output);
    }
});
