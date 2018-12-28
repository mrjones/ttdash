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
                } else if color <= 128 {
                    pixel_in_progress = 0x10;
                } else if color <= 192 {
                    pixel_in_progress = 0x20;
                } else {
                    pixel_in_progress = 0x30;
                }
            } else {
                if color < 64 {
                    pixel_in_progress |= 0x00;
                } else if color <= 128 {
                    pixel_in_progress |= 0x01;
                } else if color <= 192 {
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
    upcoming_trains: Vec<(i64, String)>,
    big_countdown: Option<String>,
    big_countdown_line: Option<String>,
    station_name: String,
}

impl ProcessedData {
    fn empty() -> ProcessedData {
        return ProcessedData{
            upcoming_trains: vec![],
            big_countdown: None,
            big_countdown_line: None,
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
            big_countdown: Some(countdown_summary(now, first_arrival_ts)),
            big_countdown_line: Some(first_arrival_line),
            station_name: data.get_name().to_string(),
        });
    }
}

fn scale(s: f32) -> rusttype::Scale {
    return rusttype::Scale{x: s, y: s};
}

fn draw_subway_line_emblem(imgbuf: &mut image::GrayImage, letter: &str, x: u32, y: u32, radius: u32, styles: &Styles) {
    imageproc::drawing::draw_filled_circle_mut(imgbuf, (x as i32, y as i32), (radius + 2)as i32, styles.color_white);
    imageproc::drawing::draw_filled_circle_mut(imgbuf, (x as i32, y as i32), radius as i32, styles.color_black);
    imageproc::drawing::draw_text_mut(imgbuf, styles.color_white, x - (radius / 2) + 2, y - radius, scale((radius * 2) as f32), &styles.font_bold, letter);
}

fn draw_subway_arrivals(imgbuf: &mut image::GrayImage, styles: &Styles, data: &ProcessedData) {
    let now = chrono::Utc::now().timestamp();

    imageproc::drawing::draw_filled_rect_mut(imgbuf, imageproc::rect::Rect::at(0,0).of_size(EPD_WIDTH as u32, EPD_HEIGHT as u32), styles.color_white);

    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 10, 10, scale(50.0), &styles.font_bold, &data.station_name);
    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 10, 50, scale(40.0), &styles.font, "To Manhattan");

    imageproc::drawing::draw_line_segment_mut(imgbuf, (10.0, 95.0), (EPD_HEIGHT as f32 - 10.0, 95.0), styles.color_black);

    use chrono::TimeZone;

    let big_line = data.big_countdown_line.clone().unwrap_or("R".to_string());
    match data.big_countdown {
        Some(ref big_text) => {
            let x;
            if big_text.len() == 1 {
                x = 70;
            } else {
                x = 10;
            }
            imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, x, 55, scale(250.0), &styles.font_black, big_text);
            if big_line != "R" {
                draw_subway_line_emblem(imgbuf, &big_line, 30, 125, 20, styles);
            }
        },
        _ => {},
    }

    let mut y = 100;
    let y_step = 40;
    for (ref ts, ref line) in data.upcoming_trains.iter().take(5) {
        let countdown = countdown_summary(now, *ts);
        let arrival = chrono_tz::US::Eastern.timestamp(*ts, 0);
        let arrival_formatted = arrival.format("%-I:%M").to_string();

        imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 219, y, scale(50.0), &styles.font_bold, &countdown);
        imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 284, y, scale(50.0), &styles.font, &arrival_formatted);

        if line != "R" {
            draw_subway_line_emblem(imgbuf, line, 375, y + 25, 12, styles);
        }

        y = y + y_step;
    }

}

