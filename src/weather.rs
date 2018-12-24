// https://api.weather.gov/points/40.7128,-74.0060
// yields:
// "forecast": "https://api.weather.gov/gridpoints/OKX/32,34/forecast",
// "forecastHourly": "https://api.weather.gov/gridpoints/OKX/32,34/forecast/hourly",
// "forecastGridData": "https://api.weather.gov/gridpoints/OKX/32,34",
// "observationStations": "https://api.weather.gov/gridpoints/OKX/32,34/stations",
extern crate chrono;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate std;

use result;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsApiPeriod {
    name: String,
    number: i32,
    start_time: String,
    end_time: String,
    temperature: i32,
    short_forecast: String,
    wind_speed: String,
}

#[derive(Serialize, Deserialize)]
struct NwsApiProperties {
    periods: Vec<NwsApiPeriod>,
}

#[derive(Serialize, Deserialize)]
struct NwsApiForecast {
    properties: NwsApiProperties,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsApiGridValue {
    valid_time: String,
    value: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsApiGridProperty {
    source_unit: String,
    uom: String,
    values: Vec<NwsApiGridValue>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsApiGridProperties {
    temperature: NwsApiGridProperty,
    probability_of_precipitation: NwsApiGridProperty,
}

#[derive(Serialize, Deserialize)]
struct NwsApiGridForecast {
    properties: NwsApiGridProperties,
}

pub struct HourlyForecast {
    pub time: chrono::DateTime<chrono::FixedOffset>,
    pub temperature: i32,
}


pub struct DailyForecast {
    pub label: String,
    pub temperature: i32,
    pub short_forecast: String,
}

#[derive(Debug)]
pub struct GridForecastEntry {
    pub time: chrono::DateTime<chrono::FixedOffset>,
    pub duration: chrono::Duration,
    pub value: f32,
}

#[derive(Debug)]
pub struct GridForecast {
    pub precip_prob: Vec<GridForecastEntry>,
}

// Parses: "PT1H" -> 1 hour, "PT13H" -> 13 hours, etc
// https://en.wikipedia.org/wiki/ISO_8601#Durations
// TODO(mrjones): Parse day/month/year durations as well?
fn parse_duration(input: &str) -> Option<chrono::Duration> {
    let required_prefix = "PT";

    if !input.starts_with(required_prefix) {
        println!("Malformed duration {}", input);
        return None;
    }

    // TODO(mrjones): File bug against maplit, for not being able to use hashmap! here
    let mut time_parsers : std::collections::HashMap<char, fn(i64) -> chrono::Duration> = std::collections::HashMap::new();
    time_parsers.insert('H', chrono::Duration::hours);
    time_parsers.insert('M', chrono::Duration::minutes);
    time_parsers.insert('S', chrono::Duration::seconds);

    let mut result = chrono::Duration::seconds(0);
    let mut acc = 0;
    for (i, c) in input.chars().skip(required_prefix.len()).enumerate() {
        match c.to_digit(10) {
            Some(d) => acc = (10 * acc) + (d as i64),
            None => {
                match time_parsers.get(&c) {
                    Some(num_to_duration_fn) => {
                        result = result + num_to_duration_fn(acc);
                        acc = 0;
                    },
                    None => {
                        println!("Bad duration string '{}' at char #{}. ", input, i);
                        return None;
                    }
                }
            },
        }
    }

    return Some(result);
}

pub fn fetch_grid_forecast() -> result::TTDashResult<GridForecast> {
    use std::io::Read;

    let url = format!("https://api.weather.gov/gridpoints/OKX/32,34");

    let mut response = reqwest::get(&url)?;
    let mut response_body = String::new();
    response.read_to_string(&mut response_body)?;

    let forecast: NwsApiGridForecast = serde_json::from_str(&response_body)?;

    let mut precip_prob = vec![];
    for precip_entry in forecast.properties.probability_of_precipitation.values {
        let spec_parts: Vec<&str> = precip_entry.valid_time.split("/").collect();

        if spec_parts.len() > 0 {
            let duration = spec_parts
                .get(1)
                .and_then(|x| parse_duration(*x))
                .unwrap_or(chrono::Duration::hours(1));

            precip_prob.push(GridForecastEntry{
                time: chrono::DateTime::parse_from_rfc3339(spec_parts[0])?,
                duration: duration,
                value: precip_entry.value,
            });

        }
    }
    return Ok(GridForecast{
        precip_prob: precip_prob,
    });
}

pub fn fetch_daily_forecast() -> result::TTDashResult<Vec<DailyForecast>> {
    use std::io::Read;

    let url = format!("https://api.weather.gov/gridpoints/OKX/32,34/forecast");

    let mut response = reqwest::get(&url)?;
    let mut response_body = String::new();
    response.read_to_string(&mut response_body)?;

    let forecast: NwsApiForecast = serde_json::from_str(&response_body)?;

    let mut result = vec![];
    for period in forecast.properties.periods {
        result.push(DailyForecast{
            label: period.name,
            temperature: period.temperature,
            short_forecast: period.short_forecast,
        });
    }
    return Ok(result);

}

pub fn fetch_hourly_forecast() -> result::TTDashResult<Vec<HourlyForecast>> {
    use std::io::Read;

    let url = format!("https://api.weather.gov/gridpoints/OKX/32,34/forecast/hourly");

    let mut response = reqwest::get(&url)?;
    let mut response_body = String::new();
    response.read_to_string(&mut response_body)?;

    let forecast: NwsApiForecast = serde_json::from_str(&response_body)?;

    let mut result = vec![];
    for period in forecast.properties.periods {
        result.push(HourlyForecast{
            temperature: period.temperature,
            time: chrono::DateTime::parse_from_rfc3339(&period.start_time)?,
        });
    }
    return Ok(result);
}
