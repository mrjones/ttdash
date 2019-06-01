// https://api.weather.gov/points/40.7128,-74.0060
// yields:
// "forecast": "https://api.weather.gov/gridpoints/OKX/32,34/forecast",
// "forecastHourly": "https://api.weather.gov/gridpoints/OKX/32,34/forecast/hourly",
// "forecastGridData": "https://api.weather.gov/gridpoints/OKX/32,34",
// "observationStations": "https://api.weather.gov/gridpoints/OKX/32,34/stations",
//
// 32,34 seems to yield somewhat different data than 33,32.  33,32 seems more
// accurate for me, maybe that other grid is on the water or something?
// TODO(mrjones): (Geocode? -> ) LAT/LNG -> URL
extern crate chrono;
extern crate chrono_tz;
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

/*
pub struct HourlyForecast {
    pub time: chrono::DateTime<chrono::FixedOffset>,
    pub temperature: i32,
}


pub struct DailyForecast {
    pub label: String,
    pub temperature: i32,
    pub short_forecast: String,
}
*/

#[derive(Debug, Clone)]
pub struct GridForecastEntry {
    pub time: chrono::DateTime<chrono::FixedOffset>,
    pub duration: chrono::Duration,
    pub value: f32,
}

#[derive(Debug)]
pub struct GridForecast {
    pub precip_prob: Vec<GridForecastEntry>,
    pub temp: Vec<GridForecastEntry>,
}

#[derive(Debug)]
pub struct DenseGridHour {
    pub precip_prob: f32,
    pub temperature: f32,
}

#[derive(Debug)]
pub struct DenseGridForecast {
    pub hours: std::collections::BTreeMap<chrono::DateTime<chrono::FixedOffset>, DenseGridHour>,
}

pub struct WeatherDisplayDay {
    pub min_t: f32,
    pub max_t: f32,
    pub precip_by_hour: std::collections::BTreeMap<u32, f32>,
}

pub struct WeatherDisplay {
    pub overall_min_t: f32,
    pub overall_max_t: f32,
    pub current_t: f32,

    pub days: std::collections::BTreeMap<chrono::Date<chrono_tz::Tz>, WeatherDisplayDay>,
}

pub fn get_weather_display() -> result::TTDashResult<WeatherDisplay> {
    use chrono::Timelike;
    use chrono::TimeZone;

    let grid_forecast = fetch_grid_forecast()?;
    let dense_forecast = densify_grid_forecast(&grid_forecast)?;

    let mut days = std::collections::BTreeMap::new();

    let mut current_date = None;
    let mut min_t = None;
    let mut max_t = None;
    let mut precip_by_hour = std::collections::BTreeMap::new();
    let now_ts = chrono::Utc::now().timestamp() - 3600;

    let mut current_t = None;

    for (hour, values) in &dense_forecast.hours {
        let local_time = chrono_tz::US::Eastern.timestamp(hour.timestamp(), 0);
        if hour.timestamp() < now_ts {
            continue;
        }

        if Some(local_time.date()) != current_date {
            match current_date {
                Some(current_date) => {
                    // Ending an old day
                    days.insert(current_date, WeatherDisplayDay{
                        min_t: min_t.unwrap(),
                        max_t: max_t.unwrap(),
                        precip_by_hour: precip_by_hour,
                    });
                },
                None => {},
            }

            // Starting a new day
            current_date = Some(local_time.date());
            min_t = None;
            max_t = None;
            precip_by_hour = std::collections::BTreeMap::new();
        }

        if current_t.is_none() {
            current_t = Some(values.temperature);
        }

        if min_t.is_none() || values.temperature < min_t.unwrap() {
            min_t = Some(values.temperature);
        }

        if max_t.is_none() || values.temperature > max_t.unwrap() {
            max_t = Some(values.temperature);
        }

        precip_by_hour.insert(local_time.hour(), values.precip_prob);
    }

    return Ok(WeatherDisplay{
        overall_min_t: dense_forecast.hours.iter().min_by_key(|(_,e)| e.temperature as u32).ok_or(result::make_error("No data"))?.1.temperature,
        overall_max_t: dense_forecast.hours.iter().max_by_key(|(_,e)| e.temperature as u32).ok_or(result::make_error("No data"))?.1.temperature,
        current_t: current_t.ok_or(result::make_error("No data"))?,
        days: days,
    });
}

