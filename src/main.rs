// sudo apt-get install fonts-roboto libssl-dev
extern crate chrono;
extern crate chrono_tz;
extern crate getopts;
extern crate hex;
extern crate image;
extern crate imageproc;
extern crate md5;
extern crate nix;
extern crate protobuf;
extern crate reqwest;
extern crate rppal;
extern crate rusttype;
#[macro_use]
extern crate serde_derive;
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
        self.weather_display = Some(weather::get_weather_display()?);
        self.forecast_timestamp = *now;

        return Ok(());
    }

    fn one_iteration(&mut self, display: bool, png_out: Option<String>, prev_processed_data: &subway::ProcessedData) -> result::TTDashResult<subway::ProcessedData>{
        let processed_data = subway::fetch_and_process_data()?;

        // TODO(mrjones): Make this async or something?
        // TODO(mrjones): Don't fetch every time
        let now = chrono::Utc::now();
        if self.weather_display.is_none() || (now.timestamp() - self.forecast_timestamp.timestamp() > 60 * 30) {
            match self.update_weather(&now) {
                Ok(_) => {},
                Err(err) => println!("Error: {:?}", err),
            }
        }

        let imgbuf = drawing::generate_image(
            &processed_data,
            self.weather_display.as_ref(),
            &self.styles)?;

        if png_out.is_some() {
            let _ = imgbuf.save(png_out.unwrap())?;
        }

        let mut needs_redraw = false;

        if prev_processed_data.big_countdown != processed_data.big_countdown {
            println!("Updating bignum {:?} -> {:?}",
                     prev_processed_data.big_countdown,
                     processed_data.big_countdown);
            needs_redraw = true;
        } else if self.last_redraw.is_none() {
            // Probably never happens in practice?
            println!("Drawing for the first time.");
            needs_redraw = true;
        } else if self.last_redraw.is_some() {
            let seconds_since_redraw = now.timestamp() - self.last_redraw.unwrap().timestamp();
            needs_redraw = seconds_since_redraw > 60 * 30;
            if needs_redraw {
                println!("Redrawing since it's been too long.");
            }
        }

        if needs_redraw {
            if display {
                display::setup_and_display_image(&imgbuf)?;
            }
            self.last_redraw = Some(now);
        } else {
            println!("Not refreshing.");
            std::thread::sleep(std::time::Duration::from_secs(5));
        }

        return Ok(processed_data);
    }
}

fn main() {
    match update::local_version() {
        Ok(v) => println!("TTDASH VERSION {}.{}", v.major, v.minor),
        Err(_) => println!("NO VERSION"),
    }

    let args: Vec<String> = std::env::args().collect();
    println!("Command Line: {:?}", args);

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

    println!("Running. display={} one-shot={} debug-port={:?} auto-update={}", display, one_shot, debug_port, auto_update);

    let mut prev_processed_data = subway::ProcessedData::empty();
    let mut ttdash = TTDash::new();

    match debug_port {
        Some(port) => {
            std::thread::spawn(move || { debug::run_debug_server(&port); });
        },
        None => {},
    }

    if auto_update {
        assert!(update::updater_configured());

        let argv0 = std::env::args().nth(0).expect("argv0");
        let argv: Vec<String> = std::env::args().collect();
        println!("argv[0] = {:?}", argv0);
        match update::binary_update_available() {
            Some(version) => {
                println!("update available");
                update::upgrade_to(&version, &argv0, &argv).expect("Upgrade");
            },
            None => {
                println!("No update available");
            },
        }
    }

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
