# Changelog

Versi memakai tanggal (`YYYY.M.D`).

## [2026.11.6]

### Ditambahkan
- Pemilih **Mode** di toolbar dengan tiga mode deteksi baru selain Ubiquiti:
  - **MikroTik (MNDP)** — neighbor discovery seperti tab Neighbors di WinBox (UDP 5678).
  - **CCTV / NVR (SADP)** — kamera dan NVR Hikvision seperti SADP Tool
    (multicast 239.255.255.250:37020).
  - **NAS (QNAP)** — NAS QNAP seperti Qfinder Pro (UDP 8097).
- Mengganti mode langsung memulai scan untuk mode itu; hasil mode sebelumnya dikosongkan.
- Judul kolom dan dialog Details menyesuaikan mode; **Web UI** memakai port yang dilaporkan
  perangkat (Hikvision) atau 8080 (QNAP).
- Android: pemilih Mode yang sama (Ubiquiti, MikroTik, SADP, QNAP); judul kolom dan detail
  mengikuti mode, Open Web UI memakai port perangkat.

### Diperbaiki
- Tema gelap: area list yang kosong tidak lagi putih dan teks header kolom kembali terbaca.

### Diubah
- Lebar jendela minimum 1130 px agar toolbar (Mode, Search, tema, About) tidak bertumpuk.

### Catatan
- Kolom firmware QNAP menampilkan tanggal dan nomor build; balasan broadcast QNAP tidak
  membawa nomor versi bertitik.

## [2026.10.18]

### Diubah
- Nama tampilan aplikasi (exe dan APK) menjadi "Ubnt Discovery".

### Diperbaiki
- Android: teks tenggelam saat tema aplikasi berbeda dari tema sistem (latar kini mengikuti tema aplikasi, status/navigation bar disesuaikan).
- Palet terang/gelap baru dengan kontras lebih jelas di APK dan exe; kartu, chip, dan tombol dirapikan.

### Ditambahkan
- Tombol tema gelap/terang di exe dan APK dengan palet kontras tinggi; pilihan disimpan.
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
