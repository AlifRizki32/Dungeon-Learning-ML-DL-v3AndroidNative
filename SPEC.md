# Spesifikasi Build Dungeon Learning ML & DL v3

Sumber kebenaran tunggal untuk pengerjaan v3. Versi 1.2, 17 November 2026.

## Cara memakai file ini

File ini yang dipegang selama v3 dikerjakan, bukan ingatan dan bukan tebakan. Aturannya:

- Bila kode yang sedang ditulis, PRD v3, atau SDD v3 bertentangan dengan file ini, pengerjaan berhenti di titik itu dan pertentangan itu ditanyakan ke Jamal dulu. Tidak ada yang memilih sendiri diam-diam.
- PRD v3 (versi dokumen 1.1) dan SDD v3 (draf 0.1) tetap dibaca untuk rincian, tetapi keputusan di Bagian 2 file ini mengalahkan isi keduanya. Ini penting karena SDD masih memuat usulan lama yang sudah diganti: porting Dungeon 3D ke wgpu dan AI Mentor lewat Gemini langsung. Dua usulan itu sudah tidak berlaku.
- Setiap angka, nama fitur, dan rumus di file ini disalin dari kode v2 yang sudah rilis. Bila sebuah rincian tidak ada di file ini dan tidak ada di kode v2, rincian itu tidak boleh dikarang; dicatat sebagai pertanyaan terbuka.
- File ini hanya berubah lewat persetujuan Jamal, dan setiap perubahan dicatat di Bagian 8.

## Keputusan yang sudah dikunci

Lima keputusan di bawah ini sudah diambil Jamal dan tidak dibuka lagi tanpa persetujuannya.

### ID paket: `dungeonlearning.app` — terkonfirmasi

ID paket sudah dikonfirmasi Jamal pada 17 November 2026: `dungeonlearning.app` — pakai n, persis seperti itu.

Bukan `dungeonlearing.app` (tanpa n, itu typo) dan bukan `id.dungeonlearning.app` (rekomendasi SDD yang tidak dipilih). String ini terkunci untuk build pertama dan seterusnya, karena ID paket tidak bisa diganti setelah rilis: mengganti ID berarti aplikasi dianggap baru dan progres pengguna hilang.

### Orientasi: potret, dikunci untuk sementara

Orientasi dikunci potret (HP tegak) untuk sementara ini. Kata "sementara" bagian dari keputusannya: kunci ini boleh dibuka lagi nanti atas keputusan Jamal, tetapi selama belum dibuka, tidak ada layout lanskap (mendatar) yang dirancang atau dibangun. Layout hanya dirancang untuk layar tegak.

### Dungeon 3D: WebView khusus, kode lama dipakai ulang

Dungeon 3D adalah satu-satunya pengecualian native di v3.

- Semua bagian v3 selain Dungeon 3D adalah Rust native.
- Dungeon 3D ditampilkan lewat WebView khusus yang hanya dipakai untuk layar itu.
- Kode TypeScript/Three.js dari v2 dipakai ulang apa adanya. Kode itu tidak diporting ke Rust dan tidak ditulis ulang ke wgpu.
- Aset Dungeon 3D dibundel lokal di dalam aplikasi, supaya layar 3D tetap jalan offline.
- Peta 2D tetap jalan mandiri. Bila layar 3D gagal dimuat, sisa game tidak ikut lumpuh.

### AI Mentor: tidak ada di v3.0

Chatbot AI Mentor resmi tidak disertakan di v3.0, untuk saat ini.

- Tidak ada UI mentor di v3.0: tidak ada tombol, layar, atau menu mentor.
- Tidak ada kunci Gemini, tidak ada server backend, dan tidak ada panggilan jaringan ke layanan AI.
- Yang diuji saat penerimaan justru ketiadaannya: aplikasi v3.0 tidak boleh memuat sisa jalur AI Mentor dari v2, termasuk panggilan `POST /api/chat`.
- Menghidupkan lagi AI Mentor adalah keputusan rilis berikutnya, bukan pekerjaan v3.0.

### Codespaces: tidak dipakai

GitHub Codespaces tidak dipakai untuk pengerjaan v3. Keputusan ini diambil Jamal pada 17 November 2026.

- Build v3 tetap lewat alur yang sudah berjalan: clone lokal di VM pengerjaan, lalu push ke GitHub.
- Tidak ada penyiapan, konfigurasi, atau biaya Codespaces yang dikerjakan untuk v3.
- Rincian dan aturan commit untuk keputusan ini ada di Bagian 6.

## Yang tidak boleh berubah dari v2

v3 adalah penulisan ulang platform, bukan redesign. Tema dan fitur v2 dipertahankan semua, kecuali AI Mentor yang dikecualikan di Bagian 2.

### Tema

Tema pixel-art v2 tidak berubah: token warna dan gaya pixel-box diambil persis dari berkas gaya v2 (`pixel.css`), bukan palet baru. Tidak ada tema, ikon, atau gaya visual baru di v3.0.

