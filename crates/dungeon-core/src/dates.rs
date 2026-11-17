//! Aritmetika tanggal kalender lokal (kunci `YYYY-MM-DD`).
//!
//! v2 memakai `Date` lokal browser; core Rust tidak membaca jam sistem
//! (murni, bisa diuji) — tanggal "hari ini" selalu dipasok pemanggil.
//! Algoritma hari-dari-sipil Howard Hinnant (tanpa dependensi tanggal).

/// Nama bulan Indonesia (pendek) — label rentang minggu.
pub const MONTHS_SHORT_ID: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des",
];

/// Nama bulan Indonesia (penuh) — proyeksi selesai.
pub const MONTHS_FULL_ID: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

/// Apakah string kunci tanggal `YYYY-MM-DD` valid (bentuk + rentang).
pub fn is_valid_date_key(s: &str) -> bool {
    parse_date_key(s).is_some()
}

/// Mengurai `YYYY-MM-DD` menjadi (tahun, bulan 1-12, hari 1-31).
/// `None` bila bentuk salah atau tanggal tidak ada (mis. 2026-02-30).
pub fn parse_date_key(s: &str) -> Option<(i32, u32, u32)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

/// Format kunci tanggal `YYYY-MM-DD` (sama seperti `formatLocalDate` v2).
pub fn format_date_key(year: i32, month: u32, day: u32) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Jumlah hari sejak 1970-01-01 (algoritma Hinnant).
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = ((month + 9) % 12) as i64;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Kebalikan `days_from_civil`: (tahun, bulan, hari).
pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y } as i32;
    (year, month, day)
}

/// Selisih hari kalender (b - a), sama seperti `diffCalendarDays` v2.
/// `None` bila salah satu kunci tidak valid.
pub fn diff_days(a: &str, b: &str) -> Option<i64> {
    let (ya, ma, da) = parse_date_key(a)?;
    let (yb, mb, db) = parse_date_key(b)?;
    Some(days_from_civil(yb, mb, db) - days_from_civil(ya, ma, da))
}

/// Menambah N hari ke kunci tanggal (pembulatan ke hari utuh).
/// Kunci tidak valid memakai `today_key` sebagai titik awal — sama seperti
/// `addDaysToDateKey` v2 yang jatuh kembali ke tanggal hari ini.
pub fn add_days(date_key: &str, days: i64, today_key: &str) -> String {
    let base = parse_date_key(date_key).or_else(|| parse_date_key(today_key));
    match base {
        Some((y, m, d)) => {
            let (ny, nm, nd) = civil_from_days(days_from_civil(y, m, d) + days);
            format_date_key(ny, nm, nd)
        }
        None => today_key.to_string(),
    }
}

/// Hari dalam minggu: 0 = Minggu, 1 = Senin, ..., 6 = Sabtu.
/// `None` bila kunci tidak valid.
pub fn weekday(date_key: &str) -> Option<u32> {
    let (y, m, d) = parse_date_key(date_key)?;
    let days = days_from_civil(y, m, d);
    Some(((days + 4).rem_euclid(7)) as u32)
}

/// Kunci Senin (`YYYY-MM-DD`) untuk minggu kalender yang memuat tanggal itu.
/// `None` bila kunci tidak valid.
pub fn monday_of_week(date_key: &str) -> Option<String> {
    let wd = weekday(date_key)?;
    let diff_to_monday: i64 = if wd == 0 { -6 } else { 1 - wd as i64 };
    let (y, m, d) = parse_date_key(date_key)?;
    let (ny, nm, nd) = civil_from_days(days_from_civil(y, m, d) + diff_to_monday);
    Some(format_date_key(ny, nm, nd))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_roundtrip_and_diff() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(days_from_civil(2026, 11, 17)), (2026, 11, 17));
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(diff_days("2026-10-28", "2026-10-29"), Some(1));
        assert_eq!(diff_days("2026-10-28", "2026-10-28"), Some(0));
        assert_eq!(diff_days("2026-10-31", "2026-10-28"), Some(-3));
        assert_eq!(diff_days("2026-02-30", "2026-03-01"), None);
        assert_eq!(add_days("2026-11-02", 3, "2026-11-02"), "2026-11-05");
        assert_eq!(add_days("2026-11-02", 14, "2026-11-02"), "2026-11-16");
        assert_eq!(add_days("2026-12-31", 1, "2026-12-31"), "2027-01-01");
        // 2026-10-30 Jumat -> Senin 2026-10-26; 2026-11-01 Minggu -> 2026-10-26.
        assert_eq!(weekday("2026-10-30"), Some(5));
        assert_eq!(weekday("2026-11-01"), Some(0));
        assert_eq!(monday_of_week("2026-10-30"), Some("2026-10-26".to_string()));
        assert_eq!(monday_of_week("2026-11-01"), Some("2026-10-26".to_string()));
        assert_eq!(monday_of_week("2026-10-26"), Some("2026-10-26".to_string()));
        assert!(!is_valid_date_key("2026-13-01"));
        assert!(!is_valid_date_key("not-a-date"));
    }
}
