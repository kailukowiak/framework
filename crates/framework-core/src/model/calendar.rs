//! Fiscal calendars: which dates belong to which year, and which days
//! the business counts.
//!
//! A calendar names the month a fiscal year starts on, the week pattern
//! its twelve periods follow, the rule that ends its year, which year
//! number the year is labelled with, and the days work happens on. The
//! month arithmetic of slices 1–3 is the special case this generalizes:
//! calendar months starting in January, labelled by the year they start
//! in — which is why those functions keep working untouched when no
//! calendar is named.
//!
//! Weekdays are ISO numbers throughout: Monday is 1, Sunday is 7. Retail
//! weeks always begin on the fiscal year's start date, so a year that
//! starts on a Sunday — every NRF year does — has Sunday-to-Saturday
//! weeks with no partial first week.
use crate::Id;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use ts_rs::TS;

/// Twelve periods as calendar months, or as 4/5-week blocks repeating per
/// quarter: 4-4-5, 4-5-4, 5-4-4. A 53-week year extends the last period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WeekPattern {
    Months,
    FourFourFive,
    FourFiveFour,
    FiveFourFour,
}

/// How a retail year ends. Calendar months end on their last day; a retail
/// year ends on a weekday the rule names: the last one of a month, or the
/// one nearest a month-day anchor. NRF 4-5-4 ends the Saturday nearest
/// January 31, which is what makes 2023 a 53-week year and 2024 a 52-week
/// one. Nearest cannot tie — two Saturdays are seven days apart, so one is
/// always strictly closer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum YearEndRule {
    LastDayOfMonth,
    LastWeekday { weekday: u8 },
    NearestWeekday { weekday: u8, month: u8, day: u8 },
}

/// Which calendar year a fiscal year is numbered with. A February-start
/// year running February 2025 to January 2026 is FY2025 by its start and
/// FY2026 by its end: the NRF labels its calendars by the start, most
/// companies and governments by the end. Relative answers — quarters,
/// weeks, year-to-date windows — never depend on this; only the year
/// number a date reports does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum YearLabel {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Calendar {
    pub id: Id,
    pub name: String,
    /// Month the fiscal year starts on, 1 for January through 12.
    pub fy_start: u8,
    pub pattern: WeekPattern,
    /// Meaningless for calendar months, which end on month ends.
    pub year_end: YearEndRule,
    pub year_label: YearLabel,
    /// ISO weekday numbers worked, Monday 1 through Sunday 7.
    pub weekend: Vec<u8>,
    /// Holidays as strict `YYYY-MM-DD` dates.
    pub holidays: Vec<String>,
}

impl Calendar {
    /// Calendar months starting in January, the behaviour every fiscal
    /// function had before calendars existed. Used when a formula names no
    /// calendar and the document sets no default.
    pub fn builtin_months() -> Calendar {
        Calendar {
            id: "builtin-calendar-months".into(),
            name: "Calendar months".into(),
            fy_start: 1,
            pattern: WeekPattern::Months,
            year_end: YearEndRule::LastDayOfMonth,
            year_label: YearLabel::End,
            weekend: vec![6, 7],
            holidays: Vec::new(),
        }
    }

    pub fn resolve(&self) -> Result<ResolvedCalendar, String> {
        ResolvedCalendar::parse(self)
    }
}

/// A calendar with its lists parsed and validated, ready for date math.
/// Resolved once per formula compile, then moved into the evaluation.
#[derive(Debug, Clone)]
pub struct ResolvedCalendar {
    pub fy_start: u32,
    pub pattern: WeekPattern,
    pub year_end: YearEndRule,
    pub year_label: YearLabel,
    weekend: [bool; 8],
    holidays: HashSet<NaiveDate>,
}

impl ResolvedCalendar {
    pub fn builtin_months() -> ResolvedCalendar {
        Calendar::builtin_months()
            .resolve()
            .expect("the built-in calendar is valid")
    }

    pub fn parse(calendar: &Calendar) -> Result<ResolvedCalendar, String> {
        validate_calendar(
            &calendar.name,
            calendar.fy_start,
            &calendar.year_end,
            &calendar.weekend,
            &calendar.holidays,
        )?;
        let mut weekend = [false; 8];
        for day in &calendar.weekend {
            weekend[*day as usize] = true;
        }
        let mut holidays = HashSet::new();
        for holiday in &calendar.holidays {
            holidays.insert(parse_calendar_date(holiday)?);
        }
        Ok(ResolvedCalendar {
            fy_start: calendar.fy_start as u32,
            pattern: calendar.pattern,
            year_end: calendar.year_end,
            year_label: calendar.year_label,
            weekend,
            holidays,
        })
    }

    /// The calendar year the fiscal year labelled `label` starts in.
    /// January-start years start in their own year under either labelling;
    /// otherwise end-labelled years start the year before.
    fn start_year(&self, label: i32) -> i32 {
        if self.fy_start > 1 && self.year_label == YearLabel::End {
            label - 1
        } else {
            label
        }
    }

