use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Representation de datas HTTP (RFC 7231 / IMF-fixdate).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpDate {
    seconds: i64,
}

impl From<SystemTime> for HttpDate {
    fn from(value: SystemTime) -> Self {
        let seconds = match value.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs() as i64,
            Err(err) => -(err.duration().as_secs() as i64),
        };
        HttpDate { seconds }
    }
}

impl fmt::Display for HttpDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day, weekday, hour, minute, second) = breakdown(self.seconds);
        write!(
            f,
            "{weekday}, {day:02} {month} {year:04} {hour:02}:{minute:02}:{second:02} GMT",
            weekday = WEEKDAYS[weekday as usize],
            day = day,
            month = MONTHS[month as usize - 1],
            year = year,
            hour = hour,
            minute = minute,
            second = second
        )
    }
}

const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_MINUTE: i64 = 60;
const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn breakdown(seconds: i64) -> (i32, u32, u32, u32, u32, u32, u32) {
    let days = div_floor(seconds, SECONDS_PER_DAY);
    let secs_of_day = seconds - days * SECONDS_PER_DAY;

    let hour = (secs_of_day / SECONDS_PER_HOUR) as u32;
    let minute = ((secs_of_day % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE) as u32;
    let second = (secs_of_day % SECONDS_PER_MINUTE) as u32;

    let weekday = ((days + 3).rem_euclid(7)) as u32; // 1970-01-01 = Thursday

    let (year, month, day) = civil_from_days(days + 719_468); // convert to proleptic Gregorian

    (year, month, day, weekday, hour, minute, second)
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let mut z = days;
    let era = div_floor(z, 146_097);
    z -= era * 146_097;
    let doe = (z - z / 1_460 + z / 36_524 - z / 146_096) as i64;
    let yoe = (doe * 400 + 591) / 146_097;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn div_floor(a: i64, b: i64) -> i64 {
    let mut result = a / b;
    let rem = a % b;
    if (rem > 0 && b < 0) || (rem < 0 && b > 0) {
        result -= 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn formats_epoch() {
        let date = HttpDate::from(UNIX_EPOCH);
        assert_eq!(date.to_string(), "Thu, 01 Jan 1970 00:00:00 GMT");
    }

    #[test]
    fn formats_known_value() {
        let ts = UNIX_EPOCH + Duration::from_secs(784_111_417);
        let date = HttpDate::from(ts);
        assert_eq!(date.to_string(), "Sat, 06 Nov 1994 08:56:57 GMT");
    }
}
