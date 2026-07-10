use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn utc_timestamp_slug() -> String {
    let timestamp = current_utc_timestamp();
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        timestamp.year,
        timestamp.month,
        timestamp.day,
        timestamp.hour,
        timestamp.minute,
        timestamp.second
    )
}

pub(crate) fn utc_iso_timestamp() -> String {
    let timestamp = current_utc_timestamp();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        timestamp.year,
        timestamp.month,
        timestamp.day,
        timestamp.hour,
        timestamp.minute,
        timestamp.second
    )
}

#[derive(Debug, Eq, PartialEq)]
struct UtcTimestamp {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

fn current_utc_timestamp() -> UtcTimestamp {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    utc_timestamp_from_unix_seconds(seconds)
}

fn utc_timestamp_from_unix_seconds(seconds: i64) -> UtcTimestamp {
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    UtcTimestamp {
        year,
        month,
        day,
        hour: (day_seconds / 3_600) as u32,
        minute: ((day_seconds % 3_600) / 60) as u32,
        second: (day_seconds % 60) as u32,
    }
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era as i32 + (era as i32) * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i32::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_unix_epoch_boundaries() {
        assert_eq!(
            utc_timestamp_from_unix_seconds(0),
            UtcTimestamp {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            }
        );
        assert_eq!(
            utc_timestamp_from_unix_seconds(1_704_067_199),
            UtcTimestamp {
                year: 2023,
                month: 12,
                day: 31,
                hour: 23,
                minute: 59,
                second: 59,
            }
        );
        assert_eq!(
            utc_timestamp_from_unix_seconds(1_704_067_200),
            UtcTimestamp {
                year: 2024,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            }
        );
    }
}
