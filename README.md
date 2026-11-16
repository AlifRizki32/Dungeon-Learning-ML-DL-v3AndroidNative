# Dungeon Learning ML & DL — v3 (Android Native, Rust)

Game belajar Machine Learning & Deep Learning berbentuk RPG dungeon 16-bit,
sekarang ditulis ulang sebagai aplikasi **Android native dengan Rust**.

> Status: M0 — instalasi toolchain (Rust + Android SDK) sedang berjalan.
> Dokumen perencanaan: `SPEC.md` (sumber kebenaran tunggal), PRD & SDD ada di
> repo v2 dan di arsip proyek.

## Keputusan terkunci

- **ID paket:** `dungeonlearning.app`
- **Target:** Android 16 (API 36), minimum API 29 (Android 10)
- **Orientasi:** potret, dikunci untuk sementara
- **UI:** Rust native penuh — semua layar kecuali Dungeon 3D
- **Dungeon 3D:** WebView khusus, kode TypeScript/Three.js v2 dipakai ulang
  apa adanya (dibundel lokal, tetap offline), tidak diporting ke Rust
- **AI Mentor / chatbot:** tidak dipakai di v3.0 — aplikasi 100% offline
- **Tema & fitur:** sama seperti v2, tidak berubah
- **Repo:** baru, tidak menimpa v1 maupun v2

## Repo terkait

- v1 (asli, web): https://github.com/AlifRizki32/Dungeon-Learning-ML-DL
- v2 (web, dibersihkan & dioptimasi): https://github.com/AlifRizki32/Dungeon-Learning-ML-DL-v2