fn draw_weather(imgbuf: &mut image::GrayImage, styles: &Styles, weather_display: &weather::WeatherDisplay) -> result::TTDashResult<()> {
    use chrono::Datelike;

    let left_x = 400;
    let top_y = 200;

    let precip_bar_max_height = 50;

    let t_bars_height = 40;
    let t_bars_offset = precip_bar_max_height + 30;

    let hour_width: u32 = 1;
    let day_width: u32 = 24 * hour_width + 5;

    let day_labels = vec!["S", "M", "T", "W", "R", "F", "S"];
    let first_entry = weather_display.days.iter().nth(0).ok_or(
        result::make_error("missing first entry"))?;
    let first_date = first_entry.0;
    let first_info = first_entry.1;

    for (date, info) in &weather_display.days {
        let day_count = date.num_days_from_ce() - first_date.num_days_from_ce();
        let min_pct = (info.min_t - weather_display.overall_min_t) / (weather_display.overall_max_t - weather_display.overall_min_t);
        let max_pct = (info.max_t - weather_display.overall_min_t) / (weather_display.overall_max_t - weather_display.overall_min_t);
        let day_label = day_labels.get(date.weekday().num_days_from_sunday() as usize).unwrap_or(&"?").to_string();

        imageproc::drawing::draw_text_mut(
            imgbuf, styles.color_black,
            /* x= */ left_x as u32 + day_count as u32 * day_width as u32 + (8 * hour_width),
            /* y= */ (top_y + precip_bar_max_height) as u32,
            scale(30.0), &styles.font_bold, &day_label);

        imageproc::drawing::draw_filled_rect_mut(
            imgbuf, imageproc::rect::Rect::at(
                left_x + day_count * day_width as i32 + 6 * hour_width as i32,
                top_y + t_bars_offset + (t_bars_height as f32 * (1.0 - max_pct)) as i32).
                of_size(12 * hour_width as u32, (t_bars_height as f32 * (max_pct - min_pct)) as u32),
            styles.color_black);

        imageproc::drawing::draw_text_mut(
            imgbuf, styles.color_black,
            /* x = */ (left_x + day_count * day_width as i32 + (8 * hour_width as i32)) as u32,
            /* y = */ (top_y + precip_bar_max_height + 75) as u32,
            scale(30.0), &styles.font, &format!("{:.0}", info.max_t));
        imageproc::drawing::draw_text_mut(
            imgbuf, styles.color_black,
            /* x = */ (left_x + day_count * day_width as i32 + (8 * hour_width as i32)) as u32,
            /* y = */ (top_y + precip_bar_max_height + 100) as u32,
            scale(30.0), &styles.font, &format!("{:.0}", info.min_t));

        for (hour, precip_prob) in &info.precip_by_hour {
            let bar_height = std::cmp::max(1, (precip_bar_max_height as f32 * (*precip_prob / 100.0)) as u32);

            imageproc::drawing::draw_filled_rect_mut(
                imgbuf,
                imageproc::rect::Rect::at(
                    /* x= */ left_x + day_count as i32 * day_width as i32 + *hour as i32 * hour_width as i32,
                    /* y= */ top_y + precip_bar_max_height - bar_height as i32)
                    .of_size(hour_width, bar_height),
                styles.color_black);
        }
    }

    imageproc::drawing::draw_text_mut(
        imgbuf, styles.color_black,
        /* x= */ 440, /* y= */ 0,
        scale(140.0),
        &styles.font_black, &format!("{}°", weather_display.current_t));

    imageproc::drawing::draw_text_mut(
        imgbuf, styles.color_black,
        /* x= */ left_x as u32,
        /* y= */ (top_y - 80) as u32,
        scale(80.0), &styles.font_bold,
        &format!("{}° / {}°", first_info.min_t, first_info.max_t));

    return Ok(());
}

fn generate_image(data: &ProcessedData, weather_display: Option<&weather::WeatherDisplay>, styles: &Styles) -> result::TTDashResult<image::GrayImage> {
    let mut imgbuf = image::GrayImage::new(EPD_WIDTH as u32, EPD_HEIGHT as u32);

    draw_subway_arrivals(&mut imgbuf, styles, data);

    if weather_display.is_some() {
        draw_weather(&mut imgbuf, styles, weather_display.unwrap())?;
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
    color_light_gray: image::Luma<u8>,
    color_dark_gray: image::Luma<u8>,
    color_white: image::Luma<u8>,
}

struct TTDash<'a> {
    weather_display: Option<weather::WeatherDisplay>,
    forecast_timestamp: chrono::DateTime<chrono::Utc>,
    styles: Styles<'a>,
}

impl<'a> TTDash<'a> {
    fn new() -> TTDash<'a> {
        let font = Vec::from(include_bytes!("/usr/share/fonts/truetype/roboto/hinted/RobotoCondensed-Regular.ttf") as &[u8]);
        let font = rusttype::FontCollection::from_bytes(font).unwrap().into_font().unwrap();

        let font_black = Vec::from(include_bytes!("/usr/share/fonts/truetype/roboto/hinted/RobotoCondensed-Bold.ttf") as &[u8]);
        let font_black = rusttype::FontCollection::from_bytes(font_black).unwrap().into_font().unwrap();

        let font_bold = Vec::from(include_bytes!("/usr/share/fonts/truetype/roboto/hinted/RobotoCondensed-Bold.ttf") as &[u8]);
        let font_bold = rusttype::FontCollection::from_bytes(font_bold).unwrap().into_font().unwrap();

        return TTDash {
            weather_display: None,
            forecast_timestamp: chrono::Utc::now(),

            styles: Styles{
                font_black: font_black,
                font_bold: font_bold,
                font: font,
                color_black: image::Luma{data: [0u8; 1]},
                color_dark_gray: image::Luma{data: [128u8; 1]},
                color_light_gray: image::Luma{data: [192u8; 1]},
                color_white: image::Luma{data: [255u8; 1]},
            },
        }
    }

    fn update_weather(&mut self, now: &chrono::DateTime<chrono::Utc>) -> result::TTDashResult<()> {
        self.weather_display = Some(weather::get_weather_display()?);
        self.forecast_timestamp = *now;

        return Ok(());
    }

    fn one_iteration(&mut self, display: bool, png_out: Option<String>, prev_processed_data: &ProcessedData) -> result::TTDashResult<ProcessedData>{
        let raw_data = fetch_data()?;
        let processed_data = process_data(&raw_data)?;

        // TODO(mrjones): Make this async or something?
        // TODO(mrjones): Don't fetch every time
        let now = chrono::Utc::now();
        if self.weather_display.is_none() || (now.timestamp() - self.forecast_timestamp.timestamp() > 60 * 30) {
            match self.update_weather(&now) {
                Ok(_) => {},
                Err(err) => println!("Error: {:?}", err),
            }
        }

        let imgbuf = generate_image(
            &processed_data,
            self.weather_display.as_ref(),
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
