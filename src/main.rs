// sudo apt-get install fonts-roboto libssl-dev
extern crate chrono;
extern crate chrono_tz;
extern crate getopts;
extern crate image;
extern crate imageproc;
extern crate protobuf;
extern crate reqwest;
extern crate rppal;
extern crate rusttype;
#[macro_use]
extern crate serde_derive;

mod result;
mod weather;
mod webclient_api;

use rppal::gpio::{Gpio, Level, Mode};
use rppal::spi::{Spi};


const EPD_WIDTH: usize = 640;
const EPD_HEIGHT: usize = 384;

const RST_PIN : u8 = 17;
const DC_PIN : u8 = 25;
const CS_PIN : u8 = 8;
const BUSY_PIN : u8 = 24;

const PANEL_SETTING : u8 = 0x00;
const POWER_SETTING : u8 = 0x01;
const POWER_ON : u8 = 0x04;
const BOOSTER_SOFT_START : u8 = 0x06;
const DATA_START_TRANSMISSION : u8 = 0x10;
const DISPLAY_REFRESH : u8 = 0x12;
const PLL_CONTROL : u8 = 0x30;
const TEMPERATURE_CALIBRATION : u8 = 0x41;
const VCOM_AND_DATA_INTERVAL_SETTING : u8 = 0x50;
const TCON_SETTING : u8 = 0x60;
const TCON_RESOLUTION : u8 = 0x61;
const VCM_DC_SETTING : u8 = 0x82;

fn send_command(gpio: &mut Gpio, spi: &mut Spi, command: u8) {
    gpio.write(DC_PIN, Level::Low);
    let v = vec![command];
    let bytes = spi.write(&v).expect("spi.write");
    assert_eq!(bytes, 1);
}

fn send_data(gpio: &mut Gpio, spi: &mut Spi, data: u8) {
    gpio.write(DC_PIN, Level::High);
    let v = vec![data];
    let bytes = spi.write(&v).expect("spi.write");
    assert_eq!(bytes, 1);
}

