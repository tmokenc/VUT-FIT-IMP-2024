use embedded_hal::i2c::I2c;
use heapless::String;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, Ssd1306};

use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{
        ascii::{FONT_5X8, FONT_6X10},
        MonoTextStyle,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};

use core::fmt::Write as _;

const BOARD_OFFSET_X: i16 = 8;
const BOARD_OFFSET_Y: i16 = 26;
const NEXT_PIECE_OFFSET_X: i16 = 42;
const NEXT_PIECE_OFFSET_Y: i16 = 10;

pub struct Display<I2C, const SIZE_MUL: i16> {
    handle: Ssd1306<I2CInterface<I2C>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
}

impl<I2C: I2c, const SIZE_MUL: i16> Display<I2C, SIZE_MUL> {
    pub fn init(i2c: I2C) -> Self {
        let interface = ssd1306::I2CDisplayInterface::new(i2c);
        let mut handle = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate270)
            .into_buffered_graphics_mode();

        handle.init().unwrap();

        Self { handle }
    }

    pub fn flush(&mut self) {
        self.handle.flush().unwrap();
    }

    pub fn draw_start_screen(&mut self) {
        let raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("../logo.raw"), 64);

        let im = Image::new(&raw, Point::new(0, 0));

        let welcome = Text::with_alignment(
            "Tetris\nIMP 2024\nxnguye27\n\nPress",
            Point::new(32, 80),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            Alignment::Center,
        );

        im.draw(&mut self.handle).unwrap();
        welcome.draw(&mut self.handle).unwrap();
        self.flush();
    }

    pub fn draw_board(&mut self, width: i16, height: i16) {
        self.handle.clear_buffer();

        let style = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(1)
            .fill_color(BinaryColor::Off)
            .build();

        Rectangle::new(
            Point::new(BOARD_OFFSET_X as i32 - 1, BOARD_OFFSET_Y as i32 - 1),
            Size::new(
                (width * SIZE_MUL) as u32 + 2,
                (height * SIZE_MUL) as u32 + 2,
            ),
        )
        .into_styled(style)
        .draw(&mut self.handle)
        .unwrap();

        Text::with_alignment(
            "Next",
            Point::new(NEXT_PIECE_OFFSET_X as i32, 5),
            MonoTextStyle::new(&FONT_5X8, BinaryColor::On),
            Alignment::Left,
        )
        .draw(&mut self.handle)
        .unwrap();
    }

    pub fn draw_piece(&mut self, dx: i16, dy: i16, on: bool) {
        let block = Rectangle::new(
            Point::new(
                (dx * SIZE_MUL + BOARD_OFFSET_X) as i32,
                (dy * SIZE_MUL + BOARD_OFFSET_Y) as i32,
            ),
            Size::new(SIZE_MUL as u32, SIZE_MUL as u32),
        );

        let style = PrimitiveStyleBuilder::new()
            .fill_color(if on {
                BinaryColor::On
            } else {
                BinaryColor::Off
            })
            .build();

        block.into_styled(style).draw(&mut self.handle).unwrap();
    }

    pub fn draw_next_piece(&mut self, dx: i16, dy: i16) {
        Rectangle::new(
            Point::new(
                (dx * SIZE_MUL + NEXT_PIECE_OFFSET_X) as i32,
                (dy * SIZE_MUL + NEXT_PIECE_OFFSET_Y) as i32,
            ),
            Size::new(SIZE_MUL as u32, SIZE_MUL as u32),
        )
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(BinaryColor::On)
                .build(),
        )
        .draw(&mut self.handle)
        .unwrap();
    }

    pub fn draw_score(&mut self, score: u64) {
        let mut score_fmt: String<11> = String::new();

        write!(&mut score_fmt, "Score\n{}", score).unwrap();

        Text::with_alignment(
            &*score_fmt,
            Point::new(20, 8),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            Alignment::Center,
        )
        .draw(&mut self.handle)
        .unwrap();
    }

    pub fn draw_game_over(&mut self, score: u64) {
        self.handle.clear_buffer();

        let mut score_fmt: String<20> = String::new();

        write!(&mut score_fmt, "Score\n{}", score).unwrap();

        let score = Text::with_alignment(
            &*score_fmt,
            Point::new(32, 60),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            Alignment::Center,
        );

        score.draw(&mut self.handle).unwrap();
        self.flush();
    }
}
