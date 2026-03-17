# bis 由来の EXT-X-VERSION 要件を検証する

Created: 2026-03-18
Model: GPT-5.4

## 概要

`EXT-X-VERSION` の最低要件が古い仕様までしか入っておらず、bis で追加された要件を見逃している。

現状の実装は主に 2 - 7 しか見ていないため、次のような playlist を許可してしまう。

- 変数置換を使っているのに `EXT-X-VERSION` が 8 未満
- `EXT-X-SKIP` があるのに `EXT-X-VERSION` が 9 未満
- `EXT-X-SKIP` で `EXT-X-DATERANGE` を置き換える更新なのに `EXT-X-VERSION` が 10 未満
- `QUERYPARAM` を使っているのに `EXT-X-VERSION` が 11 未満
- `REQ-` で始まる属性を持つのに `EXT-X-VERSION` が 12 未満
- `EXT-X-MEDIA` の `INSTREAM-ID` が CLOSED-CAPTIONS 以外なのに `EXT-X-VERSION` が 13 未満

## 根拠

- 資料: `refs/draft-pantos-hls-rfc8216bis-20.txt`
- 節: 7 `Protocol Version`
- 内容: 上記の機能ごとに必要な最小 version が明記されている

## 必要性

`EXT-X-VERSION` は互換性判定の基準なので、ここを誤ると古いクライアント向けの互換性保証が崩れる。
README の「最低互換バージョン検証」とも一致していない。

## pending 理由

bis は draft であり version 要件自体が将来変わる可能性がある。また実世界の HLS 配信サーバーは version タグを正しく設定していないことが多く、厳密なチェックを入れると実用上パースできないプレイリストが増えるリスクがある。draft を優先する必要はないため保留とする。
