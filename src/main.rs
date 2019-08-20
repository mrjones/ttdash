// sudo apt-get install fonts-roboto libssl-dev
extern crate chrono;
extern crate chrono_tz;
extern crate flexi_logger;
extern crate getopts;
extern crate hex;
extern crate image;
extern crate imageproc;
#[macro_use] extern crate log;
extern crate md5;
extern crate nix;
extern crate pretty_bytes;
extern crate protobuf;
extern crate querystring;
extern crate reqwest;
extern crate rppal;
extern crate rusttype;
#[macro_use] extern crate serde_derive;
extern crate simple_server;

mod debug;
mod display;
mod drawing;
mod result;
mod subway;
mod update;
mod weather;
mod webclient_api;


struct TTDash<'a> {
    weather_display: Option<weather::WeatherDisplay>,
    forecast_timestamp: chrono::DateTime<chrono::Utc>,
    styles: drawing::Styles<'a>,
    last_redraw: Option<chrono::DateTime<chrono::Utc>>,
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
            styles: drawing::Styles{
                font_black: font_black,
                font_bold: font_bold,
                font: font,
                color_black: image::Luma{data: [0u8; 1]},
                color_dark_gray: image::Luma{data: [128u8; 1]},
                color_light_gray: image::Luma{data: [192u8; 1]},
                color_white: image::Luma{data: [255u8; 1]},
            },
            last_redraw: None,
        }
    }

    fn update_weather(&mut self, now: &chrono::DateTime<chrono::Utc>) -> result::TTDashResult<()> {
        self.weather_display = Some(weather::get_weather_display(now.timestamp())?);
        self.forecast_timestamp = *now;

        return Ok(());
    }

    fn one_iteration(&mut self, display: bool, png_out: Option<&str>, prev_processed_data: &subway::ProcessedData, auto_update: bool) -> result::TTDashResult<Option<subway::ProcessedData>> {
        if auto_update {
            match update::binary_update_available() {
                Some(target) => {
                    info!("Upgrade available to version {}.", target.version);
                    let argv0 = std::env::args().nth(0).expect("argv0");
                    let argv: Vec<String> = std::env::args().collect();
                    match update::upgrade_to(&target, &argv0, &argv) {
                        Err(err) => error!("Upgrade error: {:?}", err),
                        _ => {},
                    }
                },
                None => {
                    debug!("No update available");
                },
            }
        }


        let processed_data = subway::fetch_and_process_data()?;

        // TODO(mrjones): Make this async or something?
        // TODO(mrjones): Don't fetch every time
        let now = chrono::Utc::now();
        if self.weather_display.is_none() || (now.timestamp() - self.forecast_timestamp.timestamp() > 60 * 30) {
            match self.update_weather(&now) {
                Ok(_) => {},
                Err(err) => error!("Error: {:?}", err),
            }
        }

        let mut needs_redraw = false;

        let data_went_back_in_time = processed_data.data_timestamp < prev_processed_data.data_timestamp;
        if data_went_back_in_time {
            warn!("Ignoring data ({}) that's older than what's already displayed ({}).",
                  processed_data.data_timestamp,
                  prev_processed_data.data_timestamp);
        }

        if prev_processed_data.big_countdown != processed_data.big_countdown &&
            !data_went_back_in_time {
            info!("Updating bignum {:?} -> {:?}",
                     prev_processed_data.big_countdown,
                     processed_data.big_countdown);
            needs_redraw = true;
        } else if self.last_redraw.is_none() {
            // Probably never happens in practice?
            info!("Drawing for the first time.");
            needs_redraw = true;
        } else if self.last_redraw.is_some() {
            let seconds_since_redraw = now.timestamp() - self.last_redraw.unwrap().timestamp();
            needs_redraw = seconds_since_redraw > 60 * 30;
            if needs_redraw {
                info!("Redrawing since it's been too long.");
            }
        }

        if needs_redraw {
            let imgbuf = drawing::generate_image(
                &processed_data,
                self.weather_display.as_ref(),
                update::local_version().ok().map(|v| v.to_string()),
                &self.styles)?;

            if png_out.is_some() {
                let _ = imgbuf.save(png_out.unwrap())?;
            }

            if display {
                display::setup_and_display_image(&imgbuf)?;
            }
            self.last_redraw = Some(now);
            return Ok(Some(processed_data));
        } else {
            debug!("Not refreshing.");
            return Ok(None)
        }
    }
}

fn short_level(level: log::Level) -> String {
    return match level {
        log::Level::Error => "E".to_string(),
        log::Level::Warn => "W".to_string(),
        log::Level::Info => "I".to_string(),
        log::Level::Debug => "D".to_string(),
        _ => level.to_string(),
    };
}

fn format_log(
    w: &mut std::io::Write,
    now: &mut flexi_logger::DeferredNow,
    record: &log::Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}{} {}:{:<4}] {}",
        short_level(record.level()),
        now.now().format("%Y%m%d %H:%M:%S%.6f"),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

fn main() {
    flexi_logger::Logger::with_env_or_str("info")
        .format(format_log)
        .log_to_file()
        .append()
        .duplicate_to_stderr(flexi_logger::Duplicate::Info)
        .rotate(
            flexi_logger::Criterion::Size(1 * 1024 * 1024),
            flexi_logger::Naming::Numbers,
            flexi_logger::Cleanup::KeepLogFiles(100))
        .print_message()
        .start()
        .unwrap();

    let args: Vec<String> = std::env::args().collect();
    info!(" - - - - - - - - - - - - - ");
    info!("Command Line: {:?}", args);
    match update::local_version() {
        Ok(version) => {
            info!("TTDash version {}", version);
        },
        _ => {},
    }

    let mut opts = getopts::Options::new();
    opts.optflag("d", "skip-display", "display to the epd device");
    opts.optflag("o", "one-shot", "keep the display up to date");
    opts.optopt("i", "save-image", "Where to put a png.", "FILENAME");
    opts.optopt("p", "debug-port", "Port to run a debug server on.", "PORT");
    opts.optflag("u", "auto-update", "Run the auto-updater.");

    let matches = opts.parse(&args[1..]).expect("parse opts");

    let display = !matches.opt_present("skip-display");
    let one_shot = matches.opt_present("one-shot");
    let debug_port = matches.opt_str("debug-port");
    let auto_update = matches.opt_present("auto-update");
    let local_png: Option<String> = matches.opt_str("save-image");

    info!("Running with config: display={} one-shot={} debug-port={:?} auto-update={} local-png={:?}", display, one_shot, debug_port, auto_update, local_png);

    let mut prev_processed_data = subway::ProcessedData::empty();
    let mut ttdash = TTDash::new();

    match debug_port {
        Some(port) => {
            let local_png = local_png.clone();
            std::thread::spawn(move || { debug::run_debug_server(&port, local_png); });
        },
        None => {},
    }

    if auto_update {
        assert!(update::updater_configured());
    }

    loop {
        match ttdash.one_iteration(display, local_png.as_ref().map(String::as_ref), &prev_processed_data, auto_update) {
            Err(err) => error!("{}", err),
            Ok(processed_data) => {
                if let Some(processed_data) = processed_data {
                    prev_processed_data = processed_data;
                }
            }
        }

        if one_shot {
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
