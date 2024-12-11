#![no_std]
#![no_main]

mod bgm;
mod display;
mod input;
mod tetris;

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

use core::cell::RefCell;
use core::mem;
use cortex_m::prelude::_embedded_hal_adc_OneShot;
use critical_section::Mutex;
use display::Display;
use embedded_hal::delay::DelayNs as _;
use embedded_hal::digital::StatefulOutputPin;
use embedded_hal::pwm::SetDutyCycle as _;
use rp235x_hal as hal;

use hal::fugit::RateExtU32;
use hal::gpio;
use hal::multicore::{Multicore, Stack};
use hal::pac::interrupt;
use hal::pwm::{Slice, SliceId, ValidSliceMode};
use hal::rosc::{self, RingOscillator};

use input::{Button, Input, Joystick, JoystickState};
use tetris::{BoardUpdate, Cell, Rotation, State as GameState, Tetris, Tetromino};

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Refresh rate of the game in nanoseconds
/// one ADC sampling takes 92ns for each input, so we subtract 2 of them (for the joystick)
/// from the refresh rate
const REFRESH_RATE_NS: u32 = 1_000_000_000 / 60 - 4000;
const TETRIS_WIDTH: usize = 10;
const TETRIS_HEIGHT: usize = 20;

/// Volume of the buzzer, or duty cycle of the PWM
const VOLUME: u8 = 1;
const COMMAND_PLAY: u32 = 0x1;
const COMMAND_STOP: u32 = 0x0;

/// Declare a memory to be used by core 1
static mut CORE1_STACK: Stack<4096> = Stack::new();

struct State {
    game: Tetris<TETRIS_WIDTH, TETRIS_HEIGHT, RingOscillator<rosc::Enabled>>,
    board_updated: BoardUpdate<16>,
    last_move_down: hal::timer::Instant,
}

struct Buttons {
    pub joystick_btn: Button<gpio::bank0::Gpio22>,
    pub timer: hal::Timer<hal::timer::CopyableTimer0>,
}

struct InputHandleTools {
    led: gpio::Pin<gpio::bank0::Gpio25, gpio::FunctionSioOutput, gpio::PullNone>,
    timer: hal::Timer<hal::timer::CopyableTimer0>,
}

static GLOBAL_STATE: Mutex<RefCell<State>> = Mutex::new(RefCell::new(State {
    game: Tetris::new(),
    board_updated: BoardUpdate::Full,
    last_move_down: hal::timer::Instant::from_ticks(0),
}));

static GLOBAL_BUTTONS: Mutex<RefCell<Option<Buttons>>> = Mutex::new(RefCell::new(None));
// static GLOBAL_JOYSTICK: Mutex<RefCell<Option<Joystick>>> = Mutex::new(RefCell::new(None));
static GLOBAL_INPUT_HANDLE_TOOLS: Mutex<RefCell<Option<InputHandleTools>>> =
    Mutex::new(RefCell::new(None));

