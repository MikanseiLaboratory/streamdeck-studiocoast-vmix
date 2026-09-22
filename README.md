# vMix [MikanseiLaboratory]

任意の台数の vMix を、Stream Deck から TCP API（既定ポート 8099）で操作するプラグインです。パスワード欄はありません。

## できること

- Program / Preview、トランジション、Stinger 1〜8、Fade to Black、Overlay 1〜8
- 録画、配信、外部出力、MultiCorder、フルスクリーン、リプレイ
- ミュート、ソロ、バス送信、再生、リスト、タイトル
- ショートカット関数と Raw TCP
- Stream Deck+ の音量ダイヤル
- 接続先は All / Group / 個別。Mix は Main と Mix 2〜16 です

Mix の番号は TCP では 1 つずれます。画面上の Mix 2 は `Mix=1`、フィードバックは `InputMix2` です。

## ビルド

隣のディレクトリに、`vmix-shortcuts` crate を含む `vmix-rs`（ブランチ `feat/vmix-shortcuts`）が必要です。GitHub Actions は `MikanseiLaboratory/vmix-rs` のそのブランチを取得します。

```sh
cargo test --workspace
cargo run -p vmix-plugin --bin typegen
cd pi && npm install && npm run build
```

ローカルへ入れる場合は `INSTALL=1 ./publish.sh` です。
