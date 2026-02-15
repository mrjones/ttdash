extern crate image;
extern crate rppal;
extern crate std;
extern crate time;

use rppal::gpio::{Gpio, Level, InputPin, OutputPin};
use rppal::spi::{Spi};

use crate::result;

const RST_PIN : u8 = 17;
const DC_PIN : u8 = 25;
const _CS_PIN : u8 = 8;
const BUSY_PIN : u8 = 24;
const PWR_PIN : u8 = 18;

#[derive(Clone, Copy, PartialEq)]
pub enum PanelVersion {
    V1,  // 640x384, 4-bit grayscale
    V2,  // 800x480, 1-bit
}

impl PanelVersion {
    pub fn width(&self) -> u32 {
        match self {
            PanelVersion::V1 => 640,
            PanelVersion::V2 => 800,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            PanelVersion::V1 => 384,
            PanelVersion::V2 => 480,
        }
    }
}

pub fn setup_and_display_image(image: &image::GrayImage, panel: PanelVersion) -> result::TTDashResult<()>{
    let mut gpio = rppal::gpio::Gpio::new()?;
    let mut dc_pin = gpio.get(DC_PIN).expect("get dc pin").into_output();
    let busy_pin = gpio.get(BUSY_PIN).expect("get busy pin").into_input();

    // Don't forget to enable SPI with sudo raspi-config
    let mut spi = rppal::spi::Spi::new(
        rppal::spi::Bus::Spi0,
        rppal::spi::SlaveSelect::Ss0,
        2000000,
        rppal::spi::Mode::Mode0)?;

    if panel == PanelVersion::V2 {
        let mut pwr_pin = gpio.get(PWR_PIN).expect("get pwr pin").into_output();
        pwr_pin.set_reset_on_drop(false);
        pwr_pin.set_high();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    init_display(&mut gpio, &mut spi, &mut dc_pin, &busy_pin, panel);
    display_image(&mut dc_pin, &busy_pin, &mut spi, image, panel);

    return Ok(());
}

fn send_command(dc_pin: &mut OutputPin, spi: &mut Spi, command: u8) {
    dc_pin.set_low();
    let v = vec![command];
    let bytes = spi.write(&v).expect("spi.write");
    assert_eq!(bytes, 1);
}

fn send_data(dc_pin: &mut OutputPin, spi: &mut Spi, data: u8) {
    dc_pin.set_high();
    let v = vec![data];
    let bytes = spi.write(&v).expect("spi.write");
    assert_eq!(bytes, 1);
}

fn send_data_bulk(dc_pin: &mut OutputPin, spi: &mut Spi, data: &[u8]) {
    dc_pin.set_high();
    for chunk in data.chunks(4096) {
        spi.write(chunk).expect("spi.write bulk");
    }
}

fn wait_until_idle_v1(busy_pin: &InputPin, timeout: time::Duration) {
    let deadline = time::Instant::now() + timeout;
    loop {
        if busy_pin.read() == Level::Low {
            return;
        }
        if time::Instant::now() > deadline {
            warn!("wait_until_idle timeout ({}) expired.", timeout);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn wait_until_idle_v2(dc_pin: &mut OutputPin, spi: &mut Spi, busy_pin: &InputPin, timeout: time::Duration) {
    let deadline = time::Instant::now() + timeout;
    loop {
        send_command(dc_pin, spi, 0x71);
        if busy_pin.read() == Level::High {
            return;
        }
        if time::Instant::now() > deadline {
            warn!("wait_until_idle timeout ({}) expired.", timeout);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn init_display(gpio: &mut Gpio, spi: &mut Spi, dc_pin: &mut OutputPin, busy_pin: &InputPin, panel: PanelVersion) {
    let mut rst_pin = gpio.get(RST_PIN).expect("get rst pin").into_output();

    match panel {
        PanelVersion::V1 => {
            rst_pin.set_low();
            std::thread::sleep(std::time::Duration::from_millis(200));
            rst_pin.set_high();
            std::thread::sleep(std::time::Duration::from_millis(200));

            send_command(dc_pin, spi, 0x01); // POWER_SETTING
            send_data(dc_pin, spi, 0x37);
            send_data(dc_pin, spi, 0x00);

            send_command(dc_pin, spi, 0x00); // PANEL_SETTING
            send_data(dc_pin, spi, 0xCF);
            send_data(dc_pin, spi, 0x08);

            send_command(dc_pin, spi, 0x06); // BOOSTER_SOFT_START
            send_data(dc_pin, spi, 0xc7);
            send_data(dc_pin, spi, 0xcc);
            send_data(dc_pin, spi, 0x28);

            send_command(dc_pin, spi, 0x04); // POWER_ON
            wait_until_idle_v1(&busy_pin, time::Duration::minutes(3));

            send_command(dc_pin, spi, 0x30); // PLL_CONTROL
            send_data(dc_pin, spi, 0x3c);

            send_command(dc_pin, spi, 0x41); // TEMPERATURE_CALIBRATION
            send_data(dc_pin, spi, 0x00);

            send_command(dc_pin, spi, 0x50); // VCOM_AND_DATA_INTERVAL_SETTING
            send_data(dc_pin, spi, 0x77);

            send_command(dc_pin, spi, 0x60); // TCON_SETTING
            send_data(dc_pin, spi, 0x22);

            send_command(dc_pin, spi, 0x61); // TCON_RESOLUTION
            send_data(dc_pin, spi, 0x02);    // source 640
            send_data(dc_pin, spi, 0x80);
            send_data(dc_pin, spi, 0x01);    // gate 384
            send_data(dc_pin, spi, 0x80);

            send_command(dc_pin, spi, 0x82); // VCM_DC_SETTING
            send_data(dc_pin, spi, 0x1E);

            send_command(dc_pin, spi, 0xe5); // FLASH MODE
            send_data(dc_pin, spi, 0x03);

            send_command(dc_pin, spi, 0x10); // DATA_START_TRANSMISSION
        },
        PanelVersion::V2 => {
            rst_pin.set_high();
            std::thread::sleep(std::time::Duration::from_millis(20));
            rst_pin.set_low();
            std::thread::sleep(std::time::Duration::from_millis(2));
            rst_pin.set_high();
            std::thread::sleep(std::time::Duration::from_millis(20));

            send_command(dc_pin, spi, 0x06); // BOOSTER_SOFT_START
            send_data(dc_pin, spi, 0x17);
            send_data(dc_pin, spi, 0x17);
            send_data(dc_pin, spi, 0x28);
            send_data(dc_pin, spi, 0x17);

            send_command(dc_pin, spi, 0x01); // POWER_SETTING
            send_data(dc_pin, spi, 0x07);
            send_data(dc_pin, spi, 0x07);
            send_data(dc_pin, spi, 0x28);
            send_data(dc_pin, spi, 0x17);

            send_command(dc_pin, spi, 0x04); // POWER_ON
            std::thread::sleep(std::time::Duration::from_millis(100));
            wait_until_idle_v2(dc_pin, spi, busy_pin, time::Duration::minutes(3));

            send_command(dc_pin, spi, 0x00); // PANEL_SETTING
            send_data(dc_pin, spi, 0x1F);

            send_command(dc_pin, spi, 0x61); // RESOLUTION
            send_data(dc_pin, spi, 0x03);    // source 800
            send_data(dc_pin, spi, 0x20);
            send_data(dc_pin, spi, 0x01);    // gate 480
            send_data(dc_pin, spi, 0xE0);

            send_command(dc_pin, spi, 0x15);
            send_data(dc_pin, spi, 0x00);

            send_command(dc_pin, spi, 0x50); // VCOM
            send_data(dc_pin, spi, 0x10);
            send_data(dc_pin, spi, 0x07);

            send_command(dc_pin, spi, 0x60); // TCON_SETTING
            send_data(dc_pin, spi, 0x22);
        },
    }
}

fn display_image(dc_pin: &mut OutputPin, busy_pin: &InputPin, spi: &mut Spi, imgbuf: &image::ImageBuffer<image::Luma<u8>, Vec<u8>>, panel: PanelVersion) {
    use image::Pixel;

    match panel {
        PanelVersion::V1 => {
            let mut pixel_in_progress: u8 = 0;

            for y in 0..imgbuf.height() {
                for x in 0..imgbuf.width() {
                    let color = imgbuf.get_pixel(x as u32, y as u32).to_luma()[0];
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

                        send_data(dc_pin, spi, pixel_in_progress);
                    }
                }
            }

            send_command(dc_pin, spi, 0x12); // DISPLAY_REFRESH
            std::thread::sleep(std::time::Duration::from_millis(100));
            wait_until_idle_v1(&busy_pin, time::Duration::minutes(3));
        },
        PanelVersion::V2 => {
            let width = imgbuf.width();
            let height = imgbuf.height();
            let bytes_per_row = ((width + 7) / 8) as usize;

            // Convert grayscale image to 1-bit packed bytes
            let mut buf = vec![0x00u8; bytes_per_row * height as usize];
            for y in 0..height {
                for x in 0..width {
                    let color = imgbuf.get_pixel(x, y).to_luma()[0];
                    // 1 = black, 0 = white (threshold at 128)
                    if color < 128 {
                        let byte_idx = (y as usize) * bytes_per_row + (x as usize) / 8;
                        let bit_idx = 7 - (x % 8);
                        buf[byte_idx] |= 1 << bit_idx;
                    }
                }
            }

            // 0x10: "old" data (all white)
            send_command(dc_pin, spi, 0x10);
            let blank = vec![0x00u8; buf.len()];
            send_data_bulk(dc_pin, spi, &blank);

            // 0x13: "new" data
            send_command(dc_pin, spi, 0x13);
            send_data_bulk(dc_pin, spi, &buf);

            send_command(dc_pin, spi, 0x12); // DISPLAY_REFRESH
            std::thread::sleep(std::time::Duration::from_millis(100));
            wait_until_idle_v2(dc_pin, spi, busy_pin, time::Duration::minutes(3));
        },
    }
}