    /// First and last dates of the fiscal year labelled `label`. Retail
    /// years tile the timeline: each starts the day after the last one
    /// ended, so three consecutive candidates always contain any date.
    pub fn year_bounds(&self, label: i32) -> (NaiveDate, NaiveDate) {
        let start = match self.pattern {
            WeekPattern::Months => {
                NaiveDate::from_ymd_opt(self.start_year(label), self.fy_start, 1)
                    .expect("validated month and day")
            }
            _ => self
                .retail_year_end(label - 1)
                .succ_opt()
                .expect("calendar dates are finite"),
        };
        let end = match self.pattern {
            WeekPattern::Months => {
                let next = if self.fy_start == 1 {
                    label + 1
                } else {
                    self.start_year(label) + 1
                };
                let next_start =
                    NaiveDate::from_ymd_opt(next, self.fy_start, 1).expect("validated month");
                next_start.pred_opt().expect("calendar dates are finite")
            }
            _ => self.retail_year_end(label),
        };
        (start, end)
    }

    /// Last date of a retail fiscal year: the year-end rule applied in the
    /// year's last fiscal month.
    fn retail_year_end(&self, label: i32) -> NaiveDate {
        // The last fiscal month is the calendar month before the start
        // month; it falls in the following calendar year whenever the
        // fiscal year does not start in January.
        let month = (self.fy_start + 10) % 12 + 1;
        let year = self.start_year(label) + i32::from(month < self.fy_start);
        match self.year_end {
            YearEndRule::LastDayOfMonth => last_day_of_month(year, month),
            YearEndRule::LastWeekday { weekday } => {
                last_weekday_of_month(year, month, weekday as u32)
            }
            YearEndRule::NearestWeekday {
                weekday,
                month: rule_month,
                day,
            } => {
                // The rule names its own anchor month — NRF's January 31 —
                // which sits in the same calendar year as the year's end.
                nearest_weekday(
                    year,
                    u32::from(rule_month),
                    u32::from(day),
                    weekday as u32,
                )
            }
        }
    }

    /// The fiscal year label, 1-based week, quarter, period, and period
    /// bounds for a date.
    pub fn locate(&self, date: NaiveDate) -> LocatedDate {
        let year = date.year();
        for label in [year - 1, year, year + 1] {
            let (start, end) = self.year_bounds(label);
            if date >= start && date <= end {
                let days = (date - start).num_days();
                let week = (days / 7 + 1) as u32;
                let weeks = ((end - start).num_days() + 1) as u32 / 7;
                let (quarter, period, period_start_week, period_weeks) =
                    self.quarter_period(week, weeks);
                let period_start =
                    start + chrono::Duration::days((period_start_week - 1) as i64 * 7);
                let period_end = period_start + chrono::Duration::days(period_weeks as i64 * 7 - 1);
                return LocatedDate {
                    year: label,
                    week,
                    weeks,
                    quarter,
                    period,
                    period_start,
                    period_end,
                    year_start: start,
                    year_end: end,
                };
            }
        }
        unreachable!("three consecutive fiscal years cover every date");
    }

    /// Quarter, period, and the period's week span for a 1-based week.
    /// Quarters are always thirteen weeks; the pattern only places the
    /// three periods inside each quarter. A 53rd week extends the year's
    /// last period.
    fn quarter_period(&self, week: u32, weeks: u32) -> (u32, u32, u32, u32) {
        match self.pattern {
            WeekPattern::Months => {
                // Periods are calendar months here; week-derived blocks do
                // not apply. Callers asking for weeks still get the week;
                // callers asking for periods use the month arithmetic.
                (0, 0, 0, 0)
            }
            _ => {
                let blocks = match self.pattern {
                    WeekPattern::FourFourFive => [4, 4, 5],
                    WeekPattern::FourFiveFour => [4, 5, 4],
                    WeekPattern::FiveFourFour => [5, 4, 4],
                    WeekPattern::Months => unreachable!(),
                };
                let quarter = ((week - 1) / 13 + 1).min(4);
                // Subtraction, not modulo: week 53 is Q4's fourteenth
                // week, not its first.
                let mut week_in_quarter = week - (quarter - 1) * 13;
                let last_stretch = weeks == 53 && quarter == 4;
                let mut period_start = 1;
                for (index, block) in blocks.iter().enumerate() {
                    let mut length = *block;
                    if last_stretch && index == 2 {
                        length += 1;
                    }
                    if week_in_quarter <= length {
                        let start = period_start;
                        return (
                            quarter,
                            (quarter - 1) * 3 + index as u32 + 1,
                            (quarter - 1) * 13 + start,
                            length,
                        );
                    }
                    week_in_quarter -= length;
                    period_start += length;
                }
                unreachable!("thirteen-week quarters cover weeks 1-13, fourteen in a long Q4");
            }
        }
    }

