extern crate image;
extern crate rppal;
extern crate std;

use rppal::gpio::{Gpio, Level, Mode};
use rppal::spi::{Spi};

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

pub fn init_display(gpio: &mut Gpio, spi: &mut Spi) {
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

pub fn display_image(gpio: &mut Gpio, spi: &mut Spi, imgbuf: &image::ImageBuffer<image::Luma<u8>, Vec<u8>>) {
    let mut pixel_in_progress: u8 = 0;

    use image::Pixel;

    for y in 0..imgbuf.height() {
        for x in 0..imgbuf.width() {
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
