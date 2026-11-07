//! Bounded iCalendar expansion for the native candidate.
mod zones;
use chrono::{
    DateTime, Datelike, Days, FixedOffset, NaiveDate, NaiveDateTime, SecondsFormat, TimeDelta,
    TimeZone, Utc,
};
use ical::{parser::ical::component::IcalEvent, property::Property, IcalParser};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Cursor,
    time::{Duration, Instant},
};

const INVALID: &str = "ICS_INVALID";
const COMPLEX: &str = "ICS_TOO_COMPLEX";
const ZONE: &str = "ICS_TIMEZONE_UNSUPPORTED";

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: u64 = 1793635200000; // 2026-11-03 00:00 Asia/Taipei

    #[test]
    fn embedded_observances_and_range_match_legacy_across_offset_change() {
        let text = [
            "BEGIN:VTIMEZONE",
            "TZID:Custom/Test",
            "BEGIN:STANDARD",
            "DTSTART:20251102T020000",
            "TZOFFSETFROM:+0300",
            "TZOFFSETTO:+0200",
            "RRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=1SU",
            "END:STANDARD",
            "BEGIN:DAYLIGHT",
            "DTSTART:20250302T020000",
            "TZOFFSETFROM:+0200",
            "TZOFFSETTO:+0300",
            "RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=1SU",
            "END:DAYLIGHT",
            "END:VTIMEZONE",
            "BEGIN:VEVENT",
            "UID:series",
            "DTSTART;TZID=Custom/Test:20261031T090000",
            "DURATION:PT1H",
            "RRULE:FREQ=DAILY;COUNT=5",
            "SUMMARY:base",
            "END:VEVENT",
            "BEGIN:VEVENT",
            "UID:series",
            "RECURRENCE-ID;TZID=Custom/Test;RANGE=THISANDFUTURE:20261102T090000",
            "DTSTART;TZID=Custom/Test:20261102T100000",
            "DURATION:PT2H",
            "SUMMARY:moved",
            "END:VEVENT",
            "BEGIN:VEVENT",
            "UID:series",
            "RECURRENCE-ID;TZID=Custom/Test:20261103T090000",
            "DTSTART;TZID=Custom/Test:20261103T110000",
            "DURATION:PT1H",
            "SUMMARY:once",
            "END:VEVENT",
        ]
        .join("\r\n");
        let parsed = parse(&calendar(&text), 1793376000000, "Asia/Taipei").unwrap();
        let got: Vec<_> = parsed["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (
                    v["title"].as_str().unwrap(),
                    v["start"].as_str().unwrap(),
                    v["end"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (
                    "base",
                    "2026-10-31T14:00:00.000+08:00",
                    "2026-10-31T15:00:00.000+08:00"
                ),
                (
                    "base",
                    "2026-11-01T15:00:00.000+08:00",
                    "2026-11-01T16:00:00.000+08:00"
                ),
                (
                    "moved",
                    "2026-11-02T16:00:00.000+08:00",
                    "2026-11-02T18:00:00.000+08:00"
                ),
                (
                    "once",
                    "2026-11-03T17:00:00.000+08:00",
                    "2026-11-03T18:00:00.000+08:00"
                ),
                (
                    "moved",
                    "2026-11-04T16:00:00.000+08:00",
                    "2026-11-04T18:00:00.000+08:00"
                ),
            ]
        );
    }

    #[test]
    fn explicit_periods_and_legacy_exrule_are_applied() {
        let text = [
            "BEGIN:VEVENT",
            "UID:series",
            "DTSTART:20261103T070000Z",
            "DURATION:PT1H",
            "RRULE:FREQ=DAILY;COUNT=4",
            "EXRULE:FREQ=DAILY;COUNT=2",
            "RDATE;VALUE=PERIOD:20261107T070000Z/PT2H,20261108T070000Z/20261108T100000Z",
            "END:VEVENT",
        ]
        .join("\r\n");
        let parsed = parse(&calendar(&text), NOW, "Asia/Taipei").unwrap();
        let events = parsed["events"].as_array().unwrap();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0]["start"], "2026-11-05T07:00:00.000Z");
        assert_eq!(events[2]["end"], "2026-11-07T09:00:00.000Z");
        assert_eq!(events[3]["end"], "2026-11-08T10:00:00.000Z");
    }

    fn calendar(events: &str) -> String {
        format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n{events}\r\nEND:VCALENDAR")
    }

    #[test]
    fn recurrence_exclusion_override_and_folded_titles_match_legacy_examples() {
        let master="BEGIN:VEVENT\r\nUID:one\r\nDTSTART:20261103T020000Z\r\nDTEND:20261103T030000Z\r\nSUMMARY:中文\r\n 測試\r\nRRULE:FREQ=DAILY;COUNT=4\r\nEXDATE:20261104T020000Z\r\nEND:VEVENT";
        let exception="BEGIN:VEVENT\r\nUID:one\r\nRECURRENCE-ID:20261105T020000Z\r\nDTSTART:20261105T040000Z\r\nDTEND:20261105T050000Z\r\nSUMMARY:改期\r\nEND:VEVENT";
        let text = calendar(&format!("{master}\r\n{exception}"));
        let parsed = parse(&text, NOW, "Asia/Taipei").unwrap();
        assert_eq!(parsed["events"].as_array().unwrap().len(), 3);
        assert_eq!(parsed["events"][0]["title"], "中文測試");
        assert_eq!(parsed["events"][1]["title"], "改期");
        assert_eq!(parsed["events"][1]["start"], "2026-11-05T04:00:00.000Z");
    }

    #[test]
    fn all_day_and_iana_times_preserve_zone_and_unknown_zones_fail() {
        let all_day="BEGIN:VEVENT\r\nUID:all\r\nDTSTART;VALUE=DATE:20261102\r\nDTEND;VALUE=DATE:20261104\r\nSUMMARY:全天\r\nEND:VEVENT";
        let iana="BEGIN:VEVENT\r\nUID:zone\r\nDTSTART;TZID=America/New_York:20261103T100000\r\nDURATION:PT1H\r\nEND:VEVENT";
        let text = calendar(&format!("{all_day}\r\n{iana}"));
        let parsed = parse(&text, NOW, "Asia/Taipei").unwrap();
        assert_eq!(
            parsed["events"][0]["start"],
            "2026-11-02T00:00:00.000+08:00"
        );
        assert_eq!(parsed["events"][0]["allDay"], true);
        assert_eq!(
            parsed["events"][1]["start"],
            "2026-11-03T10:00:00.000-05:00"
        );
        assert_eq!(parsed["events"][1]["end"], "2026-11-03T11:00:00.000-05:00");
        assert_eq!(
            parse(
                &text.replace("America/New_York", "Unknown/Zone"),
                NOW,
                "Asia/Taipei"
            )
            .unwrap_err(),
            ZONE
        );
    }
}

