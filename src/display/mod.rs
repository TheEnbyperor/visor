mod ws2812;

pub static NEXT_EYE: embassy_sync::signal::Signal<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    ()
> = embassy_sync::signal::Signal::new();

const COLUMNS: usize = 32;
const ROWS: usize = 16;
const NUM_LEDS: usize = ROWS * COLUMNS;

struct EyeImage([u16; 16]);

include!(concat!(env!("OUT_DIR"), "/eye_image_data.rs"));

const LIGHT_BLUE: ws2812::Color = ws2812::Color::new(30, 150, 250);
const LIGHT_PINK: ws2812::Color = ws2812::Color::new(245, 50, 75);
const WHITE: ws2812::Color = ws2812::Color::new(255, 255, 255);
const TRANS_FLAG: [ws2812::Color; ROWS] = [
    LIGHT_BLUE,
    LIGHT_BLUE,
    LIGHT_BLUE,
    LIGHT_PINK,
    LIGHT_PINK,
    LIGHT_PINK,
    WHITE,
    WHITE,
    WHITE,
    WHITE,
    LIGHT_PINK,
    LIGHT_PINK,
    LIGHT_PINK,
    LIGHT_BLUE,
    LIGHT_BLUE,
    LIGHT_BLUE,
];

const EYES: [EyeImage; 5] = [
    EYE_NORMAL,
    EYE_HAPPY,
    EYE_ANGRY,
    EYE_ANGLE,
    EYE_BLUSH,
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
        for x in 0..u16::BITS as usize {
            let bit = if flip {
                u16::BITS as usize - x - 1
            } else {
                x
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
    write_eye(leds, img, 0, color, false);
    write_eye(leds, img, COLUMNS - (u16::BITS as usize), color, true);
}

fn write_flag(leds: &mut[ws2812::Color], flag: [ws2812::Color; ROWS]) {
    for (y, pixel) in flag.iter().enumerate() {
        let pixel = pixel.apply_brightness(2);
        set_pixel(leds, 0, y, pixel);
        set_pixel(leds, COLUMNS - 1, y, pixel);
    }
}

#[embassy_executor::task]
pub async fn main() {
    let rmt = unsafe { esp_hal::peripherals::RMT::steal() };
    let display_pin = unsafe { esp_hal::peripherals::GPIO46::steal() };
    let mut ws2812 = ws2812::Ws2812Driver::new(rmt, display_pin).unwrap();

    let mut leds = [ws2812::Color::default(); NUM_LEDS];

    let mut eye_index = 0;
    let eye_color = ws2812::Color::new(255, 0, 0).apply_brightness(3);
    loop {
        write_eyes(&mut leds, &EYES[eye_index], eye_color);
        write_flag(&mut leds, TRANS_FLAG);
        ws2812.write(&leds).await.unwrap();

        NEXT_EYE.wait().await;
        eye_index = (eye_index + 1) % EYES.len();
    }
}
