//! Property tests for public parsers and path checks that handle untrusted input.
use armorer::{ci_policy::utc_day, safe_path};
use proptest::prelude::*;
use std::path::{Component, Path};

/// Paths built from hostile segments, plus arbitrary text.
fn path_text() -> impl Strategy<Value = String> {
    prop_oneof![
        2 => any::<String>(),
        4 => (
            prop::collection::vec(
                prop::sample::select(vec!["a", "b.txt", "..", ".", "", "x y", "λ", "a\\b", "c\u{7}", "-"]),
                0..5,
            ),
            prop::sample::select(vec!["", "/", "./", "../"]),
        )
            .prop_map(|(parts, lead)| format!("{lead}{}", parts.join("/"))),
    ]
}

proptest! {
    #[test]
    fn safe_path_never_panics_and_only_accepts_plain_relative_paths(input in path_text()) {
        let root = tempfile::tempdir().expect("temp dir");
        if let Ok(joined) = safe_path(root.path(), &input) {
            prop_assert!(joined.starts_with(root.path()));
            prop_assert!(!joined.components().any(|c| matches!(c, Component::ParentDir)));
            prop_assert!(!input.is_empty());
            prop_assert!(!input.contains('\\'));
            prop_assert!(!input.chars().any(char::is_control));
            prop_assert!(!Path::new(&input).is_absolute());
        }
    }
}

/// Independent civil-date arithmetic (Howard Hinnant's days_from_civil).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Independent inverse (Howard Hinnant's civil_from_days).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn iso((year, month, day): (i64, i64, i64)) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn last_day() -> i64 {
    days_from_civil(9999, 12, 31)
}

fn as_days(value: u64) -> i64 {
    i64::try_from(value).expect("day count fits")
}

/// Random years almost never hit the century rules or the range ends, so walk
/// every day of those years against the independent oracle.
#[test]
fn utc_day_is_exact_for_every_day_of_boundary_years() {
    for year in [1970, 1972, 1999, 2000, 2023, 2024, 2100, 2400, 9996, 9999] {
        for month in 1..=12 {
            for day in 1..=days_in_month(year, month) {
                let actual = utc_day(&iso((year, month, day))).expect("valid date");
                assert_eq!(
                    as_days(actual),
                    days_from_civil(year, month, day),
                    "{year}-{month}-{day}"
                );
            }
            let past = iso((year, month, days_in_month(year, month) + 1));
            assert!(utc_day(&past).is_err(), "{past}");
            assert!(
                utc_day(&iso((year, month, 0))).is_err(),
                "day zero in {year}-{month}"
            );
        }
    }
}

proptest! {
    #[test]
    fn utc_day_never_panics(
        value in prop_oneof![any::<String>(), "[0-9\\-]{0,12}", "[0-9]{4}-[0-9]{2}-[0-9]{2}"]
    ) {
        let _ = utc_day(&value);
    }

    #[test]
    fn utc_day_matches_independent_days_since_epoch(
        year in 1970_i64..=9999,
        month in 1_i64..=12,
        day_seed in 0_i64..31,
    ) {
        let day = 1 + day_seed % days_in_month(year, month);
        let actual = utc_day(&iso((year, month, day))).expect("valid date");
        prop_assert_eq!(as_days(actual), days_from_civil(year, month, day));
    }

    #[test]
    fn utc_day_is_strictly_increasing_day_to_day(n in 0_i64..last_day()) {
        let today = utc_day(&iso(civil_from_days(n))).expect("valid date");
        let tomorrow = utc_day(&iso(civil_from_days(n + 1))).expect("valid date");
        prop_assert_eq!(as_days(today), n);
        prop_assert_eq!(tomorrow, today + 1);
    }

    #[test]
    fn utc_day_rejects_impossible_dates(year in 1970_i64..=9999, bad in 0_u8..5) {
        let value = match bad {
            0 => iso((year, 13, 1)),
            1 => iso((year, 0, 1)),
            2 => iso((year, 2, 30)),
            3 => iso((year, 4, 31)),
            _ if is_leap(year) => iso((year, 2, 30)),
            _ => iso((year, 2, 29)),
        };
        prop_assert!(utc_day(&value).is_err(), "{value}");
    }

    #[test]
    fn utc_day_rejects_out_of_range_years(year in prop_oneof![0_i64..1970, Just(10_000_i64)]) {
        let january = format!("{year:04}-01-01");
        let december = format!("{year:04}-12-31");
        prop_assert!(utc_day(&january).is_err());
        prop_assert!(utc_day(&december).is_err());
    }
}
