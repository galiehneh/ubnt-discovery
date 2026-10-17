# LAN Device Discovery

A portable, single-`.exe` Windows 10/11 clone of the Ubiquiti Discovery Tool.
Scans the local network for airOS / UniFi / EdgeMax devices and lists them —
**no Java required** (the original is a `.jar`).

Built with .NET 8 WinForms, published as a self-contained single file.

## Features

- **Scan** — broadcasts the Ubiquiti UDP discovery probe (port 10001) on every
  active IPv4 interface and lists replies.
- Device table: Product Name, IP Address, Hardware Address (MAC), System Name,
  SSID, Firmware Version.
- **Details…** — per-device fields plus all reported network addresses.
- **Copy** — copies selected rows (tab-separated).
- **Web UI** — opens `http://<device-ip>` in the default browser.
- **Search** filter and a live device **Total**.

> Device **Reboot** and **SSH tunnel** (from the original tool) are intentionally
> out of scope — they require device credentials and challenge-response auth.

## Run

Just run the published `UbntDiscovery.exe`. Copy it anywhere — it needs no
install and no runtime. Click **Scan**.

If Windows Firewall prompts on first scan, allow UDP so replies can be received.

## Build from source

Requires the .NET 8 SDK.

```powershell
# Build + test
dotnet test

# Produce the portable single exe
dotnet publish src/UbntDiscovery -c Release
# -> src/UbntDiscovery/bin/Release/net8.0-windows/win-x64/publish/UbntDiscovery.exe
```

For ARM64 Windows, add `-r win-arm64` to the publish command.

## Project layout

```
src/UbntDiscovery/
  Protocol/   DiscoveryClient, TlvReader, DiscoveryRequest, DiscoveredDevice, ModelNames
  Ui/         MainForm, DetailsForm
tests/UbntDiscovery.Tests/   TLV parser + model-table unit tests (xUnit)
```

## Protocol notes

The discovery exchange is a UDP broadcast to port **10001**. The reply is a
4-byte header followed by TLV records (`type`, 2-byte length, value). The model
code → product-name table (114 entries) and the UI field set were recovered from
the original `ubnt-discovery-v2.5.1.jar`. `TlvReader` is defensive: unknown tags
are skipped and malformed/truncated packets never throw.
