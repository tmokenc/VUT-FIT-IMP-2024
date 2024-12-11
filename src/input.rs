use crate::hal;
use core::cmp::Ordering;
use hal::gpio;

const DELAY_BETWEEN_INTERRUPTS: u64 = 130;
const JOYSTICK_DEADZONE: u32 = 1000;

#[derive(Clone, Copy, PartialEq)]
pub enum Input {
    JoystickButton,
    Joystick(JoystickState),
}

pub struct Joystick {
    center_x: u16,
    center_y: u16,
    last_state: JoystickState,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum JoystickState {
    #[default]
    Center,
    Down,
    Left,
    Right,
    TopLeft,
    TopRight,
}

impl Joystick {
    pub fn new(center_x: u16, center_y: u16) -> Self {
        Self {
            center_x,
            center_y,
            last_state: JoystickState::Center,
        }
    }

    pub fn state_from(&mut self, x: u16, y: u16) -> Option<JoystickState> {
        let state = self.calculate_state(x, y);

        if state != self.last_state {
            self.last_state = state;
            Some(state)
        } else {
            None
        }
    }

    fn calculate_state(&self, x: u16, y: u16) -> JoystickState {
        let is_x_positive = x > self.center_x;
        let is_y_positive = y > self.center_y;

        let dx = x.abs_diff(self.center_x);
        let dy = y.abs_diff(self.center_y);

        if self.is_in_deadzone(dx, dy) {
            return JoystickState::Center;
        }

        match (is_x_positive, is_y_positive, dx.cmp(&dy)) {
            (true, true, Ordering::Less) => JoystickState::TopRight,
            (false, true, Ordering::Less) => JoystickState::TopLeft,

            (true, true, Ordering::Greater) | (true, false, Ordering::Greater) => {
                JoystickState::Right
            }
            (false, true, Ordering::Greater) | (false, false, Ordering::Greater) => {
                JoystickState::Left
            }
            (true, false, Ordering::Less) | (false, false, Ordering::Less) => JoystickState::Down,
            _ => JoystickState::Center,
        }
    }

    // Calculate the euclidean distance between the center and the current position
    fn is_in_deadzone(&self, dx: u16, dy: u16) -> bool {
        u32::from(dx).pow(2) + u32::from(dy).pow(2) <= JOYSTICK_DEADZONE.pow(2)
    }
}

pub struct Button<PIN: gpio::PinId> {
    last_interrupt: hal::timer::Instant,
    pin: gpio::Pin<PIN, gpio::FunctionSioInput, gpio::PullUp>,
}

impl<PIN: gpio::PinId> Button<PIN> {
    pub fn new(pin: gpio::Pin<PIN, gpio::FunctionSioInput, gpio::PullUp>) -> Self {
        pin.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true);

        Self {
            last_interrupt: hal::timer::Instant::from_ticks(0),
            pin,
        }
    }

    pub fn interrupted(&mut self, current_time: hal::timer::Instant) -> bool {
        let result = self.pin.interrupt_status(gpio::Interrupt::EdgeLow);

        if !result {
            return false;
        }

        self.pin.clear_interrupt(gpio::Interrupt::EdgeLow);

        // Debouncing
        if let Some(duration) = current_time.checked_duration_since(self.last_interrupt) {
            if duration.to_millis() <= DELAY_BETWEEN_INTERRUPTS {
                return false;
            }
        }

        self.last_interrupt = current_time;
        result
    }
}
