# tally

tally asks "what are you doing?" every few minutes, from the menu bar. You never start or stop
a timer. Your answers add up to daily, weekly and monthly totals that you can correct in place
and export.

- **Local only.** One SQLite file on your machine. No account, no cloud.
- **Keyboard-first.** Every prompt can be answered without touching the mouse.
- **macOS and Windows.** Linux runs but is not polished.

## Install

You need a Rust toolchain ([rustup.rs](https://rustup.rs)).

```sh
git clone <this repo> tally
cd tally
cargo install-app
```

On macOS this builds `tally.app` (with its icon and no Dock icon), puts it in `/Applications`
(or `~/Applications` if your account can't write there) and opens it. Find it again in Spotlight or Launchpad. Use `cargo xtask install --dir
~/Applications` to install somewhere else.

On Windows and Linux, `cargo install-app` runs `cargo install`, which puts the `tally` binary in
`~/.cargo/bin`.

To update, pull and run `cargo install-app` again. If tally is running, quit it from the menu bar
first.

## Using tally

- **The prompt** appears on your interval, inside your working hours. It is pre-filled with your
  last answer, so **Enter** means "still doing that". Type to filter or add an activity
  (`Category: Activity` groups it), use **↑/↓** to move through matches, and press **Esc** to skip.
- **Ask now:** the global shortcut (default **⌥⇧T**) opens the prompt when you switch tasks.
- **The dashboard** opens when you click the menu-bar icon. Click any total to edit it: `-30` takes
  30 minutes off, `+15` adds 15, and `1h 30` sets it.
- **The main window** (from the dashboard's ≡ menu) holds Calendar (your logged time as a day, week or month), Activities, Export (CSV or JSON) and Settings.
- **Away time:** when you are away longer than the idle threshold, tally stops asking and, when you
  come back, asks what that time was.

## Your data

| OS      | Folder                                  |
|---------|-----------------------------------------|
| macOS   | `~/Library/Application Support/tally/`  |
| Windows | `%APPDATA%\tally\`                      |
| Linux   | `~/.local/share/tally/`                 |

It holds `tally.db` and a week of logs in `logs/`. Set `TALLY_DATA_DIR` to try tally against a
different folder without touching your real data.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p tally          # run from source
cargo xtask bundle          # build target/release/bundle/tally.app (macOS)
```

The workspace has three crates. `tally-core` holds the scheduler, store, totals and export, with no
UI code. `tally-app` is the iced desktop app. `xtask` holds build tasks. The design is in
`DESIGN.md` and `PRODUCT.md`, and the functional spec is in `docs/superpowers/specs/`.

## Licence

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your
option. The bundled fonts are under the SIL Open Font License (see `crates/tally-app/assets/fonts`).
