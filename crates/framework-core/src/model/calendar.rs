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
    /// ISO weekday numbers *not* worked, Monday 1 through Sunday 7; the
    /// built-in default is Saturday and Sunday (6, 7).
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
            calendar.pattern,
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
                nearest_weekday(year, u32::from(rule_month), u32::from(day), weekday as u32)
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
                let (quarter, period, period_start, period_end) = match self.pattern {
                    // Calendar months have no week blocks to read periods
                    // off: the period is the month itself, and the week is
                    // simply how many sevens fit since the year started.
                    WeekPattern::Months => self.month_period(date, start),
                    _ => {
                        let (quarter, period, period_start_week, period_weeks) =
                            self.quarter_period(week, weeks);
                        let period_start =
                            start + chrono::Duration::days((period_start_week - 1) as i64 * 7);
                        let period_end =
                            period_start + chrono::Duration::days(period_weeks as i64 * 7 - 1);
                        (quarter, period, period_start, period_end)
                    }
                };
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

    /// Quarter, period 1-12, and the period's bounds for a date under
    /// calendar months: the period is the fiscal month the date falls in,
    /// and its bounds are that calendar month's own.
    fn month_period(
        &self,
        date: NaiveDate,
        year_start: NaiveDate,
    ) -> (u32, u32, NaiveDate, NaiveDate) {
        let months = (date.year() - year_start.year()) * 12 + date.month() as i32
            - year_start.month() as i32;
        let period = months.rem_euclid(12) as u32 + 1;
        let quarter = (period - 1) / 3 + 1;
        let period_start = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
            .expect("a date's own month starts on its first");
        let period_end = last_day_of_month(date.year(), date.month());
        (quarter, period, period_start, period_end)
    }

    /// Quarter, period, and the period's week span for a 1-based week.
    /// Quarters are always thirteen weeks; the pattern only places the
    /// three periods inside each quarter. A 53rd week extends the year's
    /// last period. Never called under calendar months, which have no
    /// week blocks — see `month_period`.
    fn quarter_period(&self, week: u32, weeks: u32) -> (u32, u32, u32, u32) {
        let blocks = match self.pattern {
            WeekPattern::FourFourFive => [4, 4, 5],
            WeekPattern::FourFiveFour => [4, 5, 4],
            WeekPattern::FiveFourFour => [5, 4, 4],
            WeekPattern::Months => [4, 4, 5],
        };
        // A week pattern needs a year that is a whole number of weeks, and
        // validation refuses any rule that does not give one. A document
        // stored before it did can still hold a week past the year's last,
        // and evaluating it must answer rather than panic.
        let week = week.min(weeks.max(1));
        let quarter = ((week - 1) / 13 + 1).min(4);
        // Subtraction, not modulo: week 53 is Q4's fourteenth week, not
        // its first.
        let mut week_in_quarter = week - (quarter - 1) * 13;
        let last_stretch = weeks >= 53 && quarter == 4;
        let mut period_start = 1;
        for (index, block) in blocks.iter().enumerate() {
            let mut length = *block;
            if last_stretch && index == 2 {
                length += 1;
            }
            // The last block of a quarter is the catch-all: a week the
            // earlier blocks did not take belongs to it.
            if week_in_quarter <= length || index == 2 {
                return (
                    quarter,
                    (quarter - 1) * 3 + index as u32 + 1,
                    (quarter - 1) * 13 + period_start,
                    length,
                );
            }
            week_in_quarter -= length;
            period_start += length;
        }
        (quarter, quarter * 3, (quarter - 1) * 13 + 1, 13)
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
    pub fn add_workdays(&self, date: NaiveDate, n: i64) -> Option<NaiveDate> {
        let step = chrono::Duration::days(n.signum());
        let mut remaining = n.abs();
        let mut date = date;
        // Bound the walk. Seven days always hold at least one workday —
        // validation refuses a weekend covering the whole week — so a
        // sound calendar lands inside this many steps, and a shift of a
        // thousand years is a mistake rather than a question. Answering
        // nothing beats a formula that never returns.
        let mut budget = remaining
            .saturating_mul(7)
            .saturating_add(self.holidays.len() as i64)
            .saturating_add(7)
            .min(400_000);
        while remaining > 0 {
            if budget == 0 {
                return None;
            }
            budget -= 1;
            date = date.checked_add_signed(step)?;
            if self.is_workday(date) {
                remaining -= 1;
            }
        }
        Some(date)
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
    /// Retail period 1–12; under calendar months, the fiscal month 1–12.
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
    // Validation refuses an anchor day no month can hold, February 29
    // included. A calendar stored before it did must still evaluate, so
    // fall back to the month's last day rather than panicking.
    let month = month.clamp(1, 12);
    let anchor =
        NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| last_day_of_month(year, month));
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
    pattern: WeekPattern,
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
    let mut distinct: Vec<u8> = weekend.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() >= 7 {
        return Err(
            "‘weekend’ lists the days not worked, so it cannot cover all seven — a calendar needs at least one working day"
                .into(),
        );
    }
    match *year_end {
        YearEndRule::LastDayOfMonth => {
            if pattern != WeekPattern::Months {
                return Err(
                    "A 4-4-5, 4-5-4 or 5-4-4 calendar counts whole weeks, so its year must end on a weekday: ending on the last day of a month leaves a part week. Choose a last-weekday or nearest-weekday year end, or the calendar-months pattern."
                        .into(),
                );
            }
        }
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
            // A non-leap year on purpose: February 29 is not a day every
            // fiscal year holds, so it cannot anchor a year end.
            let last = last_day_of_month(2025, month as u32).day();
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
