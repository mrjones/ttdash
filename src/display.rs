extern crate image;
extern crate rppal;
extern crate std;

use rppal::gpio::{Gpio, Level, InputPin, OutputPin};
use rppal::spi::{Spi};

use crate::result;

const RST_PIN : u8 = 17;
const DC_PIN : u8 = 25;
const _CS_PIN : u8 = 8;
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

pub fn setup_and_display_image(image: &image::GrayImage) -> result::TTDashResult<()>{
    let mut gpio = rppal::gpio::Gpio::new()?;
    let mut dc_pin = gpio.get(DC_PIN).expect("get dc pin").into_output();
    let busy_pin = gpio.get(BUSY_PIN).expect("get busy pin").into_input();

    // Don't forget to enable SPI with sudo raspi-config
    let mut spi = rppal::spi::Spi::new(
        rppal::spi::Bus::Spi0,
        rppal::spi::SlaveSelect::Ss0,
        2000000,
        rppal::spi::Mode::Mode0)?;

    init_display(&mut gpio, &mut spi, &mut dc_pin, &busy_pin);
    display_image(&mut dc_pin, &busy_pin, &mut spi, image);

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

fn wait_until_idle(busy_pin: &InputPin) {
    loop {
        if busy_pin.read() == Level::Low {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn init_display(gpio: &mut Gpio, spi: &mut Spi, dc_pin: &mut OutputPin, busy_pin: &InputPin) {
    let mut rst_pin = gpio.get(RST_PIN).expect("get rst pin").into_output();

    rst_pin.set_low();
    std::thread::sleep(std::time::Duration::from_millis(200));
    rst_pin.set_high();
    std::thread::sleep(std::time::Duration::from_millis(200));

    send_command(dc_pin, spi, POWER_SETTING);
    send_data(dc_pin, spi, 0x37);
    send_data(dc_pin, spi, 0x00);

    send_command(dc_pin, spi, PANEL_SETTING);
    send_data(dc_pin, spi, 0xCF);
    send_data(dc_pin, spi, 0x08);

    send_command(dc_pin, spi, BOOSTER_SOFT_START);
    send_data(dc_pin, spi, 0xc7);
    send_data(dc_pin, spi, 0xcc);
    send_data(dc_pin, spi, 0x28);

    send_command(dc_pin, spi, POWER_ON);
    wait_until_idle(&busy_pin);

    send_command(dc_pin, spi, PLL_CONTROL);
    send_data(dc_pin, spi, 0x3c);

    send_command(dc_pin, spi, TEMPERATURE_CALIBRATION);
    send_data(dc_pin, spi, 0x00);

    send_command(dc_pin, spi, VCOM_AND_DATA_INTERVAL_SETTING);
    send_data(dc_pin, spi, 0x77);

    send_command(dc_pin, spi, TCON_SETTING);
    send_data(dc_pin, spi, 0x22);

    send_command(dc_pin, spi, TCON_RESOLUTION);
    send_data(dc_pin, spi, 0x02);     //source 640
    send_data(dc_pin, spi, 0x80);
    send_data(dc_pin, spi, 0x01);     //gate 384
    send_data(dc_pin, spi, 0x80);

    send_command(dc_pin, spi, VCM_DC_SETTING);
    send_data(dc_pin, spi, 0x1E);      //decide by LUT file;

    send_command(dc_pin, spi, 0xe5);           //FLASH MODE;
    send_data(dc_pin, spi, 0x03);

    // Draw all black?
    send_command(dc_pin, spi, DATA_START_TRANSMISSION);
}

fn display_image(dc_pin: &mut OutputPin, busy_pin: &InputPin, spi: &mut Spi, imgbuf: &image::ImageBuffer<image::Luma<u8>, Vec<u8>>) {
    let mut pixel_in_progress: u8 = 0;

    use image::Pixel;

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

    send_command(dc_pin, spi, DISPLAY_REFRESH);
    std::thread::sleep(std::time::Duration::from_millis(100));
    wait_until_idle(&busy_pin);
}
