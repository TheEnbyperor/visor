mod ws2812;

const COLUMNS: usize = 32;
const ROWS: usize = 16;
const NUM_LEDS: usize = ROWS * COLUMNS;

struct EyeImage([u8; 16]);
const EYE_IMAGE_WIDTH: usize = 7;
const EYE_SIDE_OFFSET: usize = 5;

include!(concat!(env!("OUT_DIR"), "/eye_image_data.rs"));

const TRANS_FLAG: [ws2812::Color; ROWS] = [
    ws2812::Color::new(10, 30, 38),
    ws2812::Color::new(10, 30, 38),
    ws2812::Color::new(10, 30, 38),
    ws2812::Color::new(38, 10, 15),
    ws2812::Color::new(38, 10, 15),
    ws2812::Color::new(38, 10, 15),
    ws2812::Color::new(16, 16, 16),
    ws2812::Color::new(16, 16, 16),
    ws2812::Color::new(16, 16, 16),
    ws2812::Color::new(16, 16, 16),
    ws2812::Color::new(38, 10, 15),
    ws2812::Color::new(38, 10, 15),
    ws2812::Color::new(38, 10, 15),
    ws2812::Color::new(10, 30, 38),
    ws2812::Color::new(10, 30, 38),
    ws2812::Color::new(10, 30, 38),
];

fn set_pixel(leds: &mut[ws2812::Color], x: usize, y: usize, color: ws2812::Color) {
    let pixel = if x % 2 == 0 {
        (ROWS - y - 1) + (x * ROWS)
    } else {
        y + (x * ROWS)
    };
    leds[pixel] = color
}

fn write_eye(leds: &mut[ws2812::Color], img: &EyeImage, offset_x: usize, color: ws2812::Color, flip: bool) {
    for (y, img_row) in img.0.iter().enumerate() {
        for x in 0..EYE_IMAGE_WIDTH {
            let bit = x + (8 - EYE_IMAGE_WIDTH);
            let bit = if flip {
                8 - bit
            } else {
                bit
            };
            let pixel_color = if (img_row & (1 << bit)) != 0 {
                color
            } else {
                ws2812::Color::new(0, 0, 0)
            };
            set_pixel(leds, x + offset_x, y, pixel_color);
        }
    }
}

fn write_eyes(leds: &mut[ws2812::Color], img: &EyeImage, color: ws2812::Color) {
    write_eye(leds, img, EYE_SIDE_OFFSET, color, false);
    write_eye(leds, img, COLUMNS - EYE_SIDE_OFFSET - EYE_IMAGE_WIDTH, color, true);
}

fn write_flag(leds: &mut[ws2812::Color], flag: [ws2812::Color; ROWS]) {
    for (y, pixel) in flag.iter().enumerate() {
        set_pixel(leds, 0, y, *pixel);
        set_pixel(leds, COLUMNS - 1, y, *pixel);
    }
}

#[embassy_executor::task]
pub async fn main() {
    let rmt = unsafe { esp_hal::peripherals::RMT::steal() };
    let display_pin = unsafe { esp_hal::peripherals::GPIO46::steal() };
    let mut ws2812 = ws2812::Ws2812Driver::new(rmt, display_pin).unwrap();

    let mut leds = [ws2812::Color::default(); NUM_LEDS];

    write_flag(&mut leds, TRANS_FLAG);

    loop {
        write_eyes(&mut leds, &EYE_HAPPY, ws2812::Color::new(255, 0, 0));
        ws2812.write(&leds).await.unwrap();
        embassy_time::Timer::after_millis(1000).await;

        // write_eyes(&mut leds, &EYE_HAPPY, ws2812::Color::new(255, 0, 0));
        // ws2812.write(&leds).await.unwrap();
        // embassy_time::Timer::after_millis(1000).await;
    }
}