fn prop<'a>(p: &'a [Property], key: &str) -> Option<&'a Property> {
    p.iter().find(|p| p.name.eq_ignore_ascii_case(key))
}
fn value<'a>(p: &'a [Property], key: &str) -> Option<&'a str> {
    prop(p, key)?.value.as_deref()
}
fn param<'a>(p: &'a Property, key: &str) -> Option<&'a str> {
    p.params
        .as_ref()?
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))?
        .1
        .first()
        .map(String::as_str)
}

type Dt = DateTime<FixedOffset>;
fn local(tz: chrono_tz::Tz, date: NaiveDateTime) -> Result<DateTime<chrono_tz::Tz>, &'static str> {
    // Luxon advances a nonexistent local time by the DST gap.
    if let Some(t) = tz.from_local_datetime(&date).earliest() {
        return Ok(t);
    }
    let before = tz
        .from_local_datetime(&(date - TimeDelta::hours(24)))
        .earliest()
        .ok_or(ZONE)?;
    let after = tz
        .from_local_datetime(&(date + TimeDelta::hours(24)))
        .earliest()
        .ok_or(ZONE)?;
    use chrono::Offset;
    let gap = after.offset().fix().local_minus_utc() - before.offset().fix().local_minus_utc();
    tz.from_local_datetime(&(date + TimeDelta::seconds(i64::from(gap))))
        .earliest()
        .ok_or(ZONE)
}

