//! Upcoming meetings from a private iCal (ICS) feed, e.g. Google Calendar's "secret address in iCal format".
//! Read-only: Cornflake never writes to the calendar. The feed URL is a secret and lives in Credential Manager.

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use serde::Serialize;
use std::collections::HashMap;

pub const ICS_SECRET: &str = "calendar:ics";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CalendarEvent {
    pub uid: String,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub attendees: Vec<String>,
    pub location: Option<String>,
    pub video_url: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct Prop {
    params: HashMap<String, String>,
    value: String,
}

#[derive(Debug, Default)]
struct RawEvent {
    props: Vec<(String, Prop)>,
}

impl RawEvent {
    fn get(&self, name: &str) -> Option<&Prop> {
        self.props.iter().find(|(n, _)| n == name).map(|(_, p)| p)
    }
    fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Prop> + 'a {
        self.props.iter().filter(move |(n, _)| n == name).map(|(_, p)| p)
    }
}

/// RFC 5545 line unfolding: a line starting with a space or tab continues the previous one.
fn unfold(ics: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in ics.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if (line.starts_with(' ') || line.starts_with('\t')) && !out.is_empty() {
            out.last_mut().unwrap().push_str(&line[1..]);
        } else {
            out.push(line.to_string());
        }
    }
    out
}

fn parse_line(line: &str) -> Option<(String, Prop)> {
    // The value starts after the first colon that is not inside a quoted parameter
    let mut in_quotes = false;
    let split = line.char_indices().find(|(_, c)| {
        if *c == '"' {
            in_quotes = !in_quotes;
        }
        *c == ':' && !in_quotes
    })?;
    let (head, value) = (&line[..split.0], &line[split.0 + 1..]);
    let mut parts = head.split(';');
    let name = parts.next()?.to_ascii_uppercase();
    let params = parts
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_ascii_uppercase(), v.trim_matches('"').to_string()))
        .collect();
    Some((name, Prop { params, value: value.to_string() }))
}

fn unescape(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\N", "\n").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}

fn parse_events(ics: &str) -> Vec<RawEvent> {
    let mut events = Vec::new();
    let mut current: Option<RawEvent> = None;
    for line in unfold(ics) {
        match line.as_str() {
            "BEGIN:VEVENT" => current = Some(RawEvent::default()),
            "END:VEVENT" => {
                if let Some(e) = current.take() {
                    events.push(e);
                }
            }
            _ => {
                if let (Some(e), Some(p)) = (current.as_mut(), parse_line(&line)) {
                    e.props.push(p);
                }
            }
        }
    }
    events
}

/// Returns None for all-day dates; they are not meetings to take notes in.
fn to_utc(p: &Prop) -> Option<DateTime<Utc>> {
    if p.params.get("VALUE").map_or(false, |v| v == "DATE") || p.value.len() == 8 {
        return None;
    }
    let v = p.value.trim();
    if let Some(stripped) = v.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S").ok()?;
        return Some(Utc.from_utc_datetime(&naive));
    }
    let naive = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?;
    match p.params.get("TZID").and_then(|tz| tz.parse::<chrono_tz::Tz>().ok()) {
        Some(tz) => tz.from_local_datetime(&naive).earliest().map(|d| d.with_timezone(&Utc)),
        None => chrono::Local.from_local_datetime(&naive).earliest().map(|d| d.with_timezone(&Utc)),
    }
}

