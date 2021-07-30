extern crate chrono;

use std::env;
use std::fs;
use std::io::LineWriter;
use std::io::Write;

use chrono::prelude::*;

const DATE_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S %z";

#[derive(Clone, Copy)]
enum EventType {
    IN,
    OUT,
}

enum EventParseError {
    Discarded(String),
    Failed(String),
}

struct ParserState {
    errors: Vec<EventParseError>,
    lines_read: usize,
    parsed_events: usize,
}

struct ReportFile {
    path: String,
    lines: Vec<String>,
}

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
            fs::File::create(file_path);
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
        result
    }

    fn parse(&mut self, line: &str) {
        // For now, let this never fail
        let s = line.split(": ").collect::<Vec<&str>>();
        if s.len() != 2 {
            self.errors.push(EventParseError::Discarded(String::from(line)));
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
}

fn main() {
    let lines = vec![
        "time: 2021-03-14 21:29:49 +0000",
        "time: THIS WILL FAIL",
        "type: IN",
        "Discard this line please",
        "type: OUT",
        "time: 2021-03-17 21:29:49 +0000"
    ];
    let b = TimeReportEventBuilder::from_list(&lines);
    let tr = TimeReport { events: b };
    println!("{:#?}", tr.serialize());
    let event2 = TimeReportEvent::now(EventType::OUT);
    println!("{:#?}", event2.serialize());
    ReportFile::assert_exists();
    ReportFile::write_lines(&tr);

}
