# 変更履歴

- UPDATE
  - 後方互換がある変更
- ADD
  - 後方互換がある追加
- CHANGE
  - 後方互換のない変更
- FIX
  - バグ修正

## develop

- [CHANGE] Error 型の公開経路を `shiguredo_m3u8::error` モジュールに変更する
  - @voluntas
- [CHANGE] MSRV (rust-version) を 1.93 に上げる
  - @voluntas
- [UPDATE] エラー種別・属性 enum の `#[non_exhaustive]` を削除する
  - @voluntas

### misc

- PBT を proptest から noprop に移行する
- fuzz ターゲットを公開 API のみで動作するように変更し、内部パーサ関数の公開 (fuzz_helpers) を廃止する
- `src/parser/mod.rs` / `src/builder/mod.rs` を廃止する
- prek 設定を整備する (tombi 追加・cargo test の pre-push 限定化)
- テストコードのエラーメッセージを日本語化する
