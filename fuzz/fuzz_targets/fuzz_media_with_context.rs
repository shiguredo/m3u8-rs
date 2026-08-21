#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // URI 付きでパースする
        if let Some((uri, body)) = s.split_once('\n') {
            let _ = shiguredo_m3u8::parse_media_playlist_with_context(body, Some(uri), None);
        }

        // Multivariant Playlist を先にパースし、その結果を文脈として Media Playlist をパースする
        if let Some((multivariant_input, media_input)) = s.split_once("\n#EXTM3U\n")
            && let Ok(mv) = shiguredo_m3u8::parse_multivariant_playlist(multivariant_input)
        {
            let _ =
                shiguredo_m3u8::parse_media_playlist_with_context(media_input, None, Some(&mv));
        }
    }
});