fn wait_until_idle(gpio: &mut Gpio) {
    loop {
        if gpio.read(BUSY_PIN).expect("gpio.read") == Level::Low {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn init_display(gpio: &mut Gpio, spi: &mut Spi) {

    gpio.set_mode(RST_PIN, Mode::Output);
    gpio.set_mode(DC_PIN, Mode::Output);
    gpio.set_mode(CS_PIN, Mode::Output);
    gpio.set_mode(BUSY_PIN, Mode::Input);

    gpio.write(RST_PIN, Level::Low);
    std::thread::sleep(std::time::Duration::from_millis(200));
    gpio.write(RST_PIN, Level::High);
    std::thread::sleep(std::time::Duration::from_millis(200));

    send_command(gpio, spi, POWER_SETTING);
    send_data(gpio, spi, 0x37);
    send_data(gpio, spi, 0x00);

    send_command(gpio, spi, PANEL_SETTING);
    send_data(gpio, spi, 0xCF);
    send_data(gpio, spi, 0x08);

    send_command(gpio, spi, BOOSTER_SOFT_START);
    send_data(gpio, spi, 0xc7);
    send_data(gpio, spi, 0xcc);
    send_data(gpio, spi, 0x28);

    send_command(gpio, spi, POWER_ON);
    wait_until_idle(gpio);

    send_command(gpio, spi, PLL_CONTROL);
    send_data(gpio, spi, 0x3c);

    send_command(gpio, spi, TEMPERATURE_CALIBRATION);
    send_data(gpio, spi, 0x00);

    send_command(gpio, spi, VCOM_AND_DATA_INTERVAL_SETTING);
    send_data(gpio, spi, 0x77);

    send_command(gpio, spi, TCON_SETTING);
    send_data(gpio, spi, 0x22);

    send_command(gpio, spi, TCON_RESOLUTION);
    send_data(gpio, spi, 0x02);     //source 640
    send_data(gpio, spi, 0x80);
    send_data(gpio, spi, 0x01);     //gate 384
    send_data(gpio, spi, 0x80);

    send_command(gpio, spi, VCM_DC_SETTING);
    send_data(gpio, spi, 0x1E);      //decide by LUT file;

    send_command(gpio, spi, 0xe5);           //FLASH MODE;
    send_data(gpio, spi, 0x03);

    // Draw all black?
    send_command(gpio, spi, DATA_START_TRANSMISSION);
}

fn display_image(gpio: &mut Gpio, spi: &mut Spi, imgbuf: &image::ImageBuffer<image::Luma<u8>, Vec<u8>>) {
    let mut pixel_in_progress: u8 = 0;

    use image::Pixel;

    for y in 0..EPD_HEIGHT {
        for x in 0..EPD_WIDTH {
            let color = imgbuf.get_pixel(x as u32, y as u32).to_luma().data[0];
            if x % 2 == 0 {
                if color < 64 {
                    pixel_in_progress = 0x00;
                } else if color < 128 {
                    pixel_in_progress = 0x10;
                } else if color < 192 {
                    pixel_in_progress = 0x20;
                } else {
                    pixel_in_progress = 0x30;
                }
            } else {
                if color < 64 {
                    pixel_in_progress |= 0x00;
                } else if color < 128 {
                    pixel_in_progress |= 0x01;
                } else if color < 192 {
                    pixel_in_progress |= 0x02;
                } else {
                    pixel_in_progress |= 0x03;
                }

                send_data(gpio, spi, pixel_in_progress);
            }
        }
    }

    send_command(gpio, spi, DISPLAY_REFRESH);
    std::thread::sleep(std::time::Duration::from_millis(100));
    wait_until_idle(gpio);

}

fn fetch_data() -> result::TTDashResult<webclient_api::StationStatus> {
    let url = format!("http://linode.mrjon.es:3838/api/station/028").to_string();
    let mut response = reqwest::get(&url)?;
    let mut response_body = vec![];
    use std::io::Read;
    response.read_to_end(&mut response_body)?;
    let proto = protobuf::parse_from_bytes::<webclient_api::StationStatus>(
        &response_body)?;
    return Ok(proto);
}

struct ProcessedData {
    upcoming_trains: Vec<i64>,
    big_countdown: Option<String>,
    station_name: String,
}

impl ProcessedData {
    fn empty() -> ProcessedData {
        return ProcessedData{
            upcoming_trains: vec![],
            big_countdown: None,
            station_name: "".to_string(),
        };
    }
}

fn countdown_summary(now_ts: i64, arrival_ts: i64) -> String {
    let wait_seconds = arrival_ts - now_ts;

    if wait_seconds < 60 {
        return "<1".to_string();
    }
    return format!("{}", wait_seconds / 60);
}

fn process_data(data: &webclient_api::StationStatus) -> result::TTDashResult<ProcessedData> {
    let mut arrivals = vec![];
    let now = chrono::Utc::now().timestamp();
    for line in data.get_line() {
        if line.get_line() == "R" && line.get_direction() == webclient_api::Direction::UPTOWN {
            for arrival in line.get_arrivals() {
                if arrival.get_timestamp() > now {
                    arrivals.push(arrival.get_timestamp());
                }
            }
        }
    }

    if arrivals.len() == 0 {
        return Ok(ProcessedData::empty());
    } else {
        arrivals.sort();
        let first_arrival = arrivals[0];

        return Ok(ProcessedData{
            upcoming_trains: arrivals,
            big_countdown: Some(countdown_summary(now, first_arrival)),
            station_name: data.get_name().to_string(),
        });
    }
}

fn scale(s: f32) -> rusttype::Scale {
    return rusttype::Scale{x: s, y: s};
}

fn draw_subway_arrivals(imgbuf: &mut image::GrayImage, styles: &Styles, data: &ProcessedData) {
    let now = chrono::Utc::now().timestamp();

    imageproc::drawing::draw_filled_rect_mut(imgbuf, imageproc::rect::Rect::at(0,0).of_size(EPD_WIDTH as u32, EPD_HEIGHT as u32), styles.color_white);

    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 10, 10, scale(50.0), &styles.font_bold, &data.station_name);
    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 10, 50, scale(40.0), &styles.font, "R to Manhattan");

    imageproc::drawing::draw_line_segment_mut(imgbuf, (10.0, 95.0), (EPD_HEIGHT as f32 - 10.0, 95.0), styles.color_black);

    use chrono::TimeZone;

    match data.big_countdown {
        Some(ref big_text) => {
            let x;
            if big_text.len() == 1 {
                x = 70;
            } else {
                x = 10;
            }
            imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, x, 55, scale(250.0), &styles.font_black, big_text);

        },
        _ => {},
    }

    for i in 0..std::cmp::min(data.upcoming_trains.len(), 5) {
        let countdown = countdown_summary(now, data.upcoming_trains[i]);
        let arrival = chrono_tz::US::Eastern.timestamp(data.upcoming_trains[i], 0);
        let arrival_formatted = arrival.format("%-I:%M").to_string();
        let y = 100 + 40 * i as u32;

        imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, EPD_HEIGHT as u32 - 100, y, scale(50.0), &styles.font, &arrival_formatted);
        imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, EPD_HEIGHT as u32 - 165, y, scale(50.0), &styles.font_bold, &countdown);
    }

}

