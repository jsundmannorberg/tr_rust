extern crate chrono;

use std::env;
use std::fs;
use std::io::Write;

use std::cmp::Ordering;

use chrono::prelude::*;
use chrono::Duration;
use chrono::Datelike;

const DATE_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S %z";

#[derive(Clone, Copy, Eq, PartialEq)]
enum EventType {
    IN,
    OUT,
}

enum EventParseError {
    Discarded(String),
    Failed(String),
}

//struct ParserState {
//    errors: Vec<EventParseError>,
//    lines_read: usize,
//    parsed_events: usize,
//}

struct ReportFile {
    //path: String,
    //lines: Vec<String>,
}

#[derive(Eq, PartialEq)]
struct TimeReportEvent {
    event_type: EventType,
    time: DateTime<Utc>,
}

struct TimeReportEventBuilder {
    event_type: Option<EventType>,
    time: Option<DateTime<Utc>>,
    errors: Vec<EventParseError>,
}

struct TimeReport {
    events: Vec<TimeReportEvent>,
}

impl Ord for TimeReportEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl PartialOrd for TimeReportEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn format_minutes(minutes: i64) -> String {
    if minutes < 10 {
        format!("0{}", minutes)
    } else {
        format!("{}", minutes)
    }
}

impl ReportFile {

    fn get_lines() -> Vec<String> {
        let file_path = ReportFile::get_file_path().unwrap();
        let split = fs::read_to_string(file_path)
            .expect("Something went wrong reading the file");
        let strs = split.lines().map(|x| { String::from(x) });
        strs.collect::<Vec<String>>()
    }

    fn write_lines(report: &TimeReport) {
        let lines = report.serialize();
        let file_path = ReportFile::get_file_path().unwrap();
        println!("{}", file_path);
        ReportFile::assert_exists();
        let mut file = fs::File::create(file_path).unwrap();
        for line in lines {
            if let Err(e) = writeln!(file, "{}", line) {
                eprintln!("Couldn't write to file: {}", e);
            }
        }

    }

    fn get_file_path() -> Option<String> {
        match env::var("TIME_REPORT_PATH") {
            Ok(path) => Some(path),
            Err(_) => None
        }
    }

    fn assert_exists() {
        let file_path = match ReportFile::get_file_path() {
            Some(path) => path,
            None => {
                panic!("TIME_REPORT_PATH environment variable not found.");
            }
        };
        if !fs::metadata(&file_path).is_ok() {
            println!("File not found {}, creating a new file.", file_path);
            fs::File::create(file_path).unwrap();
        }

    }
}

impl EventType {
    fn to_str(&self) -> &'static str {
        match &self {
            EventType::IN => "IN",
            EventType::OUT => "OUT",
        }
    }

    fn parse(s: &str) -> Option<EventType> {
        match s {
            "IN" => Some(EventType::IN),
            "OUT" => Some(EventType::OUT),
            _ => None,
        }
    }
}

impl TimeReportEventBuilder {
    fn new() -> TimeReportEventBuilder {
        TimeReportEventBuilder {
            event_type: None,
            time: None,
            errors: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.event_type = None;
        self.time = None;
        self.errors = Vec::new();
    }

    fn from_list(list: &Vec<&str>) -> Vec<TimeReportEvent> {
        let mut result = Vec::new();
        let mut builder = TimeReportEventBuilder::new();
        for &line in list {
            builder.parse(line);
            match builder.get_if_done() {
                Some(event) => {
                    result.push(event);
                    builder.reset();
                },
                None => {}
            }
        }
        result.sort();
        result
    }

    fn discard(&mut self, line: &str) {
        self.errors.push(EventParseError::Discarded(String::from(line)));
    }

    fn parse(&mut self, line: &str) {
        // For now, let this never fail
        let s = line.split(": ").collect::<Vec<&str>>();
        if s.len() != 2 {
            self.discard(line);
            return;
        }
        match s[0] {
            "time" => {
                match DateTime::parse_from_str(s[1], DATE_FORMAT) {
                    Ok(dt) => {
                        self.time = Some(dt.with_timezone(&Utc));
                    },
                    _ => {
                        self.errors.push(EventParseError::Failed(String::from(line)));
                    }
                }
                
            },
            "type" => {
                self.event_type = EventType::parse(s[1]);
            },
            _ => {
                self.discard(line);
                println!("Discarded line: {}", line);
            }
        }
    }

    fn get_if_done(&self) -> Option<TimeReportEvent> {
        if self.event_type.is_none() || self.time.is_none() {
            return None
        }
        Some(TimeReportEvent {
            event_type: self.event_type.unwrap(),
            time: self.time.unwrap().clone(),
        })
    }
}

impl TimeReportEvent {
    fn now(event_type: EventType) -> TimeReportEvent {
        TimeReportEvent {
            event_type,
            time: Utc::now(),
        }
    }

