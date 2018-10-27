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

mod result;
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
                if color < 32 {
                    pixel_in_progress = 0x00;
                } else if color < 64 {
                    pixel_in_progress = 0x10;
                } else if color < 128 {
                    pixel_in_progress = 0x20;
                } else {
                    pixel_in_progress = 0x30;
                }
            } else {
                if color < 32 {
                    pixel_in_progress |= 0x00;
                } else if color < 64 {
                    pixel_in_progress |= 0x01;
                } else if color < 128 {
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
}

impl ProcessedData {
    fn empty() -> ProcessedData {
        return ProcessedData{ upcoming_trains: vec![], big_countdown: None };
    }
}

fn countdown_summary(now_ts: i64, arrival_ts: i64) -> String {
    let wait_seconds = arrival_ts - now_ts;

    if wait_seconds < 60 {
        return "<1m".to_string();
    } else if wait_seconds < 120 {
        return "1-2m".to_string();
    } else if wait_seconds < 180 {
        return "2-3m".to_string();
    }
    return format!("{}m", wait_seconds / 60);
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
        return Ok(ProcessedData{upcoming_trains: vec![], big_countdown: None});
    } else {
        arrivals.sort();
        let first_arrival = arrivals[0];
        
        return Ok(ProcessedData{
            upcoming_trains: arrivals,
            big_countdown: Some(countdown_summary(now, first_arrival))
        });
    }
}

fn generate_image(data: &ProcessedData) -> result::TTDashResult<image::GrayImage> {
    let mut imgbuf = image::GrayImage::new(EPD_WIDTH as u32, EPD_HEIGHT as u32);
    let font = Vec::from(include_bytes!("/usr/share/fonts/truetype/roboto/hinted/Roboto-Regular.ttf") as &[u8]);
    let font = rusttype::FontCollection::from_bytes(font).unwrap().into_font().unwrap();

    let font_black = Vec::from(include_bytes!("/usr/share/fonts/truetype/roboto/hinted/Roboto-Black.ttf") as &[u8]);
    let font_black = rusttype::FontCollection::from_bytes(font_black).unwrap().into_font().unwrap();

    let color_black = image::Luma{data: [0u8; 1]};
    let color_white = image::Luma{data: [255u8; 1]};
    
    let scale50 = rusttype::Scale { x: 50.0, y: 50.0 };
    let bignum_scale = rusttype::Scale { x: 250.0, y: 250.0 };
    
    imageproc::drawing::draw_filled_rect_mut(&mut imgbuf, imageproc::rect::Rect::at(0,0).of_size(EPD_WIDTH as u32, EPD_HEIGHT as u32), color_white);
    imageproc::drawing::draw_text_mut(&mut imgbuf, color_black, 10, 10, scale50, &font, "Cristina is pretty");

    use chrono::TimeZone;

    match data.big_countdown {
        Some(ref big_text) => imageproc::drawing::draw_text_mut(&mut imgbuf, color_black, 10, 20, bignum_scale, &font_black, big_text),
        _ => {},
    }

    for i in 0..std::cmp::min(data.upcoming_trains.len(), 3) {
        let arrival = chrono_tz::US::Eastern.timestamp(data.upcoming_trains[i], 0);
        let arrival_formatted = arrival.format("%-I:%M").to_string();
        imageproc::drawing::draw_text_mut(&mut imgbuf, color_black, 10, 250 + 40 * i as u32, scale50, &font, &arrival_formatted);
    }

    
    return Ok(imgbuf);
}

fn setup_and_display_image(image: &image::GrayImage) -> result::TTDashResult<()>{
    let mut gpio = rppal::gpio::Gpio::new().expect("Gpio::new()");

    // Don't forget to enable SPI with sudo raspi-config
    let mut spi = rppal::spi::Spi::new(
        rppal::spi::Bus::Spi0,
        rppal::spi::SlaveSelect::Ss0,
        2000000,
        rppal::spi::Mode::Mode0).expect("Spi::new()");

    init_display(&mut gpio, &mut spi);
    display_image(&mut gpio, &mut spi, image);

    return Ok(());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();
    opts.optflag("d", "skip-display", "display to the epd device");
    opts.optflag("o", "one-shot", "keep the display up to date");

    let matches = opts.parse(&args[1..]).expect("parse opts");

    let display = !matches.opt_present("skip-display");
    let one_shot = matches.opt_present("one-shot");
    println!("Running. display={} one-shot={}", display, one_shot);

    let mut prev_processed_data = ProcessedData::empty();
    
    loop {
        let raw_data = fetch_data().expect("fetch data");
        let processed_data = process_data(&raw_data).expect("process data");
        let imgbuf = generate_image(&processed_data).expect("generate image");

        let _ = imgbuf.save("/tmp/image.png").unwrap();

        if prev_processed_data.big_countdown != processed_data.big_countdown {
            println!("Updating bignum {:?} -> {:?}",
                     prev_processed_data.big_countdown,
                     processed_data.big_countdown);
            if display {
                setup_and_display_image(&imgbuf).expect("display image");
            }
        } else {
            println!("Big num didn't change, not refreshing");
            std::thread::sleep(std::time::Duration::from_secs(5));
        }

        if one_shot {
            break;
        }

        prev_processed_data = processed_data;
    }
}
