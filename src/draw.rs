use rand::prelude::*;

use crate::{Rgb, SCREEN_HEIGHT, SCREEN_WIDTH};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black = 0x00,  // 0, 0, 0
    White = 0x01,  // 255, 255, 255
    Green = 0x02,  // 0, 255, 0
    Blue = 0x03,   // 0, 0, 255
    Red = 0x04,    // 255, 0, 0
    Yellow = 0x05, // 255, 255, 0
    Orange = 0x06, // 255, 170, 0
    Clean = 0x07,
}
impl Color {
    pub const fn all() -> &'static [Color] {
        &[
            Color::Clean,
            Color::Black,
            Color::White,
            Color::Green,
            Color::Blue,
            Color::Red,
            Color::Yellow,
            Color::Orange,
        ]
    }

    pub fn closest(pixel: Rgb) -> Color {
        Color::all()
            .iter()
            .map(|c| -> (f32, Color) {
                let [r, g, b] = c.as_rgb();
                let dr = pixel.r - r as f32;
                let dg = pixel.g as f32 - g as f32;
                let db = pixel.b as f32 - b as f32;
                let ed = dr * dr + dg * dg + db * db;
                (ed, *c)
            })
            .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
            .unwrap()
            .1
    }

    pub fn as_rgb(&self) -> [f32; 3] {
        const LOOKUP: &'static [[f32; 3]] = &[
            [0.0, 0.0, 0.0],
            [255.0, 255.0, 255.0],
            [0.0, 255.0, 0.0],
            [0.0, 0.0, 255.0],
            [255.0, 0.0, 0.0],
            [255.0, 255.0, 0.0],
            [255.0, 170.0, 0.0],
            [180.0, 180.0, 180.0],
        ];
        LOOKUP[*self as usize]
    }
}

pub trait Drawable {
    fn get_pixel(&self, x: u16, y: u16) -> Color;
}

pub struct SolidColor(pub Color);
pub struct RandomColors;
pub struct SequentialColors;
pub struct Partial<'a, D: Drawable> {
    pub color: Color,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub rest: &'a D,
}
pub struct PaperImage {
    pub data: [Color; SCREEN_HEIGHT as usize * SCREEN_WIDTH as usize],
}

impl Drawable for PaperImage {
    fn get_pixel(&self, x: u16, y: u16) -> Color {
        let x = x as usize;
        let y = y as usize;
        self.data[x + y * SCREEN_WIDTH as usize]
    }
}

impl<D: Drawable> Drawable for Partial<'_, D> {
    fn get_pixel(&self, x: u16, y: u16) -> Color {
        if x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h {
            self.color
        } else {
            self.rest.get_pixel(x, y)
        }
    }
}

impl Drawable for SequentialColors {
    fn get_pixel(&self, x: u16, y: u16) -> Color {
        let i = ((x / 10) + (y / 2)) % Color::all().len() as u16;
        Color::all()[i as usize]
    }
}

impl Drawable for RandomColors {
    fn get_pixel(&self, _x: u16, _y: u16) -> Color {
        static COLORS: &[Color] = &[
            Color::Black,
            Color::White,
            Color::Green,
            Color::Blue,
            Color::Red,
            Color::Yellow,
            Color::Orange,
            Color::Clean,
        ];

        unsafe { *COLORS.choose(&mut thread_rng()).unwrap_unchecked() }
    }
}

impl Drawable for SolidColor {
    fn get_pixel(&self, _x: u16, _y: u16) -> Color {
        self.0
    }
}