#[derive(Clone, Copy)]
enum Zone<'a> {
    Named(chrono_tz::Tz),
    Embedded(&'a zones::Embedded),
}

impl Zone<'_> {
    fn wall(self, date: NaiveDateTime) -> Result<Dt, &'static str> {
        match self {
            Self::Named(tz) => Ok(local(tz, date)?.fixed_offset()),
            Self::Embedded(zone) => zone.at_wall(date),
        }
    }
    fn instant(self, date: NaiveDateTime) -> Result<Dt, &'static str> {
        match self {
            Self::Named(tz) => Ok(tz.from_utc_datetime(&date).fixed_offset()),
            Self::Embedded(zone) => zone.at_utc(date),
        }
    }
    fn recurrence_clock(self, date: Dt) -> Result<DateTime<rrule::Tz>, &'static str> {
        match self {
            Self::Named(tz) => Ok(date.with_timezone(&rrule::Tz::Tz(tz))),
            Self::Embedded(_) => Ok(zones::clock(self.instant(date.naive_utc())?.naive_local())),
        }
    }
    fn occurrence(self, date: DateTime<rrule::Tz>) -> Result<Dt, &'static str> {
        match self {
            Self::Named(_) => self.instant(date.naive_utc()),
            Self::Embedded(_) => self.wall(date.naive_utc()),
        }
    }
    fn shift(self, start: Dt, by: TimeDelta) -> Result<Dt, &'static str> {
        self.wall(start.naive_local().checked_add_signed(by).ok_or(INVALID)?)
    }
}

#[derive(Clone, Copy)]
struct Moment<'a> {
    date: Dt,
    zone: Zone<'a>,
    all_day: bool,
}

struct Parser {
    fallback: chrono_tz::Tz,
    embedded: HashMap<String, zones::Embedded>,
}

impl Parser {
    fn dt<'a>(
        &'a self,
        p: &Property,
        raw: &str,
        fallback: Zone<'a>,
    ) -> Result<Moment<'a>, &'static str> {
        let (date, zone, all_day) = if raw.len() == 8 {
            (
                NaiveDate::parse_from_str(raw, "%Y%m%d")
                    .map_err(|_| INVALID)?
                    .and_hms_opt(0, 0, 0)
                    .ok_or(INVALID)?,
                Zone::Named(self.fallback),
                true,
            )
        } else {
            let (raw, zone) = if let Some(utc) = raw.strip_suffix('Z') {
                (utc, Zone::Named(chrono_tz::UTC))
            } else if let Some(tz) = param(p, "TZID") {
                let zone = match self.embedded.get(tz) {
                    Some(z) => Zone::Embedded(z),
                    None => Zone::Named(tz.parse().map_err(|_| ZONE)?),
                };
                (raw, zone)
            } else {
                (raw, fallback)
            };
            (
                NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S").map_err(|_| INVALID)?,
                zone,
                false,
            )
        };
        Ok(Moment {
            date: zone.wall(date)?,
            zone,
            all_day,
        })
    }
    fn event(&self, event: &IcalEvent) -> Result<(Moment<'_>, Moment<'_>), &'static str> {
        let p = &event.properties;
        let s = prop(p, "DTSTART").ok_or(INVALID)?;
        let start = self.dt(
            s,
            s.value.as_deref().ok_or(INVALID)?,
            Zone::Named(self.fallback),
        )?;
        let end = if let Some(end) = prop(p, "DTEND") {
            self.dt(end, end.value.as_deref().ok_or(INVALID)?, start.zone)?
        } else {
            let duration = value(p, "DURATION")
                .map(duration_seconds)
                .transpose()?
                .unwrap_or(if start.all_day { 86400 } else { 0 });
            Moment {
                date: start
                    .zone
                    .shift(start.date, TimeDelta::try_seconds(duration).ok_or(INVALID)?)?,
                ..start
            }
        };
        Ok((start, end))
    }

    fn display(&self, t: Moment<'_>) -> Dt {
        match t.zone {
            Zone::Embedded(_) => t.date.with_timezone(&self.fallback).fixed_offset(),
            Zone::Named(_) => t.date,
        }
    }
}

