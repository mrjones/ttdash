extern crate chrono;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

use result;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsApiHourlyPeriod {
    number: i32,
    start_time: String,
    end_time: String,
    temperature: i32,
    short_forecast: String,
    wind_speed: String,
}

#[derive(Serialize, Deserialize)]
struct NwsApiHourlyProperties {
    periods: Vec<NwsApiHourlyPeriod>,
}

#[derive(Serialize, Deserialize)]
struct NwsApiHourlyForecast {
    properties: NwsApiHourlyProperties,
}

pub struct HourlyForecast {
    pub time: chrono::DateTime<chrono::FixedOffset>,
    pub temperature: i32,
}

pub fn fetch_hourly_forecast() -> result::TTDashResult<Vec<HourlyForecast>> {
    use std::io::Read;

    let url = format!("https://api.weather.gov/gridpoints/OKX/32,34/forecast/hourly");

    let mut response = reqwest::get(&url)?;
    let mut response_body = String::new();
    response.read_to_string(&mut response_body).expect("Response parse");

    let forecast: NwsApiHourlyForecast = serde_json::from_str(&response_body).expect("JSON parse");

    let mut result = vec![];
    for period in forecast.properties.periods {
        result.push(HourlyForecast{
            temperature: period.temperature,
            time: chrono::DateTime::parse_from_rfc3339(&period.start_time)?,
        });
    }
    return Ok(result);

}
