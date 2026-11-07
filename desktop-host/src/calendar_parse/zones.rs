//! iCalendar observances, interpreted in wall time with the pre-transition offset.
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeDelta, TimeZone};
use ical::parser::ical::component::IcalTimeZone;
use std::{collections::BTreeMap, time::Instant};

use super::{value, COMPLEX, ZONE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Transition {
    utc: i64,
    before: i32,
    after: i32,
    standard: bool,
}

pub(super) struct Embedded {
    changes: Vec<Transition>,
}

fn offset(raw: &str) -> Result<i32, &'static str> {
    let bytes = raw.as_bytes();
    if !matches!(bytes.len(), 5 | 7) || !bytes[1..].iter().all(u8::is_ascii_digit) {
        return Err(ZONE);
    }
    let number = |i| i32::from((bytes[i] - b'0') * 10 + (bytes[i + 1] - b'0'));
    let (hours, minutes, seconds) = (
        number(1),
        number(3),
        if bytes.len() == 7 { number(5) } else { 0 },
    );
    if hours > 23 || minutes > 59 || seconds > 59 {
        return Err(ZONE);
    }
    let value = hours * 3600 + minutes * 60 + seconds;
    match bytes[0] {
        b'+' => Ok(value),
        b'-' if value != 0 => Ok(-value),
        _ => Err(ZONE),
    }
}

fn naive(raw: &str) -> Result<NaiveDateTime, &'static str> {
    NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S").map_err(|_| ZONE)
}

pub(super) fn clock(date: NaiveDateTime) -> DateTime<rrule::Tz> {
    rrule::Tz::UTC.from_utc_datetime(&date)
}

pub(super) fn wall_rule(
    raw: &str,
    utc_to_wall: impl FnOnce(NaiveDateTime) -> Result<NaiveDateTime, &'static str>,
) -> Result<String, &'static str> {
    let Some((start, field)) = raw.split_once("UNTIL=") else {
        return Ok(raw.into());
    };
    let (until, rest) = field.split_once(';').unwrap_or((field, ""));
    let Some(utc) = until.strip_suffix('Z') else {
        return Ok(raw.into());
    };
    let local = utc_to_wall(naive(utc)?)?;
    let suffix = if rest.is_empty() {
        String::new()
    } else {
        format!(";{rest}")
    };
    Ok(format!(
        "{start}UNTIL={}{suffix}",
        local.format("%Y%m%dT%H%M%SZ")
    ))
}

impl Embedded {
    pub(super) fn build(
        zone: &IcalTimeZone,
        horizon: NaiveDateTime,
        remaining: &mut usize,
        deadline: Instant,
    ) -> Result<Self, &'static str> {
        use ical::parser::ical::component::IcalTimeZoneTransitionType::STANDARD;
        let mut changes = BTreeMap::new();
        for observance in &zone.transitions {
            if Instant::now() > deadline {
                return Err(COMPLEX);
            }
            let p = &observance.properties;
            let from = offset(value(p, "TZOFFSETFROM").ok_or(ZONE)?)?;
            let to = offset(value(p, "TZOFFSETTO").ok_or(ZONE)?)?;
            let standard = matches!(observance.transition, STANDARD);
            let initial = naive(value(p, "DTSTART").ok_or(ZONE)?)?;
            let mut insert = |wall: NaiveDateTime, utc: bool| -> Result<(), &'static str> {
                *remaining = remaining.checked_sub(1).ok_or(COMPLEX)?;
                let utc = wall.and_utc().timestamp() - if utc { 0 } else { i64::from(from) };
                let change = Transition {
                    utc,
                    before: from,
                    after: to,
                    standard,
                };
                if changes.insert(utc, change).is_some_and(|old| old != change) {
                    return Err(ZONE);
                }
                Ok(())
            };
            insert(initial, false)?;
            for rdate in p.iter().filter(|p| p.name.eq_ignore_ascii_case("RDATE")) {
                for raw in rdate.value.as_deref().ok_or(ZONE)?.split(',') {
                    let (raw, utc) = raw.strip_suffix('Z').map_or((raw, false), |s| (s, true));
                    let date = if raw.len() == 8 {
                        chrono::NaiveDate::parse_from_str(raw, "%Y%m%d")
                            .map_err(|_| ZONE)?
                            .and_time(initial.time())
                    } else {
                        naive(raw)?
                    };
                    insert(date, utc)?;
                }
            }
            // Separate the iterator budget from the transition storage budget.
            if let Some(raw) = value(p, "RRULE") {
                let raw = wall_rule(raw, |utc| {
                    utc.checked_add_signed(TimeDelta::seconds(i64::from(from)))
                        .ok_or(ZONE)
                })?;
                let rule: rrule::RRule<rrule::Unvalidated> =
                    format!("RRULE:{raw}").parse().map_err(|_| ZONE)?;
                if !matches!(
                    rule.get_freq(),
                    rrule::Frequency::Yearly | rrule::Frequency::Monthly
                ) {
                    return Err(ZONE);
                }
                let start = clock(initial);
                let dates = rrule::RRuleSet::new(start)
                    .rrule(rule.validate(start).map_err(|_| ZONE)?)
                    .before(clock(horizon))
                    .all(4096);
                if dates.limited || Instant::now() > deadline {
                    return Err(COMPLEX);
                }
                for date in dates.dates {
                    insert(date.naive_utc(), false)?;
                }
            }
        }
        if changes.is_empty() {
            return Err(ZONE);
        }
        Ok(Self {
            changes: changes.into_values().collect(),
        })
    }

    fn at(&self, utc: i64) -> i32 {
        let i = self.changes.partition_point(|t| t.utc <= utc);
        self.changes.get(i.wrapping_sub(1)).map_or(0, |t| t.after)
    }

    pub(super) fn at_utc(&self, utc: NaiveDateTime) -> Result<DateTime<FixedOffset>, &'static str> {
        Ok(FixedOffset::east_opt(self.at(utc.and_utc().timestamp()))
            .ok_or(ZONE)?
            .from_utc_datetime(&utc))
    }

    pub(super) fn at_wall(
        &self,
        wall: NaiveDateTime,
    ) -> Result<DateTime<FixedOffset>, &'static str> {
        let naive = wall.and_utc().timestamp();
        // ICAL.js chooses the new offset for forward gaps and standard time
        // for backward overlaps. Before the first observance it uses UTC.
        let change = self
            .changes
            .iter()
            .rev()
            .find(|t| t.utc + i64::from(t.before.min(t.after)) <= naive);
        let offset = change.map_or(0, |t| {
            if t.after < t.before && !t.standard && naive < t.utc + i64::from(t.before) {
                t.before
            } else {
                t.after
            }
        });
        FixedOffset::east_opt(offset)
            .ok_or(ZONE)?
            .from_local_datetime(&wall)
            .single()
            .ok_or(ZONE)
    }
}
