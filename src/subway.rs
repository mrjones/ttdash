extern crate chrono;
extern crate protobuf;
extern crate reqwest;

use drawing;
use result;
use webclient_api;

pub struct ProcessedData {
    pub upcoming_trains: Vec<(i64, String)>,
    pub big_countdown: Option<String>,
    pub big_countdown_line: Option<String>,
    pub station_name: String,
}

impl ProcessedData {
    pub fn empty() -> ProcessedData {
        return ProcessedData{
            upcoming_trains: vec![],
            big_countdown: None,
            big_countdown_line: None,
            station_name: "".to_string(),
        };
    }
}

pub fn fetch_and_process_data() -> result::TTDashResult<ProcessedData> {
    let raw_data = fetch_data()?;
    return process_data(&raw_data);
}

fn fetch_data() -> result::TTDashResult<webclient_api::StationStatus> {
    let url = format!("http://traintrack.nyc/api/station/028").to_string();
    let mut response = reqwest::get(&url)?;
    let mut response_body = vec![];
    use std::io::Read;
    response.read_to_end(&mut response_body)?;
    let proto = protobuf::parse_from_bytes::<webclient_api::StationStatus>(
        &response_body)?;
    return Ok(proto);
}

fn process_data(data: &webclient_api::StationStatus) -> result::TTDashResult<ProcessedData> {
    let mut arrivals = vec![];
    let now = chrono::Utc::now().timestamp();
    for line in data.get_line() {
        if line.get_direction() == webclient_api::Direction::UPTOWN {
            for arrival in line.get_arrivals() {
                if arrival.get_timestamp() > now {
                    arrivals.push((arrival.get_timestamp(), line.get_line().to_string()));
                }
            }
        }
    }

    if arrivals.len() == 0 {
        return Ok(ProcessedData::empty());
    } else {
        arrivals.sort_by_key(|x| x.0);
        let first_arrival_ts = arrivals[0].0;
        let first_arrival_line = arrivals[0].1.to_string();

        return Ok(ProcessedData{
            upcoming_trains: arrivals,
            big_countdown: Some(drawing::countdown_summary(now, first_arrival_ts)),
            big_countdown_line: Some(first_arrival_line),
            station_name: data.get_name().to_string(),
        });
    }
}