fn rule(
    raw: &str,
    start: DateTime<rrule::Tz>,
    zone: Zone<'_>,
) -> Result<rrule::RRule, &'static str> {
    let raw = match zone {
        Zone::Embedded(_) => zones::wall_rule(raw, |utc| Ok(zone.instant(utc)?.naive_local()))?,
        Zone::Named(_) => raw.into(),
    };
    let rule = format!("RRULE:{raw}")
        .parse::<rrule::RRule<rrule::Unvalidated>>()
        .map_err(|_| INVALID)?;
    rule.validate(start).map_err(|_| INVALID)
}

fn duration_seconds(text: &str) -> Result<i64, &'static str> {
    use std::sync::OnceLock;
    static RULE: OnceLock<regex::Regex> = OnceLock::new();
    let r=RULE.get_or_init(||regex::Regex::new(r"^([+-])?P(?:(\d{1,7})W|(?:(\d{1,7})D)?(?:T(?:(\d{1,7})H)?(?:(\d{1,7})M)?(?:(\d{1,7})S)?)?)$").unwrap());
    let m = r.captures(text).ok_or(INVALID)?;
    if (2..=6).all(|i| m.get(i).is_none()) {
        return Err(INVALID);
    }
    let mut seconds = 0;
    for (i, scale) in [604800, 86400, 3600, 60, 1].into_iter().enumerate() {
        if let Some(part) = m.get(i + 2) {
            seconds += part.as_str().parse::<i64>().map_err(|_| INVALID)? * scale;
        }
    }
    Ok(if m.get(1).is_some_and(|x| x.as_str() == "-") {
        -seconds
    } else {
        seconds
    })
}

fn iso(dt: Dt) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn uid(e: &IcalEvent) -> &str {
    value(&e.properties, "UID").unwrap_or("event")
}
fn cancelled(e: &IcalEvent) -> bool {
    value(&e.properties, "STATUS").is_some_and(|s| s.eq_ignore_ascii_case("CANCELLED"))
}

