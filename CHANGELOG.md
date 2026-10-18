# Changelog

Versi memakai tanggal (`YYYY.M.D`).

## [2026.10.18]

### Ditambahkan
- Aplikasi Android (`android/`): UI Kotlin/Compose, inti discovery Rust yang sama via JNI.
  APK release ditandatangani, ABI arm64-v8a dan armeabi-v7a, minSdk 24.

### Diubah
- `native-windows-gui` dan `winresource` kini hanya untuk target Windows.

## [2026.10.17]

Rilis publik pertama.

### Ditambahkan
- Rewrite ke Rust dari Ubiquiti Discovery Tool v2.5.1: satu `.exe` portabel (~0,4 MB,
  turun dari 62,9 MB pada prototipe C#), tanpa Java/.NET.
- Scan UDP 10001 di semua interface IPv4 aktif; parser TLV defensif.
- Scan otomatis 4 detik setelah aplikasi dibuka, dengan hitung mundur di tombol Scan.
- Tabel perangkat dengan sort, filter Search, Details, Copy, dan Web UI.
- Tombol **About**: avatar GitHub, versi, tautan repo, dan lisensi.
- Lisensi ganda MIT OR Apache-2.0.
