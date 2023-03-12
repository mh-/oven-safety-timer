#![no_std]
#![no_main]

//use arduino_hal::prelude::_void_ResultVoidExt;
use panic_halt as _;

const DURATION_SWITCH_OFF_MS: i32 = 2200;

const DURATION_PHASE_1_NORMAL_SECS: i32 = 8 * 60 / 1;
const DURATION_PHASE_2_REMINDER_SECS: i32 = 3 * 60 / 2;
const DURATION_PHASE_3_WARNING_SECS: i32 = 1 * 60 / 2;

const START_PHASE_1_NORMAL_SECS: i32 = 0;
const END_PHASE_1_NORMAL_SECS: i32 = START_PHASE_1_NORMAL_SECS+DURATION_PHASE_1_NORMAL_SECS-1;
const START_PHASE_2_REMINDER_SECS: i32 = END_PHASE_1_NORMAL_SECS+1;
const END_PHASE_2_REMINDER_SECS: i32 = START_PHASE_2_REMINDER_SECS+DURATION_PHASE_2_REMINDER_SECS-1;
const START_PHASE_3_WARNING_SECS: i32 = END_PHASE_2_REMINDER_SECS+1;
const END_PHASE_3_WARNING_SECS: i32 = START_PHASE_3_WARNING_SECS+DURATION_PHASE_3_WARNING_SECS-1;

pub mod led_blinking_pattern {
    #[derive(Default)]
    pub struct LedBlinkingPattern {
        on_ms: i32,
        off_ms: i32,
        pattern_start_ms: i32,
    }
    impl LedBlinkingPattern {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn set(&mut self, on_ms: i32, off_ms: i32) {
            self.on_ms = on_ms;
            self.off_ms = off_ms;
        }

        pub fn get_led_state(&mut self, current_time_ms: i32) -> bool {
            let time_in_pattern_ms = current_time_ms - self.pattern_start_ms;
            if (time_in_pattern_ms > self.on_ms + self.off_ms) || (time_in_pattern_ms < 0) {
                self.pattern_start_ms = current_time_ms;
                true
            } else if time_in_pattern_ms <= self.on_ms {
                true
            } else {
                false
            }
        }
    }
}

#[derive(PartialEq)]
enum State {
    OvenOff, 
    OvenOn, 
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    //let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    use rust_arduino_millis::arduino_millis::*;
    millis_init(dp.TC0);

    let mut built_in_led = pins.d13.into_output();
    built_in_led.set_low();

    let mut output_led = pins.d3.into_output();
    output_led.set_high();

    let mut output_relay = pins.d5.into_output();
    output_relay.set_high();

    let in_from_pushbutton = pins.d7.into_pull_up_input();

    let mut state = State::OvenOn;
    let mut led_blinking_pattern = led_blinking_pattern::LedBlinkingPattern::new();

    loop {

        // When OvenOn, check for button press:
        if state == State::OvenOn && in_from_pushbutton.is_high() {
            // button is pressed (button is NC, so now it is open and the pullup pulls the input high)
            let mut button_pressed_time_ms = 0;
            while in_from_pushbutton.is_high() {
                button_pressed_time_ms += 1;
                if button_pressed_time_ms > DURATION_SWITCH_OFF_MS {
                    // User wants to switch the oven off
                    output_relay.set_low();
                    state = State::OvenOff;
                    break;   
                }
                arduino_hal::delay_ms(1);
            }
            millis_reset();
        }

        // Read the timer:
        let time_ms: i32 = millis().try_into().unwrap();
        let mut time_secs: i32 = time_ms / 1000;

        // Fallback for timer overflow
        if state != State::OvenOn {
            time_secs = END_PHASE_3_WARNING_SECS + 1;  // off phase
        }

        // Decide LED Blinking Pattern based on time, and set OvenOff state when time ran out:
        match time_secs {
            START_PHASE_1_NORMAL_SECS..=END_PHASE_1_NORMAL_SECS => {
                // Phase 1:
                led_blinking_pattern.set(1000, 0);
            },
            START_PHASE_2_REMINDER_SECS..=END_PHASE_2_REMINDER_SECS => {
                // Phase 2:
                led_blinking_pattern.set(800, 800);
            },
            START_PHASE_3_WARNING_SECS..=END_PHASE_3_WARNING_SECS => {
                // Phase 3:
                led_blinking_pattern.set(150, 150);
            },
            _ => {
                // Off:
                state = State::OvenOff;
                led_blinking_pattern.set(30, 3000);    
            },
        }

        // When time ran out, switch the oven off:
        if state != State::OvenOn {
            output_relay.set_low();
        }

        // Set the LEDs based on the blinking pattern:
        if led_blinking_pattern.get_led_state(time_ms) {
            output_led.set_high();
            built_in_led.set_high();
        } else {
            output_led.set_low();
            built_in_led.set_low();
        }

        // Short delay:
        arduino_hal::delay_ms(1);  // must not be longer
    }


}