extern crate rppal;

use rppal::gpio::Gpio;
use rppal::spi::Spi;

const RST_PIN: u8 = 17;
const DC_PIN: u8 = 25;
const BUSY_PIN: u8 = 24;
const PWR_PIN: u8 = 18;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 480;

fn send_cmd(dc: &mut rppal::gpio::OutputPin, spi: &mut Spi, cmd: u8) {
    dc.set_low();
    spi.write(&[cmd]).expect("spi write cmd");
}

fn send_data(dc: &mut rppal::gpio::OutputPin, spi: &mut Spi, data: u8) {
    dc.set_high();
    spi.write(&[data]).expect("spi write data");
}

fn send_data_bulk(dc: &mut rppal::gpio::OutputPin, spi: &mut Spi, data: &[u8]) {
    dc.set_high();
    // Send in chunks to avoid SPI buffer limits
    for chunk in data.chunks(4096) {
        spi.write(chunk).expect("spi write bulk");
    }
}

fn read_busy(dc: &mut rppal::gpio::OutputPin, spi: &mut Spi, busy_pin: &rppal::gpio::InputPin, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);
    loop {
        // V2 requires sending command 0x71 before reading the busy pin
        send_cmd(dc, spi, 0x71);
        if busy_pin.read() == rppal::gpio::Level::High {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn main() {
    println!("=== ttdash display test v3 (7.5\" V2, 800x480) ===");
    println!();

    // GPIO
    println!("[1/7] Initializing GPIO...");
    let gpio = match Gpio::new() {
        Ok(g) => { println!("  OK"); g },
        Err(e) => { println!("  FAIL: {}", e); return; },
    };

    let mut dc_pin = gpio.get(DC_PIN).expect("DC pin").into_output();
    let busy_pin = gpio.get(BUSY_PIN).expect("BUSY pin").into_input();
    let mut rst_pin = gpio.get(RST_PIN).expect("RST pin").into_output();
    let mut pwr_pin = gpio.get(PWR_PIN).expect("PWR pin").into_output();
    println!("  Pins: DC={}, BUSY={}, RST={}, PWR={}", DC_PIN, BUSY_PIN, RST_PIN, PWR_PIN);

    // Enable display power (required on HAT Rev 2.1+)
    println!("  Enabling power (GPIO {})...", PWR_PIN);
    pwr_pin.set_high();
    std::thread::sleep(std::time::Duration::from_millis(100));
    println!("  OK: Power enabled");

    // SPI
    println!();
    println!("[2/7] Initializing SPI...");
    let mut spi = match Spi::new(
        rppal::spi::Bus::Spi0,
        rppal::spi::SlaveSelect::Ss0,
        2000000,
        rppal::spi::Mode::Mode0,
    ) {
        Ok(s) => { println!("  OK"); s },
        Err(e) => { println!("  FAIL: {}", e); return; },
    };

    // Reset
    println!();
    println!("[3/7] Resetting display...");
    rst_pin.set_high();
    std::thread::sleep(std::time::Duration::from_millis(20));
    rst_pin.set_low();
    std::thread::sleep(std::time::Duration::from_millis(2));
    rst_pin.set_high();
    std::thread::sleep(std::time::Duration::from_millis(20));
    println!("  OK");

    // Init (V2 sequence from working Waveshare Python driver)
    println!();
    println!("[4/7] Sending V2 init sequence...");

    // Booster soft start (0x06)
    send_cmd(&mut dc_pin, &mut spi, 0x06);
    send_data(&mut dc_pin, &mut spi, 0x17);
    send_data(&mut dc_pin, &mut spi, 0x17);
    send_data(&mut dc_pin, &mut spi, 0x28);
    send_data(&mut dc_pin, &mut spi, 0x17);
    println!("  Sent BOOSTER (0x06)");

    // Power setting (0x01)
    send_cmd(&mut dc_pin, &mut spi, 0x01);
    send_data(&mut dc_pin, &mut spi, 0x07);
    send_data(&mut dc_pin, &mut spi, 0x07);
    send_data(&mut dc_pin, &mut spi, 0x28);
    send_data(&mut dc_pin, &mut spi, 0x17);
    println!("  Sent POWER_SETTING (0x01)");

    // Power on (0x04)
    send_cmd(&mut dc_pin, &mut spi, 0x04);
    println!("  Sent POWER_ON (0x04), waiting for ready...");
    std::thread::sleep(std::time::Duration::from_millis(100));
    if read_busy(&mut dc_pin, &mut spi, &busy_pin, 30) {
        println!("  OK: Display ready");
    } else {
        println!("  TIMEOUT: Display not responding after 30s");
        return;
    }

    // Panel setting (0x00)
    send_cmd(&mut dc_pin, &mut spi, 0x00);
    send_data(&mut dc_pin, &mut spi, 0x1F);
    println!("  Sent PANEL_SETTING (0x00)");

    // Resolution (0x61): 800x480
    send_cmd(&mut dc_pin, &mut spi, 0x61);
    send_data(&mut dc_pin, &mut spi, 0x03);  // 800 >> 8
    send_data(&mut dc_pin, &mut spi, 0x20);  // 800 & 0xFF
    send_data(&mut dc_pin, &mut spi, 0x01);  // 480 >> 8
    send_data(&mut dc_pin, &mut spi, 0xE0);  // 480 & 0xFF
    println!("  Sent RESOLUTION (0x61): 800x480");

    // 0x15
    send_cmd(&mut dc_pin, &mut spi, 0x15);
    send_data(&mut dc_pin, &mut spi, 0x00);
    println!("  Sent 0x15");

    // VCOM (0x50)
    send_cmd(&mut dc_pin, &mut spi, 0x50);
    send_data(&mut dc_pin, &mut spi, 0x10);
    send_data(&mut dc_pin, &mut spi, 0x07);
    println!("  Sent VCOM (0x50)");

    // TCON (0x60)
    send_cmd(&mut dc_pin, &mut spi, 0x60);
    send_data(&mut dc_pin, &mut spi, 0x22);
    println!("  Sent TCON (0x60)");

    println!("  OK: V2 init complete");

    // Send test pattern
    println!();
    println!("[5/7] Sending test pattern via 0x10 (old data)...");
    let bytes_per_row = (WIDTH / 8) as usize;  // 1 bit per pixel
    let total_bytes = bytes_per_row * HEIGHT as usize;

    // 0x10: "old" image data (inverted: 0x00 = white, 0xFF = black in this channel)
    send_cmd(&mut dc_pin, &mut spi, 0x10);
    let mut buf = vec![0x00u8; total_bytes];  // all white (inverted)
    send_data_bulk(&mut dc_pin, &mut spi, &buf);
    println!("  OK: Sent {} bytes", total_bytes);

    println!();
    println!("[6/7] Sending test pattern via 0x13 (new data)...");
    // 0x13: "new" image data - 1 bit per pixel, 0=white, 1=black
    // Horizontal stripes
    send_cmd(&mut dc_pin, &mut spi, 0x13);
    for y in 0..HEIGHT {
        let val = if (y / 60) % 2 == 0 { 0x00u8 } else { 0xFFu8 };
        for _x in 0..bytes_per_row {
            buf[0] = val;  // reuse buf
        }
        let row = vec![val; bytes_per_row];
        send_data_bulk(&mut dc_pin, &mut spi, &row);
    }
    println!("  OK: Sent {} bytes", total_bytes);

    // Refresh
    println!();
    println!("[7/7] Refreshing display...");
    send_cmd(&mut dc_pin, &mut spi, 0x12);
    std::thread::sleep(std::time::Duration::from_millis(100));
    println!("  Sent DISPLAY_REFRESH (0x12), waiting...");
    if read_busy(&mut dc_pin, &mut spi, &busy_pin, 60) {
        println!("  OK: Refresh complete");
    } else {
        println!("  TIMEOUT: Refresh did not complete in 60s");
    }

    println!();
    println!("=== Done! You should see horizontal black/white stripes. ===");
}
