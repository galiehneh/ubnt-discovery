# Changelog

Versi memakai tanggal (`YYYY.M.D`).

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
