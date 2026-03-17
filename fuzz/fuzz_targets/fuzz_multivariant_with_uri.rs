#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // 入力の先頭行を URI、残りをプレイリスト本体として使う
        if let Some((uri, body)) = s.split_once('\n') {
            let _ = shiguredo_m3u8::parse_multivariant_playlist_with_uri(body, uri);
        }
    }
});