fn video_url(text: &str) -> Option<String> {
    let hosts = ["meet.google.com/", "zoom.us/j/", "teams.microsoft.com/l/meetup-join", "whereby.com/", "app.slack.com/huddle"];
    text.split(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"')
        .find(|w| w.starts_with("http") && hosts.iter().any(|h| w.contains(h)))
        .map(|w| w.trim_end_matches(['.', ',', ')']).to_string())
}

fn base_event(e: &RawEvent) -> Option<CalendarEvent> {
    let start = to_utc(e.get("DTSTART")?)?;
    let end = e.get("DTEND").and_then(to_utc).unwrap_or(start + Duration::minutes(30));
    let attendees = e
        .all("ATTENDEE")
        .filter(|a| a.params.get("PARTSTAT").map_or(true, |s| s != "DECLINED"))
        .map(|a| {
            a.params
                .get("CN")
                .cloned()
                .unwrap_or_else(|| a.value.trim_start_matches("mailto:").trim_start_matches("MAILTO:").to_string())
        })
        .collect();
    let location = e.get("LOCATION").map(|p| unescape(&p.value)).filter(|l| !l.trim().is_empty());
    let text = format!(
        "{} {}",
        location.clone().unwrap_or_default(),
        e.get("DESCRIPTION").map(|p| unescape(&p.value)).unwrap_or_default()
    );
    Some(CalendarEvent {
        uid: e.get("UID").map(|p| p.value.clone()).unwrap_or_default(),
        title: e.get("SUMMARY").map(|p| unescape(&p.value)).unwrap_or_else(|| "Untitled event".into()),
        start,
        end,
        attendees,
        location,
        video_url: video_url(&text),
    })
}

/// Occurrence starts of a recurring event within the window, using the rrule crate for RRULE and EXDATE.
fn occurrences(e: &RawEvent, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<DateTime<Utc>> {
    let (Some(dtstart), Some(rrule)) = (e.get("DTSTART"), e.get("RRULE")) else { return vec![] };
    let mut spec = String::new();
    let tz = dtstart.params.get("TZID").map(|t| format!(";TZID={t}")).unwrap_or_default();
    spec.push_str(&format!("DTSTART{tz}:{}\nRRULE:{}\n", dtstart.value, rrule.value));
    for ex in e.all("EXDATE") {
        let tz = ex.params.get("TZID").map(|t| format!(";TZID={t}")).unwrap_or_default();
        spec.push_str(&format!("EXDATE{tz}:{}\n", ex.value));
    }
    let Ok(set) = spec.parse::<rrule::RRuleSet>() else { return vec![] };
    let after = from.with_timezone(&rrule::Tz::UTC);
    let before = to.with_timezone(&rrule::Tz::UTC);
    set.after(after).before(before).all(200).dates.into_iter().map(|d| d.with_timezone(&Utc)).collect()
}

/// Expands the feed into concrete events overlapping [from, to], sorted by start.
pub fn events_between(ics: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<CalendarEvent> {
    let raw = parse_events(ics);
    // Modified single occurrences of a series (RECURRENCE-ID) replace the generated ones
    let mut overrides: HashMap<(String, DateTime<Utc>), &RawEvent> = HashMap::new();
    for e in &raw {
        if let (Some(uid), Some(rid)) = (e.get("UID"), e.get("RECURRENCE-ID").and_then(to_utc)) {
            overrides.insert((uid.value.clone(), rid), e);
        }
    }
    let mut out = Vec::new();
    for e in &raw {
        if e.get("RECURRENCE-ID").is_some() || e.get("STATUS").map_or(false, |s| s.value == "CANCELLED") {
            continue;
        }
        let Some(base) = base_event(e) else { continue };
        if e.get("RRULE").is_none() {
            if base.end > from && base.start < to {
                out.push(base);
            }
            continue;
        }
        let length = base.end - base.start;
        for start in occurrences(e, from - length, to) {
            match overrides.get(&(base.uid.clone(), start)) {
                Some(o) if o.get("STATUS").map_or(false, |s| s.value == "CANCELLED") => {}
                Some(o) => {
                    if let Some(ev) = base_event(o) {
                        out.push(ev);
                    }
                }
                None => out.push(CalendarEvent { start, end: start + length, ..base.clone() }),
            }
        }
    }
    out.retain(|e| e.end > from && e.start < to);
    out.sort_by_key(|e| e.start);
    out
}

pub async fn fetch(url: &str) -> Result<String, String> {
    let url = url.trim().replacen("webcal://", "https://", 1);
    if !url.starts_with("https://") {
        return Err("The calendar address must start with https:// or webcal://".into());
    }
    let resp = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("could not reach the calendar: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("calendar returned {}", resp.status()));
    }
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if !body.contains("BEGIN:VCALENDAR") {
        return Err("That address did not return a calendar (iCal) file.".into());
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICS: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\n\
BEGIN:VEVENT\r\nUID:weekly-1\r\nDTSTART;TZID=Europe/Berlin:20261005T100000\r\nDTEND;TZID=Europe/Berlin:20261005T103000\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO,WE\r\nEXDATE;TZID=Europe/Berlin:20261007T100000\r\nSUMMARY:Nordlicht standup\r\n\
ATTENDEE;CN=Katrin Weber;PARTSTAT=ACCEPTED:mailto:katrin@example.com\r\n\
ATTENDEE;CN=\"Stefan, CFO\";PARTSTAT=DECLINED:mailto:stefan@example.com\r\n\
DESCRIPTION:Join: https://meet.google.com/abc-defg-hij\\nThanks\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:weekly-1\r\nRECURRENCE-ID;TZID=Europe/Berlin:20261012T100000\r\n\
DTSTART;TZID=Europe/Berlin:20261012T140000\r\nDTEND;TZID=Europe/Berlin:20261012T143000\r\nSUMMARY:Nordlicht standup (moved)\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:once\r\nDTSTART:20261008T150000Z\r\nDTEND:20261008T160000Z\r\nSUMMARY:Investor call with Atlas\r\n LP update\r\n\
LOCATION:https://zoom.us/j/123456\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:allday\r\nDTSTART;VALUE=DATE:20261008\r\nSUMMARY:Holiday\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:cancelled\r\nDTSTART:20261008T090000Z\r\nSUMMARY:Cancelled\r\nSTATUS:CANCELLED\r\nEND:VEVENT\r\n\
END:VCALENDAR\r\n";

    #[test]
    fn expands_recurrence_with_exdate_override_and_timezone() {
        let from = Utc.with_ymd_and_hms(2026, 10, 6, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 10, 13, 0, 0, 0).unwrap();
        let ev = events_between(ICS, from, to);
        let titles: Vec<_> = ev.iter().map(|e| e.title.as_str()).collect();
        // Wed 7 Oct is excluded, Mon 12 Oct moved to 14:00 Berlin; all-day and cancelled events are skipped
        assert_eq!(titles, vec!["Investor call with AtlasLP update", "Nordlicht standup (moved)"]);
        assert_eq!(ev[1].start, Utc.with_ymd_and_hms(2026, 10, 12, 12, 0, 0).unwrap());
        assert_eq!(ev[0].video_url.as_deref(), Some("https://zoom.us/j/123456"));
    }

    #[test]
    fn attendees_exclude_declined_and_keep_quoted_names() {
        let from = Utc.with_ymd_and_hms(2026, 10, 5, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 10, 6, 0, 0, 0).unwrap();
        let ev = events_between(ICS, from, to);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].start, Utc.with_ymd_and_hms(2026, 10, 5, 8, 0, 0).unwrap());
        assert_eq!(ev[0].attendees, vec!["Katrin Weber"]);
        assert_eq!(ev[0].video_url.as_deref(), Some("https://meet.google.com/abc-defg-hij"));
    }
}