/// Entry point to our bare-metal application.
///
/// The `#[hal::entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables and the spinlock are initialised.
///
/// The function configures the rp235x peripherals, then toggles a GPIO pin in
/// an infinite loop. If there is an LED connected to that pin, it will blink.
#[hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    // The single-cycle I/O block controls our GPIO pins
    let mut sio = hal::Sio::new(pac.SIO);

    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // Spawn core 1 for background music handle
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let timer_1 = timer.clone();

    core1
        .spawn(unsafe { &mut CORE1_STACK.mem }, move || {
            core1_task(timer_1);
        })
        .unwrap();

    // Set the pins to their default state
    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda_pin: gpio::Pin<_, gpio::FunctionI2C, _> = pins.gpio20.reconfigure();
    let scl_pin: gpio::Pin<_, gpio::FunctionI2C, _> = pins.gpio21.reconfigure();

    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut display: Display<_, 5> = Display::init(i2c);
    let rnd = RingOscillator::new(pac.ROSC).initialize();
    let mut adc = hal::adc::Adc::new(pac.ADC, &mut pac.RESETS);

    // Onboard LED
    let led = pins.gpio25.reconfigure();
    let buttons = Buttons {
        joystick_btn: input::Button::new(pins.gpio22.reconfigure()),
        timer: timer.clone(),
    };
    let mut joystick_x = hal::adc::AdcPin::new(pins.gpio27.into_floating_input()).unwrap();
    let mut joystick_y = hal::adc::AdcPin::new(pins.gpio26.into_floating_input()).unwrap();

    let mut joystick_handle = Joystick::new(
        adc.read(&mut joystick_y).unwrap(),
        adc.read(&mut joystick_x).unwrap(),
    );

    // Initialize the global states
    critical_section::with(|cs| {
        GLOBAL_STATE.borrow(cs).borrow_mut().game.set_rng(rnd);
        GLOBAL_BUTTONS.borrow(cs).replace(Some(buttons));
        // GLOBAL_JOYSTICK.borrow(cs).replace(Some(joystick));
        GLOBAL_INPUT_HANDLE_TOOLS
            .borrow(cs)
            .replace(Some(InputHandleTools { led, timer }));
    });

    // for it to take its tools due to the safety of its static mut
    // the JoystickState::Center is ignored case, so no input action will be taken
    input_handler(Input::Joystick(JoystickState::Center));

    // Enable interrupts
    unsafe {
        cortex_m::peripheral::NVIC::unmask(hal::pac::Interrupt::IO_IRQ_BANK0);
    }

    loop {
        // Poll joystick first
        let joystick_x = adc.read(&mut joystick_x).unwrap();
        let joystick_y = adc.read(&mut joystick_y).unwrap();

        if let Some(state) = joystick_handle.state_from(joystick_y, joystick_x) {
            input_handler(Input::Joystick(state));
        }

        critical_section::with(|cs| {
            let mut state = GLOBAL_STATE.borrow(cs).borrow_mut();

            if state.game.is_playing() {
                let instant = timer.get_counter();
                if let Some(duration) = instant.checked_duration_since(state.last_move_down) {
                    if duration.to_millis() >= state.game.drop_speed() {
                        let board_update = state.game.act(tetris::Action::SoftDrop);
                        state.board_updated.merge(board_update);
                        state.last_move_down = instant;
                    }
                }
            }

            match mem::take(&mut state.board_updated) {
                BoardUpdate::None => return,
                BoardUpdate::Partial(data) => {
                    for (coord, cell) in data {
                        display.draw_piece(coord.x, coord.y, cell == Cell::Occured);
                    }

                    display.flush();
                    return;
                }
                BoardUpdate::Full => (), // Handle full update below
            }

            let current_tetromino_blocks = state.game.get_current_tetromino_position();

            match &state.game.state {
                GameState::New => display.draw_start_screen(),
                GameState::GameOver { score } => {
                    display.draw_game_over(*score);
                    sio.fifo.write(COMMAND_STOP);
                }
                GameState::Playing { score, queue, .. } => {
                    display.draw_board(TETRIS_WIDTH as i16, TETRIS_HEIGHT as i16);
                    display.draw_score(*score);

                    for pixel in state.game.board.iter() {
                        display.draw_piece(pixel.x, pixel.y, true);
                    }

                    for pixel in current_tetromino_blocks {
                        display.draw_piece(pixel.x, pixel.y, true);
                    }

                    let next_piece = queue.peek();
                    let next_piece_blocks = tetris::get_tetromino_blocks(
                        next_piece,
                        if matches!(next_piece, Tetromino::I | Tetromino::L | Tetromino::J) {
                            Rotation::Left
                        } else {
                            Rotation::default()
                        },
                    );

                    for block in next_piece_blocks {
                        display.draw_next_piece(block.x, block.y);
                    }

                    display.flush();
                    sio.fifo.write(COMMAND_PLAY);
                }
            }
        });

        // let duration = timer.get_counter().checked_duration_since(now).unwrap();
        // let remaining_time = REFRESH_RATE_NS - duration.to_nanos() as u32;
        timer.delay_ns(REFRESH_RATE_NS);
    }
}