// Parses: "PT1H" -> 1 hour, "PT13H" -> 13 hours, etc
// https://en.wikipedia.org/wiki/ISO_8601#Durations
// TODO(mrjones): Parse day/month/year durations as well?
fn parse_duration(input: &str) -> result::TTDashResult<chrono::Duration> {
    let required_prefix = "P";

    if !input.starts_with(required_prefix) {
        return Err(result::make_error(&format!("Malformed duration {}", input)));
    }

    // TODO(mrjones): File bug against maplit, for not being able to use hashmap! here
    let mut time_parsers : std::collections::HashMap<char, fn(i64) -> chrono::Duration> = std::collections::HashMap::new();
    time_parsers.insert('H', chrono::Duration::hours);
    time_parsers.insert('M', chrono::Duration::minutes);
    time_parsers.insert('S', chrono::Duration::seconds);

    let mut date_parsers : std::collections::HashMap<char, fn(i64) -> chrono::Duration> = std::collections::HashMap::new();
    date_parsers.insert('D', chrono::Duration::days);

    let mut result = chrono::Duration::seconds(0);
    let mut acc = 0;

    #[derive(PartialEq)]
    enum ParseRegion { DatePart, TimePart };
    let mut parse_region = ParseRegion::DatePart;

    for (i, c) in input.chars().skip(required_prefix.len()).enumerate() {
        if c == 'T' {
            parse_region = ParseRegion::TimePart;
        } else {
            match c.to_digit(10) {
                Some(d) => acc = (10 * acc) + (d as i64),
                None => {
                    let parsers = if parse_region == ParseRegion::DatePart { &date_parsers} else { &time_parsers };
                    match parsers.get(&c) {
                        Some(num_to_duration_fn) => {
                            result = result + num_to_duration_fn(acc);
                            acc = 0;
                        },
                        None => {
                            return Err(result::make_error(&format!("Bad duration string '{}' at char #{}. ", input, i)));
                        }
                    }
                },
            }
        }
    }

    return Ok(result);
}

fn parse_time_and_duration(input: &str) -> result::TTDashResult<(chrono::DateTime<chrono::FixedOffset>, chrono::Duration)> {
    let parts: Vec<&str> = input.split("/").collect();

    if parts.len() < 2 {
        return Err(result::make_error(&format!(
            "Couldn't parse time+duration string: '{}'", input)));
    }

    return Ok((chrono::DateTime::parse_from_rfc3339(parts[0])?,
               parse_duration(parts[1])?));
}

fn parse_grid_entry(entry: &NwsApiGridValue) -> result::TTDashResult<GridForecastEntry> {
    let (time, duration) = parse_time_and_duration(&entry.valid_time)?;
    return Ok(GridForecastEntry{
        time: time,
        duration: duration,
        value: entry.value,
    });
}

pub fn fetch_grid_forecast() -> result::TTDashResult<GridForecast> {
    use std::io::Read;

    let url = format!("https://api.weather.gov/gridpoints/OKX/33,32");

    let mut response = reqwest::get(&url)?;
    let mut response_body = String::new();
    response.read_to_string(&mut response_body)?;

    let forecast: NwsApiGridForecast = serde_json::from_str(&response_body)?;

    let precip_probs : result::TTDashResult<Vec<GridForecastEntry>> =
        forecast.properties.probability_of_precipitation.values.iter().map(parse_grid_entry).collect();
    let temps : result::TTDashResult<Vec<GridForecastEntry>> =
        forecast.properties.temperature.values.iter().
        map(parse_grid_entry).
        map(|e_res| {
            return e_res.map(|e| {
                let mut e2 = e.clone();
                e2.value = 32.0 + e.value * 9.0 / 5.0;
                return e2;
            });
        }).collect();
    return Ok(GridForecast{
        precip_prob: precip_probs?,
        temp: temps?,
    });
}

pub fn densify_grid_forecast(sparse: &GridForecast) -> result::TTDashResult<DenseGridForecast> {
    let mut result = DenseGridForecast{
        hours: std::collections::BTreeMap::new(),
    };

    for precip_entry in &sparse.precip_prob {
        for i in 0..precip_entry.duration.num_hours() {
            let hour = precip_entry.time + chrono::Duration::hours(i);
            let mut hour_entry = result.hours.entry(hour)
                .or_insert(DenseGridHour{precip_prob: 0.0, temperature: 0.0});

            hour_entry.precip_prob = precip_entry.value;
        }
    }

    for temp_entry in &sparse.temp {
        for i in 0..temp_entry.duration.num_hours() {
            let hour = temp_entry.time + chrono::Duration::hours(i);
            let mut hour_entry = result.hours.entry(hour)
                .or_insert(DenseGridHour{precip_prob: 0.0, temperature: 0.0});

            hour_entry.temperature = temp_entry.value;
        }
    }

    return Ok(result);
}

/*
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
*/

#[cfg(test)]
mod tests {
    extern crate chrono;

    use super::parse_duration;

    #[test]
    fn simple_time_durations() {
        assert_eq!(chrono::Duration::hours(1), parse_duration("PT1H").unwrap());
        assert_eq!(chrono::Duration::hours(2), parse_duration("PT2H").unwrap());
        assert_eq!(chrono::Duration::hours(12), parse_duration("PT12H").unwrap());
        assert_eq!(chrono::Duration::minutes(1), parse_duration("PT1M").unwrap());
        assert_eq!(chrono::Duration::minutes(10), parse_duration("PT10M").unwrap());
        assert_eq!(chrono::Duration::seconds(5), parse_duration("PT5S").unwrap());
        assert_eq!(chrono::Duration::seconds(55), parse_duration("PT55S").unwrap());
    }

    #[test]
    fn simple_date_durations() {
        assert_eq!(chrono::Duration::days(1), parse_duration("P1D").unwrap());
        assert_eq!(chrono::Duration::days(2), parse_duration("P2D").unwrap());
        assert_eq!(chrono::Duration::days(12), parse_duration("P12D").unwrap());
    }

    #[test]
    fn combination_durations() {
        assert_eq!(chrono::Duration::minutes(90) + chrono::Duration::seconds(10),
                   parse_duration("PT1H30M10S").unwrap());
        assert_eq!(chrono::Duration::hours(36), parse_duration("P1DT12H").unwrap());
    }
}
