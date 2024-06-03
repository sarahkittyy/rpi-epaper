use std::{
    env,
    error::Error,
    ops::{AddAssign, Sub},
    thread::sleep,
    time::{Duration, Instant},
};

use draw::PaperImage;
use rppal::{
    gpio::{Gpio, InputPin, OutputPin},
    spi::{self, Bus, Mode, SlaveSelect, Spi},
};

mod cmd;
use crate::{
    cmd::{Command, Init},
    draw::Color,
};

mod draw;

const _DIN: u8 = 10; // spi0 mosi
const _CLK: u8 = 11; // spi0 sclk
const _CS: u8 = 8; // spi0 ce0 (chip select)
const DC: u8 = 25; // data (high)/command (low)
const BUSY: u8 = 24;
const RESET: u8 = 17;
pub const SCREEN_WIDTH: u16 = 600;
pub const SCREEN_HEIGHT: u16 = 448;

pub struct EPaper {
    spi: Spi,
    dc: OutputPin,
    busy: InputPin,
    reset: OutputPin,
}

impl EPaper {
    pub fn init(spi: Spi, dc: OutputPin, busy: InputPin, reset: OutputPin) -> Self {
        let mut s = Self {
            spi,
            dc,
            busy,
            reset,
        };
        s.reset();
        s
    }

    pub fn reset(&mut self) {
        self.reset.set_high();
        sleep(Duration::from_millis(600));
        self.reset.set_low();
        sleep(Duration::from_millis(2));
        self.reset.set_high();
        sleep(Duration::from_millis(200));
    }
}

pub trait SpiDevice {
    fn send_cmd(&mut self, cmd: u8) -> spi::Result<()>;
    fn send_data(&mut self, data: &[u8]) -> spi::Result<()>;
    fn wait_busy_high(&self);
    fn wait_busy_low(&self);
}

impl SpiDevice for EPaper {
    fn send_cmd(&mut self, cmd: u8) -> spi::Result<()> {
        self.dc.set_low();
        self.spi.write(&[cmd])?;
        Ok(())
    }

    fn send_data(&mut self, data: &[u8]) -> spi::Result<()> {
        self.dc.set_high();
        self.spi.write(data)?;
        Ok(())
    }

    fn wait_busy_high(&self) {
        while self.busy.is_low() {
            sleep(Duration::from_millis(10));
        }
    }

    fn wait_busy_low(&self) {
        while self.busy.is_high() {
            sleep(Duration::from_millis(10));
        }
    }
}

#[derive(Clone, Copy)]
pub struct Rgb {
    r: f32,
    g: f32,
    b: f32,
}

impl From<bmp::Pixel> for Rgb {
    fn from(value: bmp::Pixel) -> Self {
        Self {
            r: value.r.into(),
            g: value.g.into(),
            b: value.b.into(),
        }
    }
}

impl From<Color> for Rgb {
    fn from(value: Color) -> Self {
        let [r, g, b] = value.as_rgb();
        Self { r, g, b }
    }
}

impl From<Rgb> for bmp::Pixel {
    fn from(value: Rgb) -> Self {
        bmp::Pixel {
            r: value.r.clamp(0.0, 255.0) as u8,
            g: value.g.clamp(0.0, 255.0) as u8,
            b: value.b.clamp(0.0, 255.0) as u8,
        }
    }
}

impl AddAssign for Rgb {
    fn add_assign(&mut self, rhs: Rgb) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
    }
}

impl Sub for Rgb {
    type Output = Rgb;
    fn sub(self, rhs: Rgb) -> Rgb {
        Rgb {
            r: self.r - rhs.r,
            g: self.g - rhs.g,
            b: self.b - rhs.b,
        }
    }
}

fn floyd_steinberg_dither(img: &bmp::Image) -> PaperImage {
    // weight is out of 16
    fn diffuse_error(error: Rgb, weight: f32) -> Rgb {
        Rgb {
            r: error.r * weight / 16.0,
            g: error.g * weight / 16.0,
            b: error.b * weight / 16.0,
        }
    }
    // create temp pixel data to modify in place during algo
    let mut input = [Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    }; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize];
    for x in 0..SCREEN_WIDTH as u32 {
        for y in 0..SCREEN_HEIGHT as u32 {
            input[x as usize + y as usize * SCREEN_WIDTH as usize] = img.get_pixel(x, y).into();
        }
    }
    let mut out = [Color::Clean; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize];
    let width = SCREEN_WIDTH as usize;
    let height = SCREEN_HEIGHT as usize;
    let idx = |x, y| -> usize { x + y * width };
    for y in 0..SCREEN_HEIGHT as usize {
        for x in 0..SCREEN_WIDTH as usize {
            let oldpixel = input[idx(x, y)];
            let newpixel = Color::closest(oldpixel.into());
            out[x + y * width] = newpixel;
            let error = Rgb::from(oldpixel) - Rgb::from(newpixel);
            // todo: clean up bounds check
            if x + 1 < width {
                input[idx(x + 1, y)] += diffuse_error(error, 7.0);
            }
            if x + 1 < width && y + 1 < height {
                input[idx(x + 1, y + 1)] += diffuse_error(error, 1.0);
            }
            if x != 0 && y + 1 < height {
                input[idx(x - 1, y + 1)] += diffuse_error(error, 3.0);
            }
            if y + 1 < height {
                input[idx(x, y + 1)] += diffuse_error(error, 5.0);
            }
        }
    }
    PaperImage { data: out }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let clean = args.next();

    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 5_000_000, Mode::Mode0)?;
    let dc = Gpio::new()?.get(DC)?.into_output();
    let busy = Gpio::new()?.get(BUSY)?.into_input();
    let reset = Gpio::new()?.get(RESET)?.into_output();
    let mut display = EPaper::init(spi, dc, busy, reset);

    let mut image_bmp: &'static [u8] = include_bytes!("image.bmp");

    let img = bmp::from_reader(&mut image_bmp)?;
    assert!(img.get_width() as u16 == SCREEN_WIDTH && img.get_height() as u16 == SCREEN_HEIGHT);

    println!("Reset display");
    display.reset();
    display.wait_busy_high();
    println!("Init display");
    Init.send(&mut display)?;
    let now = Instant::now();
    println!("Printing image");
    if clean.is_some_and(|c| c == "clean") {
        cmd::Draw(&draw::SolidColor(Color::Clean)).send(&mut display)?;
    } else {
        cmd::Draw(&floyd_steinberg_dither(&img)).send(&mut display)?;
    }
    println!("Took {:?}", now.elapsed());

    Ok(())
}