    fn serialize(&self) -> Vec<String> {
        vec![
            format!("type: {}", &self.event_type.to_str()),
            format!("time: {}", &self.time.format(DATE_FORMAT).to_string()),
        ]
    }
}

impl TimeReport {
    fn serialize(&self) -> Vec<String> {
        self.events.iter().flat_map(|event| event.serialize()).collect()
    }

    fn add_event(&mut self, event_type: EventType) {
        self.events.push(TimeReportEvent::now(event_type));
        self.events.sort();
    }

    fn today<'a>(&'a self) -> Vec<&'a TimeReportEvent> {
        self.events.iter().filter(|event| event.time.date() == Utc::now().date()).collect()
    }

    fn events_in_day<'a>(&'a self, date: &Date<Utc>) -> Vec<&'a TimeReportEvent> {
        self.events.iter().filter(|event| event.time.date() == *date).collect()
    }

    fn days_this_week(&self) -> Vec<Date<Utc>> {
        let today = Utc::now().date();
        let weekday: Weekday = Utc::now().date().weekday();
        let days_from_monday = weekday.num_days_from_monday();
        (0..days_from_monday+1).map(|day| today - Duration::days(days_from_monday as i64 - day as i64)).collect()

    }

    fn total_time(&self, events: &Vec<&TimeReportEvent>) -> Duration {
        if events.len() == 0 {
            return Duration::zero()
        }
        let mut current_event = events[0];
        let mut current_result = current_event.time.signed_duration_since(current_event.time);
        for event in events {
            if event.event_type == EventType::OUT {
                if current_event.event_type == EventType::IN {
                    current_result = current_result + event.time.signed_duration_since(current_event.time);
                }
                current_event = event;
            } else {
                if current_event.event_type != EventType::IN {
                    current_event = event;
                }
            }
        }
        if current_event.event_type != EventType::OUT {
            current_result = current_result  + TimeReportEvent::now(EventType::OUT).time.signed_duration_since(current_event.time);
        }
        current_result
    }

    fn print_duration(&self, duration: &Duration) {
        println!("{}:{}", duration.num_hours(), format_minutes(duration.num_minutes()%60));
    }

    fn print_today(&self) {
        let today = self.today();
        for event in &today {
            println!("{:#?}", event.serialize());
        }
        //let total = self.total_time(&today);
        //self.print_duration(&total);
        //println!("{:#?}", self.days_this_week());
        self.print_week();
    }

    fn print_week(&self) {
        self.days_this_week().iter().for_each(|day| self.print_duration(&self.total_time(&self.events_in_day(day))));
    }
}

fn main() {
    let lines = ReportFile::get_lines();
    let b = TimeReportEventBuilder::from_list(&lines.iter().map(|x| &x[..]).collect::<Vec<&str>>());
    let mut tr = TimeReport { events: b };
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let event_type = match &args[1].to_ascii_uppercase()[..] {
            "IN" => EventType::IN,
            _ => EventType::OUT,
        };
        &tr.add_event(event_type);
        ReportFile::write_lines(&tr);
    }
    tr.print_today();
}