fn draw_daily_forecast(imgbuf: &mut image::GrayImage, styles: &Styles, daily_forecast: &Vec<weather::DailyForecast>) {
    let weather_x = 400;
    let weather_y = 150;

    let y_step = 80;
    let mut y = weather_y;
    for ref day in daily_forecast.iter().take(3) {
        imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, weather_x, y, scale(30.0), &styles.font_bold, &day.label);
        imageproc::drawing::draw_line_segment_mut(imgbuf, (weather_x as f32, (y + 30) as f32), (EPD_WIDTH as f32 - 10.0, (y + 30) as f32), styles.color_black);
        imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, weather_x, y + 35, scale(30.0), &styles.font_bold, format!("{}°", day.temperature).as_ref());
        imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, weather_x + 45, y + 35, scale(30.0), &styles.font, &day.short_forecast);
        y = y + y_step;
    }
}

fn draw_hourly_trend(imgbuf: &mut image::GrayImage, styles: &Styles, hourly_forecast: &Vec<weather::HourlyForecast>) {
    let weather_x = 440.0;
    let weather_y = 110.0;  // Note: This is the _bottom_
    let weather_width = 170.0;
    let weather_height = 70.0;

    let min_t = hourly_forecast.iter()
        .take(24)
        .map(|x| x.temperature)
        .min()
        .unwrap_or(1);
    let max_t = hourly_forecast.iter()
        .take(24)
        .map(|x| x.temperature)
        .max()
        .unwrap_or(1);

    let x_step = weather_width / 24.0;

    let mut x = weather_x;
    let mut last_x = None;
    let mut last_y = None;

    let mut last_t = None;
    let mut trending_up = None;

    for hour in hourly_forecast.iter().take(24) {
        let y_fraction = (hour.temperature - min_t) as f32 / (max_t - min_t) as f32;
        let y = weather_y - (y_fraction * weather_height);
        if last_x.is_some() && last_y.is_some() {
            imageproc::drawing::draw_line_segment_mut(
                imgbuf, (last_x.unwrap(), last_y.unwrap()), (x, y), styles.color_black);
        }

        if last_t.is_none() {
            imageproc::drawing::draw_text_mut(
                imgbuf, styles.color_black, (x - 40.0) as u32, (y - 10.0) as u32, scale(30.0), &styles.font, format!("{}°", hour.temperature).as_ref());
        }

        if last_t.is_some() {
            let last_t = last_t.unwrap();
            if hour.temperature < last_t {
                if trending_up == Some(true) {
                    imageproc::drawing::draw_text_mut(
                        imgbuf, styles.color_black, (x - x_step) as u32, (weather_y - weather_height - 30.0) as u32, scale(30.0), &styles.font, format!("{}°", last_t).as_ref());
                }
                trending_up = Some(false);
            } else if hour.temperature > last_t {
                if trending_up == Some(false) {
                    imageproc::drawing::draw_text_mut(
                        imgbuf, styles.color_black, (x - x_step) as u32, (weather_y) as u32, scale(30.0), &styles.font, format!("{}°", last_t).as_ref());
                }
                trending_up = Some(true);
            }
        }

        last_t = Some(hour.temperature);
        last_x = Some(x);
        last_y = Some(y);

        x = x + x_step;
    }
}

fn generate_image(data: &ProcessedData, hourly_forecast: Option<&Vec<weather::HourlyForecast>>, daily_forecast: Option<&Vec<weather::DailyForecast>>, styles: &Styles) -> result::TTDashResult<image::GrayImage> {
    let mut imgbuf = image::GrayImage::new(EPD_WIDTH as u32, EPD_HEIGHT as u32);

    draw_subway_arrivals(&mut imgbuf, styles, data);

    if daily_forecast.is_some() {
        draw_daily_forecast(&mut imgbuf, styles, daily_forecast.unwrap());
    }

    if hourly_forecast.is_some() {
        draw_hourly_trend(&mut imgbuf, styles, hourly_forecast.unwrap());
    }

    return Ok(image::imageops::crop(&mut imgbuf, 0, 0, EPD_WIDTH as u32, EPD_HEIGHT as u32).to_image());
}

