# m3u8-rs

[![crates.io](https://img.shields.io/crates/v/shiguredo_m3u8.svg)](https://crates.io/crates/shiguredo_m3u8)
[![docs.rs](https://docs.rs/shiguredo_m3u8/badge.svg)](https://docs.rs/shiguredo_m3u8)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![GitHub Actions](https://github.com/shiguredo/m3u8-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/shiguredo/m3u8-rs/actions/workflows/ci.yml)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?logo=discord&logoColor=white)](https://discord.gg/shiguredo)

## About Shiguredo's open source software

We will not respond to PRs or issues that have not been discussed on Discord. Also, Discord is only available in Japanese.

Please read <https://github.com/shiguredo/oss> before use.

## 時雨堂のオープンソースソフトウェアについて

利用前に <https://github.com/shiguredo/oss> をお読みください。

## 概要

Rust で実装された依存 0 の M3U8 パーサー + ビルダーライブラリです。HLS (HTTP Live Streaming) の Multivariant Playlist と Media Playlist の両方に対応しています。

## 特徴

- 依存ライブラリ 0
- Multivariant Playlist と Media Playlist の両方をパース・ビルド可能
- RFC 8216 (HLS) 準拠の厳密なバリデーション
- HLS 2nd Edition (draft-pantos-hls-rfc8216bis) の新規タグに対応
- LL-HLS (Low Latency HLS) タグに対応
- `EXT-X-DEFINE` による変数置換に対応

## 使い方

### Multivariant Playlist のパース

```rust
use shiguredo_m3u8::parse_multivariant_playlist;

let input = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000000\nlow.m3u8\n";
let playlist = parse_multivariant_playlist(input)?;
```

### URI 付き Multivariant Playlist のパース

URI を指定すると、`EXT-X-DEFINE` の `IMPORT` による変数インポートなどで利用されます。

```rust
use shiguredo_m3u8::parse_multivariant_playlist_with_uri;

let playlist = parse_multivariant_playlist_with_uri(input, "http://example.com/master.m3u8")?;
```

### Media Playlist のパース

```rust
use shiguredo_m3u8::parse_media_playlist;

let input = "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXTINF:10.0,\nsegment.ts\n";
let playlist = parse_media_playlist(input)?;
```

### 文脈付きパース

Media Playlist を Multivariant Playlist の情報と URI を使ってパースできます。Multivariant Playlist の情報があると、rendition group の参照整合性などの追加検証が行われます。

```rust
use shiguredo_m3u8::{parse_multivariant_playlist, parse_media_playlist_with_context};

let master = parse_multivariant_playlist(master_input)?;
let media = parse_media_playlist_with_context(
    media_input,
    Some("http://example.com/playlist.m3u8"),
    Some(&master),
)?;
```

### Multivariant Playlist のビルド

```rust
use shiguredo_m3u8::{parse_multivariant_playlist, write_multivariant_playlist};

let playlist = parse_multivariant_playlist(input)?;
let output = write_multivariant_playlist(&playlist);
```

### Media Playlist のビルド

```rust
use shiguredo_m3u8::{parse_media_playlist, write_media_playlist};

let playlist = parse_media_playlist(input)?;
let output = write_media_playlist(&playlist);
```

### ラウンドトリップ

パース → ビルド → パースが同じ結果になることを保証しています。

```rust
use shiguredo_m3u8::{parse_media_playlist, write_media_playlist};

let parsed = parse_media_playlist(input)?;
let written = write_media_playlist(&parsed);
let reparsed = parse_media_playlist(&written)?;
assert_eq!(parsed, reparsed);
```

## HLS タグ

このライブラリが対応している HLS タグの一覧です。

### 基本タグ

- `#EXTM3U`
- `#EXT-X-VERSION`
- `#EXT-X-INDEPENDENT-SEGMENTS`
- `#EXT-X-START`
- `#EXT-X-DEFINE`

### Multivariant Playlist

- `#EXT-X-STREAM-INF`
  - BANDWIDTH, AVERAGE-BANDWIDTH, CODECS, SUPPLEMENTAL-CODECS, RESOLUTION, FRAME-RATE, HDCP-LEVEL, ALLOWED-CPC, VIDEO-RANGE, AUDIO, VIDEO, SUBTITLES, CLOSED-CAPTIONS, NAME, STABLE-VARIANT-ID, PATHWAY-ID
- `#EXT-X-MEDIA`
  - TYPE, GROUP-ID, NAME, URI, LANGUAGE, ASSOC-LANGUAGE, DEFAULT, AUTOSELECT, FORCED, INSTREAM-ID, CHARACTERISTICS, CHANNELS, BIT-DEPTH, SAMPLE-RATE, STABLE-RENDITION-ID
- `#EXT-X-I-FRAME-STREAM-INF`
  - BANDWIDTH, AVERAGE-BANDWIDTH, CODECS, SUPPLEMENTAL-CODECS, RESOLUTION, HDCP-LEVEL, VIDEO-RANGE, VIDEO, PATHWAY-ID, URI
- `#EXT-X-SESSION-DATA`
  - DATA-ID, VALUE, URI, LANGUAGE
- `#EXT-X-SESSION-KEY`
  - METHOD, URI, IV, KEYFORMAT, KEYFORMATVERSIONS
- `#EXT-X-CONTENT-STEERING`
  - SERVER-URI, PATHWAY-ID

### Media Playlist

- `#EXT-X-TARGETDURATION`
- `#EXT-X-MEDIA-SEQUENCE`
- `#EXT-X-DISCONTINUITY-SEQUENCE`
- `#EXT-X-PLAYLIST-TYPE`
- `#EXT-X-I-FRAMES-ONLY`
- `#EXT-X-ENDLIST`
- `#EXTINF`
- `#EXT-X-BYTERANGE`
- `#EXT-X-DISCONTINUITY`
- `#EXT-X-KEY`
  - METHOD (NONE, AES-128, AES-256-GCM, SAMPLE-AES, SAMPLE-AES-CTR), URI, IV, KEYFORMAT, KEYFORMATVERSIONS
- `#EXT-X-MAP`
- `#EXT-X-PROGRAM-DATE-TIME`
- `#EXT-X-DATERANGE`
  - ID, CLASS, START-DATE, END-DATE, DURATION, PLANNED-DURATION, END-ON-NEXT, X-\*, SCTE35-\*
- `#EXT-X-GAP`
- `#EXT-X-BITRATE`

### LL-HLS (Low Latency HLS)

- `#EXT-X-SERVER-CONTROL`
  - CAN-SKIP-UNTIL, CAN-SKIP-DATERANGES, HOLD-BACK, PART-HOLD-BACK, CAN-BLOCK-RELOAD
- `#EXT-X-PART-INF`
  - PART-TARGET
- `#EXT-X-PART`
  - URI, DURATION, INDEPENDENT, BYTERANGE, GAP
- `#EXT-X-PRELOAD-HINT`
  - TYPE (PART / MAP), URI, BYTERANGE-START, BYTERANGE-LENGTH
- `#EXT-X-RENDITION-REPORT`
  - URI, LAST-MSN, LAST-PART
- `#EXT-X-SKIP`
  - SKIPPED-SEGMENTS, RECENTLY-REMOVED-DATERANGES

## バリデーション

パース時に以下の検証を行います。

- `#EXT-X-VERSION` の最低互換バージョン検証
- singleton タグの重複検出
- Media Playlist と Multivariant Playlist のタグ混在検出
- fMP4 セグメントの `EXT-X-MAP` 必須検証
- `EXT-X-BYTERANGE` の offset 省略の整合性検証
- `EXT-X-STREAM-INF` の rendition group 参照不整合検出
- `EXT-X-RENDITION-REPORT` の相対 URI と `LAST-MSN` 必須検証
- `EXT-X-SESSION-DATA` の `DATA-ID` と `LANGUAGE` 重複検出
- `EXT-X-DATERANGE` の属性整合と `PROGRAM-DATE-TIME` 必須検証
- `EXT-X-DATERANGE` の ID 重複時の属性整合性検証
- SUBTITLES の `URI` 必須と `CODECS` の `wvtt` 不足検出
- `CLOSED-CAPTIONS` の `NONE` 値と group-id の混在禁止
- 属性リストの重複と不正な quoted-string 検出
- `EXT-X-KEY` の `METHOD` に応じた属性制約検証
- LL-HLS のタグ間制約検証（`HOLD-BACK`、`PART-HOLD-BACK` の下限値検証を含む）
- `EXT-X-ENDLIST` と `EXT-X-PRELOAD-HINT` の共存禁止

## 規格書

このライブラリが準拠している規格の一覧です。

- RFC 8216 - HTTP Live Streaming
  - <https://datatracker.ietf.org/doc/html/rfc8216>
- HTTP Live Streaming 2nd Edition (draft-pantos-hls-rfc8216bis)
  - <https://datatracker.ietf.org/doc/html/draft-pantos-hls-rfc8216bis>

## ライセンス

Apache License 2.0

```text
Copyright 2026-2026, Shiguredo Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