pub(crate) fn parse(text: &str, now: u64, zone: &str) -> Result<Value, &'static str> {
    if text.len() > 1048576 {
        return Err("ICS_TOO_LARGE");
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut calendar = IcalParser::new(Cursor::new(text.as_bytes()));
    let c = calendar.next().ok_or(INVALID)?.map_err(|_| INVALID)?;
    if value(&c.properties, "VERSION").is_none() || calendar.next().is_some() {
        return Err(INVALID);
    }
    if c.events.len() > 3000 {
        return Err(COMPLEX);
    }
    let timezone = zone.parse().map_err(|_| ZONE)?;
    let date = Utc
        .timestamp_millis_opt(i64::try_from(now).map_err(|_| INVALID)?)
        .single()
        .ok_or(INVALID)?
        .with_timezone(&timezone)
        .date_naive();
    let midnight = |d: NaiveDate| {
        local(timezone, d.and_hms_opt(0, 0, 0).ok_or(INVALID)?).map(|d| d.fixed_offset())
    };
    let from = midnight(date)?;
    let until = midnight(date.checked_add_days(Days::new(7)).ok_or(INVALID)?)?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut budget = 50000usize;
    let event_year = c
        .events
        .iter()
        .flat_map(|e| &e.properties)
        .filter(|p| {
            matches!(
                p.name.as_str(),
                "DTSTART" | "DTEND" | "RECURRENCE-ID" | "RDATE" | "EXDATE"
            )
        })
        .filter_map(|p| p.value.as_deref())
        .flat_map(|v| v.split([',', '/']))
        .filter_map(|v| v.get(..4)?.parse::<i32>().ok())
        .max()
        .unwrap_or(date.year());
    let horizon = NaiveDate::from_ymd_opt((event_year.max(date.year()) + 2).min(9999), 12, 31)
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .ok_or(ZONE)?;
    let mut embedded = HashMap::new();
    if c.timezones.len() > 64 {
        return Err(COMPLEX);
    }
    let used_zones: HashSet<_> = c
        .events
        .iter()
        .flat_map(|e| &e.properties)
        .filter_map(|p| param(p, "TZID"))
        .collect();
    for zone in &c.timezones {
        let id = value(&zone.properties, "TZID").ok_or(ZONE)?;
        if used_zones.contains(id)
            && embedded
                .insert(
                    id.into(),
                    zones::Embedded::build(zone, horizon, &mut budget, deadline)?,
                )
                .is_some()
        {
            return Err(ZONE);
        }
    }
    let parser = Parser {
        fallback: timezone,
        embedded,
    };
    let mut events = Vec::new();
    let mut seen = HashSet::new();
    let mut masters = HashSet::new();
    let mut exceptions = HashMap::new();
    for e in &c.events {
        let Some(rid) = prop(&e.properties, "RECURRENCE-ID") else {
            masters.insert(uid(e));
            continue;
        };
        if param(rid, "RANGE").is_some_and(|v| !v.eq_ignore_ascii_case("THISANDFUTURE")) {
            return Err("ICS_RECURRENCE_UNSUPPORTED");
        }
        // Keep raw exceptions until the master supplies the floating timezone.
        exceptions
            .entry(uid(e))
            .or_insert_with(Vec::new)
            .push((rid, e));
    }
    let mut add = |e: &IcalEvent, start: Moment<'_>, end: Moment<'_>| {
        let all_day = start.all_day;
        let (start, end) = (parser.display(start), parser.display(end));
        if cancelled(e) || start >= until || end < from || (end == from && end > start) {
            return;
        }
        let start_text = iso(start);
        let id = format!("{}:{start_text}", uid(e));
        if !seen.insert(id.clone()) {
            return;
        }
        let title = value(&e.properties, "SUMMARY")
            .filter(|s| !s.is_empty())
            .unwrap_or("（未命名行程）");
        let mut unescaped = String::new();
        let mut chars = title.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                unescaped.push(c);
                continue;
            }
            match chars.next() {
                Some('n' | 'N') => unescaped.push('\n'),
                Some(c) => unescaped.push(c),
                None => unescaped.push('\\'),
            }
        }
        let mut count = 0;
        let title: String = unescaped
            .chars()
            .take_while(|c| {
                count += c.len_utf16();
                count <= 300
            })
            .collect();
        events.push((
            start.timestamp_millis(),
            json!({"id":id,"title":title,"start":start_text,"end":iso(end),"allDay":all_day}),
        ));
    };
    for e in &c.events {
        if Instant::now() > deadline {
            return Err(COMPLEX);
        }
        let p = &e.properties;
        if cancelled(e) || (prop(p, "RECURRENCE-ID").is_some() && masters.contains(uid(e))) {
            continue;
        }
        let (start, end) = parser.event(e)?;
        if !p
            .iter()
            .any(|p| ["RRULE", "RDATE", "EXRULE", "EXDATE"].contains(&p.name.as_str()))
        {
            add(e, start, end);
            continue;
        }
        let start_clock = start.zone.recurrence_clock(start.date)?;
        let mut set = rrule::RRuleSet::new(start_clock);
        let mut periods = HashMap::new();
        let mut has_rule = false;
        for p in p {
            match p.name.as_str() {
                "RRULE" => {
                    has_rule = true;
                    set = set.rrule(rule(
                        p.value.as_deref().ok_or(INVALID)?,
                        start_clock,
                        start.zone,
                    )?);
                }
                "EXRULE" => {
                    set = set.exrule(rule(
                        p.value.as_deref().ok_or(INVALID)?,
                        start_clock,
                        start.zone,
                    )?);
                }
                "RDATE" | "EXDATE" => {
                    for raw in p.value.as_deref().ok_or(INVALID)?.split(',') {
                        let (begin, finish) = raw
                            .split_once('/')
                            .map_or((raw, None), |(a, b)| (a, Some(b)));
                        let occurrence = parser.dt(p, begin, start.zone)?;
                        let date = start.zone.recurrence_clock(occurrence.date)?;
                        if let Some(finish) = finish {
                            if p.name != "RDATE" || occurrence.all_day {
                                return Err(INVALID);
                            }
                            let end = if finish.starts_with('P') {
                                Moment {
                                    date: occurrence.zone.shift(
                                        occurrence.date,
                                        TimeDelta::try_seconds(duration_seconds(finish)?)
                                            .ok_or(INVALID)?,
                                    )?,
                                    ..occurrence
                                }
                            } else {
                                parser.dt(p, finish, occurrence.zone)?
                            };
                            if end.date <= occurrence.date {
                                return Err(INVALID);
                            }
                            periods.insert(date.timestamp(), (occurrence, end));
                        }
                        set = if p.name == "RDATE" {
                            set.rdate(date)
                        } else {
                            set.exdate(date)
                        };
                    }
                }
                _ => {}
            }
        }
        if !has_rule {
            set = set.rdate(start_clock);
        }
        let mut overrides = HashMap::new();
        let mut ranges = BTreeMap::new();
        let mut lookahead = TimeDelta::zero();
        for (rid, item) in exceptions.get(uid(e)).into_iter().flatten() {
            let original = parser.dt(rid, rid.value.as_deref().ok_or(INVALID)?, start.zone)?;
            let key = start.zone.recurrence_clock(original.date)?.timestamp();
            overrides.insert(key, *item);
            if param(rid, "RANGE").is_some() {
                let range = if cancelled(item) {
                    None
                } else {
                    let (start, end) = parser.event(item)?;
                    let shift = start.date.naive_local() - original.date.naive_local();
                    lookahead = lookahead.max(-shift);
                    Some((start, shift, end.date - start.date))
                };
                ranges.insert(key, (*item, range));
            }
        }
        let limit = until.checked_add_signed(lookahead).ok_or(INVALID)?;
        let occurrences = set
            .before(start.zone.recurrence_clock(limit)?)
            .all((budget + 1) as u16);
        if occurrences.limited || Instant::now() > deadline {
            return Err(COMPLEX);
        }
        budget = budget.checked_sub(occurrences.dates.len()).ok_or(COMPLEX)?;
        for date in occurrences.dates {
            let key = date.timestamp();
            if let Some(item) = overrides.get(&key) {
                if !cancelled(item) {
                    let (s, e) = parser.event(item)?;
                    add(item, s, e);
                }
                continue;
            }
            let occurrence = start.zone.occurrence(date)?;
            if let Some((_, (item, range))) = ranges.range(..=key).next_back() {
                if let Some((model, shift, duration)) = range {
                    let wall = occurrence
                        .naive_local()
                        .checked_add_signed(*shift)
                        .ok_or(INVALID)?;
                    let shifted = Moment {
                        date: model.zone.wall(wall)?,
                        ..*model
                    };
                    let finish = Moment {
                        date: shifted.zone.shift(shifted.date, *duration)?,
                        ..shifted
                    };
                    add(item, shifted, finish);
                }
            } else if let Some((s, e2)) = periods.get(&key) {
                add(e, *s, *e2);
            } else {
                let occurrence = Moment {
                    date: occurrence,
                    ..start
                };
                let duration = if start.all_day {
                    end.date.naive_local() - start.date.naive_local()
                } else {
                    end.date - start.date
                };
                let finish = Moment {
                    date: occurrence.zone.shift(occurrence.date, duration)?,
                    ..occurrence
                };
                add(e, occurrence, finish);
            }
        }
    }
    events.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1["title"].as_str().cmp(&b.1["title"].as_str()))
    });
    let truncated = events.len() > 200;
    let events: Vec<_> = events.into_iter().take(200).map(|(_, v)| v).collect();
    Ok(
        json!({"events":events,"truncated":truncated,"from":iso(from),"until":iso(until),"zone":zone}),
    )
}