### Inventaris fitur yang wajib ada

- 5 lantai ditambah Master Tower, dengan 28 topik terbagi 6 / 5 / 6 / 3 / 8 per lantai.
- 5 boss, dari Raja Matematika sampai Raja Deadline Agung.
- 7 quest Road to Master.
- Lesson engine v2, termasuk pembagian materi per chunk.
- Spellbook.
- Camp tracker.
- Musik chiptune v2, dinilai dari paritas perilaku, bukan harus identik bit demi bit.
- Penyimpanan progres lokal, dengan impor save v2 lewat format ekspor yang sama. Save dari localStorage browser tidak bisa dibaca langsung oleh aplikasi, jadi jalurnya adalah berkas ekspor v2.

### Angka dan aturan yang disalin dari kode v2

Angka-angka ini kunci. Bila hasil porting berbeda dari angka ini, portingnya yang salah.

| Aturan | Nilai dari v2 |
| --- | --- |
| Rumus XP | `round(((min + max) / 2) * 50)` |
| Kurva level | `60 * (L - 1)^2` |
| Kotak jadwal review | 1, 3, 7, 14, 30 hari |
| Rating review | lupa / ragu / ingat |
| Skala penyesuaian review | dijepit 0,5 sampai 2 |
| Batas sesi review | maks. 10 item per sesi |
| Kunci save lokal v2 | `dungeon-ml-dl:v1` |

Konten kurikulum v2 berstatus `needs-human-review`. Status itu ikut terbawa ke v3 apa adanya; pengerjaan v3 tidak mengubah isi kurikulum.

## Arsitektur yang dikerjakan

Struktur mengikuti SDD v3: workspace Rust 3 lapis.

| Lapis | Isi | Aturan |
| --- | --- | --- |
| `dungeon-data` | Data dan konten, JSON v2 dipakai ulang dengan serde | Tidak menulis ulang konten |
| `dungeon-core` | Logika murni game: XP, level, buka-kunci, streak, lesson, review | Bisa diuji tanpa Android; test unit v2 ikut diporting |
| `dungeon-app` | Aplikasi Android: UI, penyimpanan, audio, WebView Dungeon 3D | Satu-satunya lapis yang menyentuh Android |

Pilihan teknis dari SDD yang dipakai: UI egui di atas wgpu, audio kira, penyimpanan save atomik dengan cadangan. WebView hanya muncul di `dungeon-app` dan hanya untuk Dungeon 3D.

### Usulan yang belum menjadi keputusan

Dua hal di bawah ini masih usulan SDD, bukan keputusan Jamal. Keduanya boleh dipakai sebagai titik awal kerja, tetapi tidak boleh ditulis sebagai keputusan final di kode, README, atau rilis sebelum Jamal menyetujui.

- Android minimum API 29.
- Toolkit UI egui sebagai pilihan final, selama belum ada keputusan lain dari Jamal.

### Target terukur

Target ini dari PRD v3 dan menjadi kriteria penerimaan di Bagian 7.

| Target | Nilai |
| --- | ---: |
| Lebar layar yang didukung | 360–430 dp |
| Target sentuh minimum | 48 dp |
| Start dingin | maks. 2 detik |
| Ukuran APK | maks. 50 MB |
| Fitur inti offline | 100% |

## Aturan anti-melenceng

Aturan ini berlaku untuk setiap sesi pengerjaan v3, termasuk sesi baru yang tidak membawa ingatan percakapan sebelumnya.

- Sebelum mengerjakan satu bagian, baca dulu bagian terkait di file ini dan cek sumbernya di kode v2. Urutannya selalu: file ini dulu, lalu kode v2, baru menulis kode.
- Tidak ada fitur baru di luar inventaris Bagian 3. Ide yang muncul di tengah jalan dicatat sebagai pertanyaan terbuka, bukan langsung dibangun.
- Tidak ada dependensi, layanan, atau akun baru tanpa persetujuan Jamal. Khususnya: tidak ada kunci API apa pun yang ditanam di APK.
- Tidak ada perubahan tema, nama fitur, atau istilah di dalam game. Nama di v3 sama dengan nama di v2.
- Bila sebuah perilaku v2 tidak jelas dari kodenya, perilaku itu ditanyakan, bukan ditebak. Hasil tebakan tidak boleh masuk kode dengan label paritas.
- Klaim selesai harus disertai bukti yang bisa dicek: hasil build, hasil test, atau hasil uji di perangkat. "Seharusnya jalan" bukan bukti.
- Repo v3 adalah repo baru. Repo v1 dan v2 tidak ditimpa dan tidak diubah oleh pengerjaan v3.

## Codespaces: diputuskan tidak dipakai

Jamal memutuskan pada 17 November 2026: GitHub Codespaces tidak dipakai untuk pengerjaan v3.

