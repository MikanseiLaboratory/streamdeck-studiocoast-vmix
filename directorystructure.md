# ディレクトリ構成

```
crates/vmix-pool/         N 台の vMix TCP 接続、ACTS、XML キャッシュ
crates/vmix-plugin/       Stream Deck プラグイン本体とアクション
pi/                       Property Inspector (React)
plugin/dev.mikanseilaboratory.vmix.sdPlugin/
  manifest.json
  en.json
  images/
  propertyinspector/      Vite のビルド出力
  bin/plugin              macOS ユニバーサルバイナリ
  bin/plugin.exe          Windows 実行ファイル
publish.sh / publish.ps1
```

ショートカット一覧は隣接する `vmix-rs` の `vmix-shortcuts` crate にあり、`cargo run -p vmix-plugin --bin typegen` が Property Inspector へコピーします。