fn setup_and_display_image(image: &image::GrayImage) -> result::TTDashResult<()>{
    let mut gpio = rppal::gpio::Gpio::new()?;

    // Don't forget to enable SPI with sudo raspi-config
    let mut spi = rppal::spi::Spi::new(
        rppal::spi::Bus::Spi0,
        rppal::spi::SlaveSelect::Ss0,
        2000000,
        rppal::spi::Mode::Mode0)?;

    init_display(&mut gpio, &mut spi);
    display_image(&mut gpio, &mut spi, image);

    return Ok(());
}

struct Styles<'a> {
    font: rusttype::Font<'a>,
    font_bold: rusttype::Font<'a>,
    font_black: rusttype::Font<'a>,

    color_black: image::Luma<u8>,
    color_white: image::Luma<u8>,
}

struct TTDash<'a> {
    daily_forecast: Option<Vec<weather::DailyForecast>>,
    hourly_forecast: Option<Vec<weather::HourlyForecast>>,
    forecast_timestamp: chrono::DateTime<chrono::Utc>,
    styles: Styles<'a>,
}

impl<'a> TTDash<'a> {
    fn new() -> TTDash<'a> {
        let font = Vec::from(include_bytes!("/usr/share/fonts/truetype/roboto/hinted/Roboto-Regular.ttf") as &[u8]);
        let font = rusttype::FontCollection::from_bytes(font).unwrap().into_font().unwrap();

        let font_black = Vec::from(include_bytes!("/usr/share/fonts/truetype/roboto/hinted/RobotoCondensed-Bold.ttf") as &[u8]);
        let font_black = rusttype::FontCollection::from_bytes(font_black).unwrap().into_font().unwrap();

        let font_bold = Vec::from(include_bytes!("/usr/share/fonts/truetype/roboto/hinted/Roboto-Bold.ttf") as &[u8]);
        let font_bold = rusttype::FontCollection::from_bytes(font_bold).unwrap().into_font().unwrap();

        return TTDash {
            daily_forecast: None,
            hourly_forecast: None,
            forecast_timestamp: chrono::Utc::now(),

            styles: Styles{
                font_black: font_black,
                font_bold: font_bold,
                font: font,
                color_black: image::Luma{data: [0u8; 1]},
                color_white: image::Luma{data: [255u8; 1]},
            },
        }
    }


    fn one_iteration(&mut self, display: bool, png_out: Option<String>, prev_processed_data: &ProcessedData) -> result::TTDashResult<ProcessedData>{
        let raw_data = fetch_data()?;
        let processed_data = process_data(&raw_data)?;

        // TODO(mrjones): Make this async or something?
        // TODO(mrjones): Don't fetch every time
        let now = chrono::Utc::now();
        if self.hourly_forecast.is_none() || (now.timestamp() - self.forecast_timestamp.timestamp() > 60 * 30) {
            println!("Fetching weather forecast");
            self.hourly_forecast = Some(weather::fetch_hourly_forecast()?);
            self.daily_forecast = Some(weather::fetch_daily_forecast()?);
            self.forecast_timestamp = now;
        }

        let imgbuf = generate_image(
            &processed_data,
            self.hourly_forecast.as_ref(),
            self.daily_forecast.as_ref(),
            &self.styles)?;

        if png_out.is_some() {
            let _ = imgbuf.save(png_out.unwrap())?;
        }

        if prev_processed_data.big_countdown != processed_data.big_countdown {
            println!("Updating bignum {:?} -> {:?}",
                     prev_processed_data.big_countdown,
                     processed_data.big_countdown);
            if display {
                setup_and_display_image(&imgbuf)?;
            }
        } else {
            println!("Big num didn't change, not refreshing");
            std::thread::sleep(std::time::Duration::from_secs(5));
        }

        return Ok(processed_data);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();
    opts.optflag("d", "skip-display", "display to the epd device");
    opts.optflag("o", "one-shot", "keep the display up to date");
    opts.optopt("i", "save-image", "Where to put a png.", "FILENAME");

    let matches = opts.parse(&args[1..]).expect("parse opts");

    let display = !matches.opt_present("skip-display");
    let one_shot = matches.opt_present("one-shot");

    println!("Running. display={} one-shot={}", display, one_shot);

    let mut prev_processed_data = ProcessedData::empty();
    let mut ttdash = TTDash::new();

    loop {
        match ttdash.one_iteration(display, matches.opt_str("save-image"), &prev_processed_data) {
            Err(err) => eprintln!("{}", err),
            Ok(processed_data) => prev_processed_data = processed_data,
        }

        if one_shot {
            break;
        }
    }
}