- Build v3 tetap lewat alur yang sudah berjalan: clone lokal di VM pengerjaan, lalu push ke GitHub lewat konektor.
- Toolchain Rust dan Android SDK/NDK dipasang dan dikelola di VM pengerjaan itu, bukan di Codespaces.
- Tidak ada penyiapan, konfigurasi, atau biaya Codespaces untuk v3. Membuka lagi pilihan ini butuh keputusan baru dari Jamal.
- Uji penerimaan v3 tetap wajib di HP fisik Jamal (lihat Bagian 7), apa pun lingkungan build-nya.

### Aturan commit

Bertambahnya commit bukan tujuan yang dikejar dengan commit kosong. Commit bertambah karena pekerjaan dipecah kecil dan sering per fase atau fitur. Aturannya:

- Satu commit adalah satu unit kerja nyata yang selesai dan bisa dijelaskan, misalnya satu bagian logika yang lolos test atau satu layar yang selesai.
- Pesan commit menyebut pekerjaan yang dilakukan, bukan sekadar nomor urut atau penanda waktu.
- Tidak ada commit kosong, commit pemecah berkas tanpa alasan, atau commit otomatis hanya untuk menaikkan hitungan.
- Dengan aturan ini, jumlah commit memang akan bertambah selama v3 dikerjakan, karena pekerjaannya dipecah per unit yang jelas.

## Checklist pengerjaan dan kriteria selesai

Pengerjaan mengikuti fase PRD. Sebuah fase hanya selesai bila gerbang ceknya lolos, bukan bila kodenya terasa selesai.

1. Toolchain: Rust, Android SDK/NDK, dan target `aarch64-linux-android` terpasang di VM pengerjaan (bukan Codespaces, sesuai Bagian 6) dan satu build kosong berhasil. ID paket memakai string terkonfirmasi di Bagian 2: `dungeonlearning.app`.
2. Inti game: `dungeon-data` dan `dungeon-core` selesai, test unit hasil porting dari v2 lolos, dan angka-angka Bagian 3 cocok dengan kode v2.
3. UI: semua layar v2 terpetakan ke layar Android potret, lebar 360–430 dp, target sentuh minimal 48 dp, token tema sama dengan v2.
4. Penyimpanan: save atomik dengan cadangan berjalan, dan impor save dari ekspor v2 terbukti dengan satu berkas ekspor asli.
5. Audio: musik chiptune berjalan dengan paritas perilaku dari v2.
6. Dungeon 3D: layar 3D terbuka dari WebView khusus, memakai kode TypeScript/Three.js v2 apa adanya, dibundel lokal, dan jalan offline. Peta 2D tetap jalan bila layar 3D gagal dimuat.
7. Uji perangkat dan rilis: APK diuji di HP fisik Jamal, karena emulator Android tidak bisa berjalan di VM pengerjaan (tidak ada KVM dan RAM bebas hanya sekitar 473 MB). Rilis masuk repo v3 baru.

### Kriteria penerimaan v3.0

- Semua fitur di inventaris Bagian 3 ada dan berjalan, dengan satu pengecualian yang disengaja: AI Mentor tidak ada sama sekali, termasuk tidak ada UI mentor dan tidak ada panggilan jaringan AI atau backend.
- Tema pixel-art sama dengan v2.
- Orientasi terkunci potret.
- Target terukur di Bagian 4 terpenuhi: start dingin, ukuran APK, dan fitur inti 100% offline, termasuk Dungeon 3D dari bundel lokal.
- ID paket yang tertera di APK adalah `dungeonlearning.app`, string terkonfirmasi di Bagian 2.

## Log perubahan spesifikasi

| Versi | Tanggal | Perubahan |
| --- | --- | --- |
| 1.0 | 16 Nov 2026 | File dibuat. Mencatat empat keputusan terkunci: ID paket `dungeonlearning.app` (ambigu, wajib konfirmasi), orientasi potret dikunci sementara, Dungeon 3D via WebView khusus, AI Mentor tidak ada di v3.0. Codespaces dicatat sebagai pertimbangan yang belum diputuskan. |
| 1.1 | 17 Nov 2026 | ID paket dikonfirmasi Jamal: `dungeonlearning.app` — pakai n. Peringatan ambiguitas dan gerbang konfirmasi sebelum build pertama dihapus; string ini terkunci untuk rilis. |
| 1.2 | 17 Nov 2026 | Codespaces diputuskan tidak dipakai, sesuai keputusan Jamal. Bagian 6 diganti dari bahan pertimbangan menjadi keputusan final: build tetap lewat clone lokal di VM pengerjaan lalu push ke GitHub, dengan commit kecil dan sering per fase/fitur. Keputusan ini ditambahkan sebagai keputusan kelima yang terkunci di Bagian 2. |

Perubahan berikutnya hanya masuk tabel ini setelah disetujui Jamal, dengan menyebut keputusan yang berubah dan tanggalnya.