    /// Fiscal year number for a date under this calendar's labelling.
    pub fn fiscal_year(&self, date: NaiveDate) -> i32 {
        self.locate(date).year
    }

    /// Whether the business works a date: not a weekend day, not a holiday.
    pub fn is_workday(&self, date: NaiveDate) -> bool {
        !self.weekend[date.weekday().number_from_monday() as usize]
            && !self.holidays.contains(&date)
    }

    /// The date `n` business days from `date`, forward for positive `n`
    /// and back for negative. The start date is day zero, never counted —
    /// the Excel WORKDAY convention — so shifting by zero returns the date
    /// itself even on a weekend.
    pub fn add_workdays(&self, mut date: NaiveDate, n: i64) -> NaiveDate {
        let step = chrono::Duration::days(n.signum());
        let mut remaining = n.abs();
        while remaining > 0 {
            date += step;
            if self.is_workday(date) {
                remaining -= 1;
            }
        }
        date
    }

    /// Business days from `start` through `end` inclusive, negated when
    /// the end precedes the start.
    pub fn networkdays(&self, start: NaiveDate, end: NaiveDate) -> i64 {
        let (from, to, sign) = if end >= start {
            (start, end, 1)
        } else {
            (end, start, -1)
        };
        let mut count = 0;
        let mut date = from;
        while date <= to {
            if self.is_workday(date) {
                count += 1;
            }
            date = date.succ_opt().expect("calendar dates are finite");
        }
        sign * count
    }
}

/// A date placed in its fiscal year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatedDate {
    pub year: i32,
    pub week: u32,
    /// Weeks in the year: 52, or 53 in a long year.
    pub weeks: u32,
    pub quarter: u32,
    /// Retail period 1–12; zero under calendar months, which count months.
    pub period: u32,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub year_start: NaiveDate,
    pub year_end: NaiveDate,
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let (year, month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .expect("validated month")
        .pred_opt()
        .expect("calendar dates are finite")
}

fn last_weekday_of_month(year: i32, month: u32, weekday: u32) -> NaiveDate {
    let mut date = last_day_of_month(year, month);
    while date.weekday().number_from_monday() != weekday {
        date = date.pred_opt().expect("a month holds every weekday");
    }
    date
}

fn nearest_weekday(year: i32, month: u32, day: u32, weekday: u32) -> NaiveDate {
    let anchor = NaiveDate::from_ymd_opt(year, month, day).expect("validated anchor date");
    let current = anchor.weekday().number_from_monday();
    let forward = (weekday + 7 - current) % 7;
    let back = (current + 7 - weekday) % 7;
    if forward < back {
        anchor + chrono::Duration::days(forward as i64)
    } else {
        anchor - chrono::Duration::days(back as i64)
    }
}

fn parse_calendar_date(text: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| format!("‘{text}’ is not a YYYY-MM-DD date"))
}

/// The shared field validation for adding and updating a calendar, so both
/// operations refuse the same way.
pub fn validate_calendar(
    name: &str,
    fy_start: u8,
    year_end: &YearEndRule,
    weekend: &[u8],
    holidays: &[String],
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("A calendar needs a name".into());
    }
    if !(1..=12).contains(&fy_start) {
        return Err(
            "fy_start must be a whole month number from 1 to 12, with 1 meaning January".into(),
        );
    }
    for day in weekend {
        if !(1..=7).contains(day) {
            return Err(format!(
                "Weekend days are ISO weekday numbers from 1 (Monday) to 7 (Sunday); ‘{day}’ is not one"
            ));
        }
    }
    match *year_end {
        YearEndRule::LastDayOfMonth => {}
        YearEndRule::LastWeekday { weekday } => {
            if !(1..=7).contains(&weekday) {
                return Err(format!(
                    "Year-end weekdays are ISO weekday numbers from 1 (Monday) to 7 (Sunday); ‘{weekday}’ is not one"
                ));
            }
        }
        YearEndRule::NearestWeekday {
            weekday,
            month,
            day,
        } => {
            if !(1..=7).contains(&weekday) {
                return Err(format!(
                    "Year-end weekdays are ISO weekday numbers from 1 (Monday) to 7 (Sunday); ‘{weekday}’ is not one"
                ));
            }
            if !(1..=12).contains(&month) {
                return Err(format!(
                    "Year-end anchor months run 1 to 12; ‘{month}’ is not one"
                ));
            }
            let last = last_day_of_month(2024, month as u32).day();
            if day < 1 || u32::from(day) > last {
                return Err(format!(
                    "Year-end anchor days run 1 to {last} in month {month}; ‘{day}’ is not one"
                ));
            }
        }
    }
    for holiday in holidays {
        parse_calendar_date(holiday)?;
    }
    Ok(())
}
