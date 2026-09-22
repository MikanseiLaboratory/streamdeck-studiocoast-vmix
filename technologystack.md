# 技術スタック

変更する場合は承認を得ること。

- Rust 1.85 (edition 2021)
- `streamdeck-plugin` 0.1.0 (`macros`, `typegen`)
- `vmix-rs`（TCP、`std`）。ローカルの `../vmix-rs` を参照し、`feat/vmix-shortcuts` の `vmix-shortcuts` crate を含む
- `tokio` 1
- Property Inspector: React 18、Vite 6、TypeScript 5、`@mikanseilaboratory/streamdeck-pi-client` 0.1.0
- 設定型の共有: `ts-rs` 10.1
- 対象: Windows x64、macOS arm64、macOS x64
- Stream Deck SDK 2、vMix TCP API（既定ポート 8099）。HTTP API は使わない
