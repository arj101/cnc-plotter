use super::timestamp;
use core::fmt::Debug;
use stm32h7xx_hal::gpio;
use stm32h7xx_hal::gpio::{Edge, ExtiPin, Floating, Input, Output, PullUp, PushPull};
use stm32h7xx_hal::hal::digital::v2::InputPin;

#[derive(Debug, Clone, Copy)]
pub enum DecodedDir {
    Positive,
    Negative,
    Stationary,
}

type Opto1A = gpio::gpiob::PB9<Input<Floating>>;
type Opto1B = gpio::gpioe::PE11<Input<Floating>>;

type Opto2A = gpio::gpiof::PF3<Input<Floating>>;
type Opto2B = gpio::gpiog::PG12<Input<Floating>>;

pub struct OptoDecoder<T: OptoGpio + Sized> {
    pub pins: T,
    pos: i32,
    pass_by_duration: u64,
    last_pass_by: u64,
    dir: DecodedDir,
}

impl<T: OptoGpio + Sized> OptoDecoder<T> {
    pub fn new(opto: T) -> OptoDecoder<T> {
        OptoDecoder {
            pins: opto,
            pos: 0i32,
            pass_by_duration: 0u64,
            last_pass_by: 0u64,
            dir: DecodedDir::Stationary,
        }
    }

    #[inline]
    pub fn pos(&self) -> i32 {
        self.pos
    }

    #[inline]
    pub fn pass_by_duration(&self) -> u64 {
        self.pass_by_duration
    }

    pub fn dir(&self) -> DecodedDir {
        self.dir
    }

    pub fn calibrate_pos(&mut self, base: i32) {
        self.pos = base;
    }

    /// this function is supposed to be run repeatedly
    pub fn tick(&mut self) {
        if timestamp() - self.last_pass_by > 50_000 {
            self.pass_by_duration = 0;
            self.dir = DecodedDir::Stationary;
        }
    }

    #[inline]
    pub fn isr(&mut self) {
        //Interrupt Service Routine, keep it as short as possible
        if self.pins.opto_a_state() != self.pins.opto_b_state() {
            self.pos -= 1;
            self.dir = DecodedDir::Negative;
        } else {
            self.pos += 1;
            self.dir = DecodedDir::Positive;
        }

        // self.pass_by_duration = timestamp() - self.last_pass_by;
        // self.last_pass_by = timestamp();
    }
}

pub struct Opto1Gpio {
    opto_a: Opto1A,
    opto_b: Opto1B,
}

impl Opto1Gpio {
    pub fn new(opto_a: Opto1A, opto_b: Opto1B) -> Opto1Gpio {
        Opto1Gpio { opto_a, opto_b }
    }

    /// clear interrupt pending bit pe9
    #[inline]
    pub fn clear_interrupt_pending_bit_pe9(&mut self) {
        self.opto_a.clear_interrupt_pending_bit();
    }
}

pub struct Opto2Gpio {
    opto_a: Opto2A,
    opto_b: Opto2B,
}

impl Opto2Gpio {
    pub fn new(opto_a: Opto2A, opto_b: Opto2B) -> Opto2Gpio {
        Opto2Gpio { opto_a, opto_b }
    }

    /// clear interrupt pending bit pf3
    #[inline]
    pub fn clear_interrupt_pending_bit_pf3(&mut self) {
        self.opto_a.clear_interrupt_pending_bit();
    }
}

pub trait OptoGpio {
    fn opto_a_state(&self) -> bool;
    fn opto_b_state(&self) -> bool;
}

impl OptoGpio for Opto1Gpio {
    #[inline]
    fn opto_a_state(&self) -> bool {
        self.opto_a.is_high().unwrap()
    }

    #[inline]
    fn opto_b_state(&self) -> bool {
        self.opto_b.is_high().unwrap()
    }
}
impl OptoGpio for Opto2Gpio {
    #[inline]
    fn opto_a_state(&self) -> bool {
        self.opto_a.is_high().unwrap()
    }

    #[inline]
    fn opto_b_state(&self) -> bool {
        self.opto_b.is_high().unwrap()
    }
}
