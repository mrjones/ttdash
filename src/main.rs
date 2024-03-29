// sudo apt-get install fonts-roboto libssl-dev
#[macro_use] extern crate anyhow;
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
extern crate prost;
extern crate querystring;
extern crate reqwest;
extern crate rppal;
extern crate rusttype;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_with;
#[macro_use] extern crate time;
extern crate tiny_http;

mod bustime;
mod debug;
mod display;
mod drawing;
mod purpleair;
mod result;
mod subway;
mod update;
mod weather;


pub mod webclient_api {
    include!(concat!(env!("OUT_DIR"), "/webclient_api.rs"));
}


struct TTDash<'a> {
    weather_display: Option<weather::WeatherDisplay>,
    forecast_timestamp: chrono::DateTime<chrono::Utc>,
    air_quality: Option<purpleair::AirQuality>,
    air_quality_timestamp: chrono::DateTime<chrono::Utc>,
    bus_time_data: Option<bustime::BusTimeDisplayData>,
    styles: drawing::Styles<'a>,
    last_redraw: Option<chrono::DateTime<chrono::Utc>>,
}

impl<'a> TTDash<'a> {
    fn new() -> TTDash<'a> {
        // ln

        let font = include_bytes!("/usr/share/fonts/truetype/roboto/unhinted/RobotoCondensed-Regular.ttf");
        let font = rusttype::Font::try_from_bytes(font).unwrap();

        let font_black = include_bytes!("/usr/share/fonts/truetype/roboto/unhinted/RobotoCondensed-Bold.ttf");
        let font_black = rusttype::Font::try_from_bytes(font_black).unwrap();

        let font_bold = include_bytes!("/usr/share/fonts/truetype/roboto/unhinted/RobotoCondensed-Bold.ttf");
        let font_bold = rusttype::Font::try_from_bytes(font_bold).unwrap();

        return TTDash {
            weather_display: None,
            forecast_timestamp: chrono::Utc::now(),
            air_quality: None,
            air_quality_timestamp: chrono::Utc::now(),
            bus_time_data: None,
            styles: drawing::Styles{
                font_black: font_black,
                font_bold: font_bold,
                font: font,
                color_black: image::Luma([0u8; 1]),
                color_dark_gray: image::Luma([128u8; 1]),
                color_light_gray: image::Luma([192u8; 1]),
                color_white: image::Luma([255u8; 1]),
            },
            last_redraw: None,
        }
    }

    fn update_air_quality(&mut self, creds: &purpleair::Credentials, now: &chrono::DateTime<chrono::Utc>) -> result::TTDashResult<()> {
        self.air_quality = Some(purpleair::get_air_quality(creds)?);
        self.air_quality_timestamp = *now;

        return Ok(());
    }

    fn update_weather(&mut self, now: &chrono::DateTime<chrono::Utc>) -> result::TTDashResult<()> {
        self.weather_display = Some(weather::get_weather_display(now.timestamp())?);
        self.forecast_timestamp = *now;

        return Ok(());
    }

    fn one_iteration(&mut self, display: bool, png_out: Option<&str>, prev_processed_data: &subway::ProcessedData, auto_update: bool, purpleair_creds: Option<&purpleair::Credentials>, mta_bustime_creds: Option<&String>) -> result::TTDashResult<Option<subway::ProcessedData>> {
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
                Err(err) => error!("Error updating weather: {:?}", err),
            }
        }

        if purpleair_creds.is_some() {
            if self.air_quality.is_none() || (now.timestamp() - self.air_quality_timestamp.timestamp() > 60) {
                match self.update_air_quality(purpleair_creds.unwrap(), &now) {
                    Ok(_) => { info!("AQ: {:?}", self.air_quality); },
                    Err(err) => { error!("Error updating air quality: {:?}", err); },
                }
            }

        }

        if mta_bustime_creds.is_some() {
            let bustimes = bustime::get_garfield_bus_arrivals(mta_bustime_creds.unwrap().as_ref());
            debug!("FINAL BUSTIMES: {:?}", bustimes);

            match bustimes {
                Ok(bustimes) => {
                    let now = time::OffsetDateTime::now_utc();

                    let ts_to_wait_minutes = |ts: &time::OffsetDateTime| {
                        let delta = ts.clone() - now;
                        return delta.whole_minutes();
                    };

                    let uptown_waits: Vec<i64> = bustimes.uptown_timestamps.iter().map(ts_to_wait_minutes).collect();
                    let downtown_waits: Vec<i64> = bustimes.downtown_timestamps.iter().map(ts_to_wait_minutes).collect();
                    debug!("UPTOWN WAITS: {:?}", uptown_waits);
                    debug!("DOWNTOWN WAITS: {:?}", downtown_waits);

                    self.bus_time_data = Some(bustime::BusTimeDisplayData{
                        uptown_waits,
                        downtown_waits,
                        timestamp: time::OffsetDateTime::now_utc(),
                    });
                },
                Err(err) => error!("Error getting bustimes: {}", err),
            }
        }

        let mut needs_redraw = false;

        let data_went_back_in_time = processed_data.data_timestamp < prev_processed_data.data_timestamp;
        if data_went_back_in_time {
            debug!("Ignoring data ({}) that's older than what's already displayed ({}).",
                  processed_data.data_timestamp,
                  prev_processed_data.data_timestamp);
        }

        if prev_processed_data.big_countdown != processed_data.big_countdown &&
            !data_went_back_in_time {
            debug!("Updating bignum {:?} -> {:?}",
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
                self.air_quality.as_ref(),
                self.bus_time_data.as_ref(),
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
    w: &mut dyn std::io::Write,
    now: &mut flexi_logger::DeferredNow,
    record: &log::Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}{} {}:{:<4}] {}",
        short_level(record.level()),
        now.now().format("%Y%m%d %H:%M:%S.%3f"),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

fn main() {
    flexi_logger::Logger::try_with_env_or_str("info")
        .unwrap()
        .format(format_log)
        .log_to_file(flexi_logger::FileSpec::default().use_timestamp(true))
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

    opts.optopt("", "purpleair-credentials-file", "Name of a file containing JSON {key: xx, id: xxx} value with purpleair credentials.", "FILE");

    opts.optopt("", "mta-bustime-credentials-file", "Name of a file containing the MTA bustime API key", "FILE");

    let matches = opts.parse(&args[1..]).expect("parse opts");

    let display = !matches.opt_present("skip-display");
    let one_shot = matches.opt_present("one-shot");
    let debug_port = matches.opt_str("debug-port");
    let auto_update = matches.opt_present("auto-update");
    let local_png: Option<String> = matches.opt_str("save-image");
    let purpleair_creds: Option<purpleair::Credentials> =
        matches.opt_str("purpleair-credentials-file").map(
            |file| purpleair::credentials_from_file(file)
                .expect("while reading purpleair-credentials-file"));
    let mta_bustime_creds: Option<String> =
        matches.opt_str("mta-bustime-credentials-file").map(
            |file| std::fs::read_to_string(file)
                .expect("while reading purpleair-credentials-file"));

    info!("Running with config: display={} one-shot={} debug-port={:?} auto-update={} local-png={:?}, purpleair-credentials={:?} mta-bustime-credentials={:?}", display, one_shot, debug_port, auto_update, local_png, purpleair_creds, mta_bustime_creds);

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
        match ttdash.one_iteration(display, local_png.as_ref().map(String::as_ref), &prev_processed_data, auto_update, purpleair_creds.as_ref(), mta_bustime_creds.as_ref()) {
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
