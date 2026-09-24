# vMix Plugin for StreamDeck

Stream Deck plugin for any number of named vMix instances.

Download the latest plugin from the [releases page](https://github.com/MikanseiLaboratory/streamdeck-studiocoast-vmix/releases/latest).

## Requirements

- Stream Deck 6.4+
- vMix with the TCP API enabled (Settings → Web Controller, port 8099)
- Windows 10 or macOS 10.15+

## Use

1. In vMix, enable **Settings → Web Controller → TCP API**.
2. `127.0.0.1:8099` is already listed. Other machines are added by IP address from **Manage instances**.
3. Choose a target. Input and Mix lists come from that vMix.

If it stays on Connecting or Unreachable, the reason is on the instance card. The same lines are appended to:

- Windows: `%APPDATA%\Elgato\StreamDeck\logs\dev.mikanseilaboratory.vmix.log`
- macOS: `~/Library/Logs/ElgatoStreamDeck/dev.mikanseilaboratory.vmix.log`

## Build

`vmix-rs` (branch `feat/vmix-shortcuts`) must be checked out next to this repo.

```sh
cargo test --workspace
cargo run -p vmix-plugin --bin typegen
cd pi && npm install && npm run build
```

`INSTALL=1 ./publish.sh` installs the local build.

## License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
