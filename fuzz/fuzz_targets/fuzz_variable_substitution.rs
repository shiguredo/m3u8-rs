#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // 任意の文字列をセグメント URI に埋め込み、変数展開を公開 API 経由で行う
        let input = format!(
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-DEFINE:NAME=\"a\",VALUE=\"x\"\n#EXT-X-DEFINE:NAME=\"b\",IMPORT=a\n#EXTINF:10,\n{s}\n#EXT-X-ENDLIST\n"
        );
        let _ = shiguredo_m3u8::parse_media_playlist(&input);
    }
});
