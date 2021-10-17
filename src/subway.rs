extern crate chrono;
extern crate prost;
extern crate reqwest;

use crate::drawing;
use crate::result;
use crate::webclient_api;

pub struct ProcessedData {
    pub upcoming_trains: Vec<(i64, String)>,
    pub upcoming_outbound_trains: Vec<(i64, String)>,
    pub big_countdown: Option<String>,
    pub big_countdown_line: Option<String>,
    pub station_name: String,
    pub data_timestamp: i64,
}

impl ProcessedData {
    pub fn empty() -> ProcessedData {
        return ProcessedData{
            upcoming_trains: vec![],
            upcoming_outbound_trains: vec![],
            big_countdown: None,
            big_countdown_line: None,
            station_name: "".to_string(),
            data_timestamp: 0,
        };
    }
}

pub fn fetch_and_process_data() -> result::TTDashResult<ProcessedData> {
    let raw_data = fetch_data()?;
    return process_data(&raw_data);
}

fn fetch_data() -> result::TTDashResult<webclient_api::StationStatus> {
    use prost::Message;

    let url = format!("http://traintrack.nyc/api/station/028").to_string();
    let mut response = reqwest::blocking::get(&url)?;
    let mut response_body = vec![];
    use std::io::Read;
    response.read_to_end(&mut response_body)?;
    let proto = webclient_api::StationStatus::decode(response_body.as_slice())?;
    return Ok(proto);
}

fn process_data(data: &webclient_api::StationStatus) -> result::TTDashResult<ProcessedData> {
    let mut arrivals: Vec<(i64, String)> = vec![];
    let mut outbound_arrivals: Vec<(i64, String)> = vec![];
    let now = chrono::Utc::now().timestamp();
    for line in &data.line {
        match line.direction() {
            webclient_api::Direction::Uptown => {
                for arrival in &line.arrivals {
                    if arrival.timestamp() > now {
                        arrivals.push((arrival.timestamp(), line.line().to_string()));
                    }
                }
            },
            webclient_api::Direction::Downtown => {
                for arrival in &line.arrivals {
                    if arrival.timestamp() > now {
                        outbound_arrivals.push((arrival.timestamp(), line.line().to_string()));
                    }
                }
            },
        }
    }

    if arrivals.len() == 0 {
        return Ok(ProcessedData::empty());
    } else {
        arrivals.sort_by_key(|x| x.0);
        outbound_arrivals.sort_by_key(|x| x.0);
        let first_arrival_ts = arrivals[0].0;
        let first_arrival_line = arrivals[0].1.clone();

        return Ok(ProcessedData{
            upcoming_trains: arrivals,
            upcoming_outbound_trains: outbound_arrivals,
            big_countdown: Some(drawing::countdown_summary(now, first_arrival_ts)),
            big_countdown_line: Some(first_arrival_line),
            station_name: data.name().to_string(),
            data_timestamp: data.data_timestamp(),
        });
    }
}