fn input_handler(input: input::Input) {
    static mut TOOLS: Option<InputHandleTools> = None;

    // Safety: this only run once right after the initialization and is guard by the critical
    // section
    unsafe {
        if TOOLS.is_none() {
            critical_section::with(|cs| {
                TOOLS = GLOBAL_INPUT_HANDLE_TOOLS.borrow(cs).take();
            });
        }
    }

    // Safety: After the first run, TOOLS will always be Some
    let Some(ref mut tools) = (unsafe { TOOLS.as_mut() }) else {
        return;
    };

    tools.led.toggle().unwrap();

    let action = match input {
        Input::JoystickButton => Some(tetris::Action::HardDrop),
        Input::Joystick(JoystickState::Center) => None,
        Input::Joystick(JoystickState::Down) => Some(tetris::Action::SoftDrop),
        Input::Joystick(JoystickState::Left) => Some(tetris::Action::MoveLeft),
        Input::Joystick(JoystickState::Right) => Some(tetris::Action::MoveRight),
        Input::Joystick(JoystickState::TopLeft) => Some(tetris::Action::Rotate),
        Input::Joystick(JoystickState::TopRight) => Some(tetris::Action::Rotate),
    };

    if let Some(action) = action {
        critical_section::with(move |cs| {
            let mut state = GLOBAL_STATE.borrow(cs).borrow_mut();
            if !state.game.is_playing() && action == tetris::Action::HardDrop {
                state.game.start();
                state.board_updated = BoardUpdate::Full;
                state.last_move_down = tools.timer.get_counter();
            } else {
                let board_update = state.game.act(action);
                state.board_updated.merge(board_update);
                if action == tetris::Action::SoftDrop {
                    state.last_move_down = tools.timer.get_counter();
                }
            }
        });
    }
}

/// Core 1 task to play the background music
/// This will listen to the command from the main core to play or stop the music
fn core1_task(mut timer: hal::Timer<hal::timer::CopyableTimer0>) {
    let mut pac = unsafe { hal::pac::Peripherals::steal() };
    let mut sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Init PWMs
    let pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

    // Configure PWM4
    let mut pwm = pwm_slices.pwm0;
    pwm.set_ph_correct();
    pwm.enable();

    pwm.channel_b.output_to(pins.gpio1);

    loop {
        if sio.fifo.read_blocking() != COMMAND_PLAY {
            continue;
        }

        // Got the play command from the main core
        for (note, duration) in bgm::melody() {
            play_note(&mut pwm, note);
            timer.delay_ms(duration - bgm::SILENT_DURATION);
            play_note(&mut pwm, bgm::Note::Rest);
            timer.delay_ms(bgm::SILENT_DURATION);

            // Check for stop command
            if sio.fifo.read() == Some(COMMAND_STOP) {
                // Got the stop command from the main core
                break;
            }
        }
    }
}

fn play_note<I: SliceId, M: ValidSliceMode<I>>(pwm: &mut Slice<I, M>, note: bgm::Note) {
    let frequency = note.frequency();
    pwm.set_div_int(frequency.clk_div);
    pwm.set_top(frequency.cnt);
    pwm.set_counter(0);
    pwm.channel_b.set_duty_cycle_percent(VOLUME).unwrap();
}

#[interrupt]
fn IO_IRQ_BANK0() {
    static mut BUTTONS: Option<Buttons> = None;

    if BUTTONS.is_none() {
        critical_section::with(|cs| {
            *BUTTONS = GLOBAL_BUTTONS.borrow(cs).take();
        });
    }

    let Some(buttons) = BUTTONS else {
        return;
    };

    let now = buttons.timer.get_counter();
    let maybe_input = buttons
        .joystick_btn
        .interrupted(now)
        .then_some(Input::JoystickButton);

    if let Some(input) = maybe_input {
        crate::input_handler(input);
    }
}

/// Program metadata for `picotool info`
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Tetris"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];
