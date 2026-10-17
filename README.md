# LAN Device Discovery

A portable, single-`.exe` Windows 10/11 clone of the Ubiquiti Discovery Tool.
Scans the local network for airOS / UniFi / EdgeMax devices and lists them —
**no Java, no .NET, no installer** (the original is a `.jar`).

Written in Rust with [native-windows-gui](https://crates.io/crates/native-windows-gui)
(plain Win32 controls). The release exe is about **0.4 MB**, statically linked
against the C runtime, and depends only on DLLs that ship with Windows.

## Features

- **Scan** — broadcasts the Ubiquiti UDP discovery probe (port 10001) on every
  active IPv4 interface and lists replies.
- Device table: Product Name, IP Address, Hardware Address (MAC), System Name,
  SSID, Firmware Version. Click a column header to sort.
- **Details…** — per-device fields plus all reported network addresses.
- **Copy** — copies selected rows (tab-separated).
- **Web UI** — opens `http://<device-ip>` in the default browser.
- **Search** filter and a live device **Total**.

> Device **Reboot** and **SSH tunnel** (from the original tool) are intentionally
> out of scope — they require device credentials and challenge-response auth.

## Run

Just run `UbntDiscovery.exe`. Copy it anywhere — it needs no install and no
runtime. Click **Scan**.

If Windows Firewall prompts on first scan, allow UDP so replies can be received.

## Build from source

Requires the Rust toolchain (`stable-x86_64-pc-windows-msvc`) and the Visual
Studio Build Tools (MSVC linker).

```powershell
cargo test                    # unit tests
cargo clippy --all-targets -- -D warnings
cargo build --release         # -> target\release\UbntDiscovery.exe
```

For ARM64 Windows: `rustup target add aarch64-pc-windows-msvc` and
`cargo build --release --target aarch64-pc-windows-msvc`.

## Project layout

```
src/protocol/  client (UDP scan), tlv (reply parser), device, model_names, request
src/ui/        main_window, details_window, model (pure UI logic, unit-tested)
resources/     app.ico, app.manifest (comctl32 v6 visual styles, per-monitor DPI)
build.rs       embeds the icon, manifest and version info
```

## Protocol notes

The discovery exchange is a UDP broadcast to port **10001**. The reply is a
4-byte header followed by TLV records (`type`, 2-byte length, value). The model
code → product-name table (114 entries) and the UI field set were recovered from
the original `ubnt-discovery-v2.5.1.jar`. The parser is defensive: unknown tags
are skipped and malformed/truncated packets never panic.
