# Repoti — Reverse Pomodoro Timer

Work and break times are **minimums**: the timer counts down, beeps, then keeps counting up.
Sessions are appended to `~/.local/share/repoti/sessions.csv` (location changeable in settings).

## Build (Ubuntu 24.04)

```sh
sudo apt install build-essential curl libwebkit2gtk-4.1-dev libssl-dev librsvg2-dev libxdo-dev
curl https://sh.rustup.rs -sSf | sh   # Rust toolchain
npm install
npm run tauri build -- --bundles deb
```

For development with hot reload: `npm run tauri dev`

## Install

```sh
sudo apt install ./src-tauri/target/release/bundle/deb/Repoti_0.1.0_amd64.deb
```

Then launch **Repoti** from the app menu. Uninstall with `sudo apt remove repoti`.
