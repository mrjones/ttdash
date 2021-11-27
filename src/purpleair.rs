extern crate anyhow;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_with;

use anyhow::Context;
use crate::result;
use serde_with::{DisplayFromStr, serde_as};

#[derive(Debug)]
pub struct AirQuality {
    pub raw_pm25_ugm3: f32,
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub id: String,
    pub key: String,
}

pub fn credentials_from_file<P: AsRef<std::path::Path>>(path: P) -> result::TTDashResult<Credentials> {
    let debug_path = path.as_ref().to_str().map(|x| x.to_string());
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening purpleair creds from '{:?}'", debug_path))?;
    let reader = std::io::BufReader::new(file);
    let creds: Credentials = serde_json::from_reader(reader)
        .with_context(|| format!("while parsing credentials"))?;
    return Ok(creds);
}

pub fn get_air_quality(credentials: &Credentials) -> result::TTDashResult<AirQuality> {
    return get_air_quality_ext(&credentials.id, &credentials.key, real_fetch_json_fn);
}

fn get_air_quality_ext(id: &str, key: &str, fetch_json_fn: fn(&str) -> result::TTDashResult<String>) -> result::TTDashResult<AirQuality> {
    let raw_json = fetch_json_fn(
        &format!("https://www.purpleair.com/json?show={}&key={}", id, key))?;


    let response: PurpleAirResponse = serde_json::from_str(&raw_json)?;

    let first_result_with_data = response.results.iter()
        .filter(|r| r.raw_pm25_ugm3.is_some())
        .nth(0)
        .ok_or(result::make_error("Result didn't have one valid result"))?;

    return Ok(AirQuality{
        raw_pm25_ugm3: first_result_with_data.raw_pm25_ugm3.ok_or(
            result::make_error("pm2.5 missing"))?,
        });
}

fn real_fetch_json_fn(url: &str) -> result::TTDashResult<String> {
    use std::io::Read;

    let client = reqwest::blocking::Client::new();
    let mut response = client.get(url).header(reqwest::header::USER_AGENT, "ttdash from http://mrjon.es").send()?;
    let mut response_body = String::new();
    response.read_to_string(&mut response_body)?;
    return Ok(response_body);
}


#[derive(Serialize, Deserialize)]
struct PurpleAirResponse {
    results: std::vec::Vec<PurpleAirResult>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct PurpleAirResult {
    #[serde(rename = "ID")]
    id: i32,

    #[serde(rename = "Label")]
    label: String,

    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default, rename = "p_2_5_um")]
    raw_pm25_ugm3: Option<f32>,
}


#[cfg(test)]
mod tests {
    use super::result;

    #[test]
    fn json_parse() {
        let raw_json = std::fs::read_to_string("testdata/purpleair.json")
            .expect("Error reading purpleair.json");

        let response: super::PurpleAirResponse = serde_json::from_str(&raw_json)
            .expect("Error parsing JSON");

        assert_eq!(response.results[0].id, 12345);
        assert_eq!(response.results[0].raw_pm25_ugm3, Some(4.66));
    }

    #[test]
    fn fetch_air_quality() {
        let fake_fetch_fn = |_url: &str| -> result::TTDashResult<String> {
            return Ok(std::fs::read_to_string("testdata/purpleair.json").expect("error reading purpleair.json"));
        };

        let aq = super::get_air_quality_ext(1, "key", fake_fetch_fn).expect("Get air quality failed");

        assert_eq!(4.66, aq.raw_pm25_ugm3);
    }
}
