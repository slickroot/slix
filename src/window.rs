use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};

#[derive(Debug, PartialEq)]
pub struct Window {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl Window {
    pub fn yesterday<Tz: TimeZone>(now: DateTime<Tz>) -> Window {
        let today = now.date_naive();
        Window {
            start: start_of_day(&now.timezone(), today.pred_opt().unwrap()),
            end: start_of_day(&now.timezone(), today),
        }
    }

    pub fn today<Tz: TimeZone>(now: DateTime<Tz>) -> Window {
        let today = now.date_naive();
        Window {
            start: start_of_day(&now.timezone(), today),
            end: now.with_timezone(&Utc),
        }
    }

    pub fn contains(&self, at: &DateTime<Utc>) -> bool {
        self.start <= *at && *at < self.end
    }
}

fn start_of_day<Tz: TimeZone>(zone: &Tz, date: NaiveDate) -> DateTime<Utc> {
    let midnight = date.and_hms_opt(0, 0, 0).unwrap();
    zone.from_local_datetime(&midnight)
        .earliest()
        .or_else(|| {
            zone.from_local_datetime(&(midnight + Duration::hours(1)))
                .earliest()
        })
        .unwrap()
        .with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::Tz;

    fn at(zone: Tz, y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Tz> {
        zone.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn normal_day_spans_local_midnight_to_midnight() {
        let zone = Tz::Europe__Paris;
        let window = Window::yesterday(at(zone, 2026, 3, 10, 15, 30));

        assert_eq!(window.start, at(zone, 2026, 3, 9, 0, 0).with_timezone(&Utc));
        assert_eq!(window.end, at(zone, 2026, 3, 10, 0, 0).with_timezone(&Utc));
    }

    #[test]
    fn just_after_midnight_still_means_the_previous_day() {
        let zone = Tz::Europe__Paris;
        let window = Window::yesterday(at(zone, 2026, 3, 10, 0, 1));

        assert_eq!(window.start, at(zone, 2026, 3, 9, 0, 0).with_timezone(&Utc));
        assert_eq!(window.end, at(zone, 2026, 3, 10, 0, 0).with_timezone(&Utc));
    }

    #[test]
    fn spring_forward_day_is_23_hours() {
        let zone = Tz::Europe__Paris;
        let window = Window::yesterday(at(zone, 2026, 3, 30, 12, 0));

        assert_eq!(window.end - window.start, Duration::hours(23));
    }

    #[test]
    fn fall_back_day_is_25_hours() {
        let zone = Tz::Europe__Paris;
        let window = Window::yesterday(at(zone, 2026, 10, 26, 12, 0));

        assert_eq!(window.end - window.start, Duration::hours(25));
    }

    #[test]
    fn day_whose_midnight_does_not_exist_starts_at_first_valid_instant() {
        let zone = Tz::America__Sao_Paulo;
        let window = Window::yesterday(at(zone, 2018, 11, 5, 12, 0));

        assert_eq!(
            window.start,
            at(zone, 2018, 11, 4, 1, 0).with_timezone(&Utc)
        );
        assert_eq!(window.end, at(zone, 2018, 11, 5, 0, 0).with_timezone(&Utc));
    }

    #[test]
    fn today_spans_local_midnight_to_now() {
        let zone = Tz::Europe__Paris;
        let now = at(zone, 2026, 3, 10, 15, 30);
        let window = Window::today(now);

        assert_eq!(
            window.start,
            at(zone, 2026, 3, 10, 0, 0).with_timezone(&Utc)
        );
        assert_eq!(window.end, now.with_timezone(&Utc));
    }

    #[test]
    fn just_after_midnight_today_is_a_short_window() {
        let zone = Tz::Europe__Paris;
        let now = at(zone, 2026, 3, 10, 0, 1);
        let window = Window::today(now);

        assert_eq!(
            window.start,
            at(zone, 2026, 3, 10, 0, 0).with_timezone(&Utc)
        );
        assert_eq!(window.end, now.with_timezone(&Utc));
    }

    #[test]
    fn contains_is_true_at_start() {
        let window = Window::yesterday(at(Tz::Europe__Paris, 2026, 3, 10, 15, 30));

        assert!(window.contains(&window.start));
    }

    #[test]
    fn contains_is_false_at_end() {
        let window = Window::yesterday(at(Tz::Europe__Paris, 2026, 3, 10, 15, 30));

        assert!(!window.contains(&window.end));
    }

    #[test]
    fn local_midnight_belongs_to_today_and_not_yesterday() {
        let zone = Tz::Europe__Paris;
        let now = at(zone, 2026, 3, 10, 15, 30);
        let midnight = at(zone, 2026, 3, 10, 0, 0).with_timezone(&Utc);

        assert!(Window::today(now).contains(&midnight));
        assert!(!Window::yesterday(now).contains(&midnight));
    }
}
