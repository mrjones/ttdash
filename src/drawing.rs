extern crate chrono;
extern crate chrono_tz;
extern crate image;
extern crate imageproc;
extern crate rusttype;
extern crate std;

use crate::result;
use crate::subway;
use crate::weather;

pub struct Styles<'a> {
    pub font: rusttype::Font<'a>,
    pub font_bold: rusttype::Font<'a>,
    pub font_black: rusttype::Font<'a>,

    pub color_black: image::Luma<u8>,
    pub color_light_gray: image::Luma<u8>,
    pub color_dark_gray: image::Luma<u8>,
    pub color_white: image::Luma<u8>,
}

const EPD_WIDTH: usize = 640;
const EPD_HEIGHT: usize = 384;

pub fn generate_image(data: &subway::ProcessedData,
                      weather_display: Option<&weather::WeatherDisplay>,
                      version: Option<String>,
                      styles: &Styles) -> result::TTDashResult<image::GrayImage> {
    let mut imgbuf = image::GrayImage::new(EPD_WIDTH as u32, EPD_HEIGHT as u32);

    draw_subway_arrivals(&mut imgbuf, styles, data);

    if weather_display.is_some() {
        draw_weather(&mut imgbuf, styles, weather_display.unwrap())?;
    }

    draw_version(&mut imgbuf, styles, version.unwrap_or("UNKNOWN VERSION".to_string()).as_ref());

    return Ok(imgbuf);
}

fn draw_version(imgbuf: &mut image::GrayImage, styles: &Styles, version: &str) {
    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 570, 5, scale(15.0), &styles.font, version);
}


fn draw_subway_arrivals(imgbuf: &mut image::GrayImage, styles: &Styles, data: &subway::ProcessedData) {
    let now = chrono::Utc::now().timestamp();

    imageproc::drawing::draw_filled_rect_mut(imgbuf, imageproc::rect::Rect::at(0,0).of_size(EPD_WIDTH as u32, EPD_HEIGHT as u32), styles.color_white);

    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 10, 0, scale(50.0), &styles.font_bold, &data.station_name);
    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 10, 40, scale(40.0), &styles.font, "Manhattan");

    imageproc::drawing::draw_line_segment_mut(imgbuf, (10.0, 85.0), (EPD_HEIGHT as f32 - 10.0, 85.0), styles.color_black);

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
                draw_subway_line_emblem(imgbuf, &big_line, 30, 115, 20, styles);
            }
        },
        _ => {},
    }

    let mut y = 90;
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


    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 10, 290, scale(40.0), &styles.font, "Bay Ridge");
    imageproc::drawing::draw_line_segment_mut(imgbuf, (10.0, 335.0), (EPD_HEIGHT as f32 - 10.0, 335.0), styles.color_black);

    let outbound_text: String = if data.upcoming_outbound_trains.is_empty() {
        "NO BAY RIDGE TRAINS".to_string()
    } else {
        data.upcoming_outbound_trains.iter()
                .take(5)
                .filter(|(_, line)| line == "R")
                .map(|(ts, line)|
                     if line == "R" {
                         countdown_summary(now, *ts)
                     } else {
                         format!("{} ({})", countdown_summary(now, *ts), line)
                     })
                .collect::<Vec<String>>()
                .join(", ")
    };

    imageproc::drawing::draw_text_mut(imgbuf, styles.color_black, 10, 340, scale(40.0), &styles.font_bold, &outbound_text);

}

pub fn countdown_summary(now_ts: i64, arrival_ts: i64) -> String {
    let wait_seconds = arrival_ts - now_ts;

    if wait_seconds < 60 {
        return "<1".to_string();
    }
    return format!("{}", wait_seconds / 60);
}

fn draw_raindrop(x: i32, y: i32, height: i32, imgbuf: &mut image::GrayImage, styles: &Styles) -> result::TTDashResult<()> {
    let x = x as f32;
    let y = y as f32;
    let height = height as f32;
    let bottom_horizontal_control_offset = height / 2.0;
    let top_vertical_control_offset = height / 3.0;

    imageproc::drawing::draw_cubic_bezier_curve_mut(
        imgbuf,
        (x, y),
        (x, y + height),
        (x, y + top_vertical_control_offset),
        (x + bottom_horizontal_control_offset, y + height),
        styles.color_black);

    imageproc::drawing::draw_cubic_bezier_curve_mut(
        imgbuf,
        (x, y),
        (x, y + height),
        (x, y + top_vertical_control_offset),
        (x - bottom_horizontal_control_offset, y + height),
        styles.color_black);

    return Ok(());
}

