//! Ported from https://github.com/rbirkby/picotetris/blob/master/song.cpp

const BPM: u32 = 144;
const WHOLE_NOTE: u32 = (60000 * 4) / BPM;
pub const SILENT_DURATION: u32 = WHOLE_NOTE / 64;

#[derive(Copy, Clone, PartialEq)]
pub enum Note {
    A4,   // 440 Hz,
    B4,   // 494 Hz,
    Gs4,  // 415 Hz,
    A5,   // 880 Hz,
    C5,   // 523 Hz,
    D5,   // 587 Hz,
    E5,   // 659 Hz,
    F5,   // 698 Hz,
    G5,   // 784 Hz,
    Gs5,  // 831 Hz,
    Rest, // 60000 Hz,
}

pub struct Frequency {
    pub clk_div: u8,
    pub cnt: u16,
}

impl Note {
    pub fn frequency(&self) -> Frequency {
        match self {
            Self::A4 => Frequency {
                clk_div: 10,
                cnt: 34091,
            },
            Self::B4 => Frequency {
                clk_div: 181,
                cnt: 1678,
            },
            Self::Gs4 => Frequency {
                clk_div: 11,
                cnt: 32835,
            },
            Self::C5 => Frequency {
                clk_div: 5,
                cnt: 57334,
            },
            Self::D5 => Frequency {
                clk_div: 9,
                cnt: 28377,
            },
            Self::E5 => Frequency {
                clk_div: 4,
                cnt: 56883,
            },
            Self::F5 => Frequency {
                clk_div: 6,
                cnt: 35793,
            },
            Self::G5 => Frequency {
                clk_div: 3,
                cnt: 63776,
            },
            Self::Gs5 => Frequency {
                clk_div: 5,
                cnt: 36118,
            },
            Self::A5 => Frequency {
                clk_div: 5,
                cnt: 34091,
            },
            Self::Rest => Frequency {
                clk_div: 1,
                cnt: 2500,
            },
        }
    }
}

use Note::*;

//Based on the arrangement at https://www.flutetunes.com/tunes.php?id=192
const TETRIS_BGM: &[(Note, u32, bool)] = &[
    (E5, 4, false),
    (B4, 8, false),
    (C5, 8, false),
    (D5, 4, false),
    (C5, 8, false),
    (B4, 8, false),
    (A4, 4, false),
    (A4, 8, false),
    (C5, 8, false),
    (E5, 4, false),
    (D5, 8, false),
    (C5, 8, false),
    (B4, 4, true),
    (C5, 8, false),
    (D5, 4, false),
    (E5, 4, false),
    (C5, 4, false),
    (A4, 4, false),
    (A4, 8, false),
    (A4, 4, false),
    (B4, 8, false),
    (C5, 8, false),
    (D5, 4, true),
    (F5, 8, false),
    (A5, 4, false),
    (G5, 8, false),
    (F5, 8, false),
    (E5, 4, true),
    (C5, 8, false),
    (E5, 4, false),
    (D5, 8, false),
    (C5, 8, false),
    (B4, 4, false),
    (B4, 8, false),
    (C5, 8, false),
    (D5, 4, false),
    (E5, 4, false),
    (C5, 4, false),
    (A4, 4, false),
    (A4, 4, false),
    (Rest, 4, false),
    (E5, 4, false),
    (B4, 8, false),
    (C5, 8, false),
    (D5, 4, false),
    (C5, 8, false),
    (B4, 8, false),
    (A4, 4, false),
    (A4, 8, false),
    (C5, 8, false),
    (E5, 4, false),
    (D5, 8, false),
    (C5, 8, false),
    (B4, 4, true),
    (C5, 8, false),
    (D5, 4, false),
    (E5, 4, false),
    (C5, 4, false),
    (A4, 4, false),
    (A4, 8, false),
    (A4, 4, false),
    (B4, 8, false),
    (C5, 8, false),
    (D5, 4, true),
    (F5, 8, false),
    (A5, 4, false),
    (G5, 8, false),
    (F5, 8, false),
    (E5, 4, true),
    (C5, 8, false),
    (E5, 4, false),
    (D5, 8, false),
    (C5, 8, false),
    (B4, 4, false),
    (B4, 8, false),
    (C5, 8, false),
    (D5, 4, false),
    (E5, 4, false),
    (C5, 4, false),
    (A4, 4, false),
    (A4, 4, false),
    (Rest, 4, false),
    (E5, 2, false),
    (C5, 2, false),
    (D5, 2, false),
    (B4, 2, false),
    (C5, 2, false),
    (A4, 2, false),
    (Gs4, 2, false),
    (B4, 4, false),
    (Rest, 8, false),
    (E5, 2, false),
    (C5, 2, false),
    (D5, 2, false),
    (B4, 2, false),
    (C5, 4, false),
    (E5, 4, false),
    (A5, 2, false),
    (Gs5, 2, false),
];

/// Returns an infinite iterator over the notes and its duration of the Tetris theme song.
pub fn melody() -> impl Iterator<Item = (Note, u32)> {
    TETRIS_BGM
        .into_iter()
        .map(|(note, divider, dotted)| {
            let mut duration = WHOLE_NOTE / divider;

            if *dotted {
                // dotted notes are 1.5x the duration of a regular note
                // so 4-dotted notes in the song is roughly equivalent to divider of 2.67 regular notes
                duration *= 3;
                duration /= 2;
            }

            (*note, duration)
        })
        .cycle()
}