fn draw_daily_forecast(left_x: i32, top_y: i32, imgbuf: &mut image::GrayImage, styles: &Styles, weather_display: &weather::WeatherDisplay) -> result::TTDashResult<()> {
    use chrono::Datelike;

    let precip_bar_max_height = 50;

    let hour_width: u32 = 2;
    let day_width: u32 = 24 * hour_width + 5;

    let day_labels = vec!["S", "M", "T", "W", "R", "F", "S"];
    let first_entry = weather_display.days.iter().nth(0).ok_or(
        result::make_error("missing first entry"))?;
    let first_date = first_entry.0;

    let left_offset = 10; // Space for raindrop

    for (date, info) in weather_display.days.iter().take(4) {
        let day_count = date.num_days_from_ce() - first_date.num_days_from_ce();
        let day_label = day_labels.get(date.weekday().num_days_from_sunday() as usize).unwrap_or(&"?").to_string();

        // Day Label (SMTWRFS)
        imageproc::drawing::draw_text_mut(
            imgbuf, styles.color_black,
            /* x= */ (left_x + left_offset) as u32 + day_count as u32 * day_width as u32 + (8 * hour_width),
            /* y= */ (top_y) as u32,
            scale(40.0), &styles.font_bold, &day_label);

        // High temperature
        imageproc::drawing::draw_text_mut(
            imgbuf, styles.color_black,
            /* x = */ (left_x + left_offset + day_count * day_width as i32 + (4 * hour_width as i32)) as u32,
            /* y = */ (top_y + 30) as u32,
            scale(45.0), &styles.font, &format!("{:.0}", info.max_t));

        // Precip bars
        let precip_bar_top = top_y + 75;
        let precip_bar_width = (4 * day_width) as i32;

        draw_raindrop(left_x, top_y + 90, 25, imgbuf, styles)?;
        imageproc::drawing::draw_line_segment_mut(
            imgbuf,
            ((left_x + left_offset) as f32, precip_bar_top as f32),
            ((left_x + left_offset + precip_bar_width) as f32, precip_bar_top as f32),
            styles.color_black);
        imageproc::drawing::draw_line_segment_mut(
            imgbuf,
            ((left_x + left_offset) as f32, (precip_bar_top + precip_bar_max_height) as f32),
            ((left_x + left_offset + precip_bar_width) as f32, (precip_bar_top + precip_bar_max_height) as f32),
            styles.color_black);

        for (hour, precip_prob) in &info.precip_by_hour {
            let bar_height = std::cmp::max(1, (precip_bar_max_height as f32 * (*precip_prob / 100.0)) as u32);

            imageproc::drawing::draw_filled_rect_mut(
                imgbuf,
                imageproc::rect::Rect::at(
                    /* x= */ left_x + left_offset + day_count as i32 * day_width as i32 + *hour as i32 * hour_width as i32,
                    /* y= */ precip_bar_top + precip_bar_max_height - bar_height as i32)
                    .of_size(hour_width, bar_height),
                styles.color_black);
        }
    }

    return Ok(());
}

fn draw_weather(imgbuf: &mut image::GrayImage, styles: &Styles, weather_display: &weather::WeatherDisplay) -> result::TTDashResult<()> {
    let left_x: i32 = 400;
    let top_y: i32 = 00;

    draw_daily_forecast(left_x, top_y + 240, imgbuf, styles, weather_display)?;

    let first_entry = weather_display.days.iter().nth(0).ok_or(
        result::make_error("missing first entry"))?;
    let first_info = first_entry.1;

    imageproc::drawing::draw_text_mut(
        imgbuf, styles.color_black,
        /* x= */ (left_x + 40) as u32, /* y= */ top_y as u32,
        scale(140.0),
        &styles.font_black, &format!("{:.0}°", weather_display.current_t));

    imageproc::drawing::draw_text_mut(
        imgbuf, styles.color_black,
        /* x= */ left_x as u32,
        /* y= */ (top_y + 110) as u32,
        scale(80.0), &styles.font_bold,
        &format!("{}° / {}°", first_info.min_t, first_info.max_t));

    // Dew point ranges:
    // [0]  < 55: Pleasant
    // [1] 56-60: Comfortable
    // [2] 61-65: Sticky
    // [3] 66-70: Uncomfortable
    // [4] 71-75: Oppressive
    // [5]  > 76: Miserable
    let dew_point = first_info.max_dew_point as i32;
    let dew_point_bucket =
        std::cmp::min(5, std::cmp::max(0, (dew_point - 50) / 5));

    let dp_box_width = 35;
    let dp_box_height = 15;
    let dp_box_gap = 10;
    for i in 0..5 as i32 {
        let rect = imageproc::rect::Rect::at(left_x + i * (dp_box_width + dp_box_gap), top_y + 200).of_size(dp_box_width as u32, dp_box_height as u32);
        if i < dew_point_bucket {
            imageproc::drawing::draw_filled_rect_mut(imgbuf, rect, styles.color_black);
        } else {
            imageproc::drawing::draw_hollow_rect_mut(imgbuf, rect, styles.color_black);
        }
    }

    return Ok(());
}

fn scale(s: f32) -> rusttype::Scale {
    return rusttype::Scale{x: s, y: s};
}

fn draw_subway_line_emblem(imgbuf: &mut image::GrayImage, letter: &str, x: u32, y: u32, radius: u32, styles: &Styles) {
    imageproc::drawing::draw_filled_circle_mut(imgbuf, (x as i32, y as i32), (radius + 2)as i32, styles.color_white);
    imageproc::drawing::draw_filled_circle_mut(imgbuf, (x as i32, y as i32), radius as i32, styles.color_black);
    imageproc::drawing::draw_text_mut(imgbuf, styles.color_white, x - (radius / 2) + 2, y - radius, scale((radius * 2) as f32), &styles.font_bold, letter);
}
