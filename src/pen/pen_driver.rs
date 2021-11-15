use super::servo_pwm::ServoPwm;

use crate::ethernet::global_ethernet::eth_send;

use core::cmp::PartialEq;
use core::marker::Copy;
use embedded_timeout_macros::embedded_hal::digital::v2::OutputPin;
use stm32h7::stm32h743v::lptim1::isr::DOWN_A;
use stm32h7::stm32h743v::I2C1;
use stm32h7xx_hal::gpio::{self, Alternate, Output, PushPull, AF4};
use stm32h7xx_hal::i2c::I2c;
use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::pwm::{self, Pwm};
use stm32h7xx_hal::rcc::rec::{I2c1, Tim3};
use stm32h7xx_hal::rcc::{CoreClocks, PeripheralREC};

const PEN_DRIVER_ADDR: u8 = 0x8;
pub const UP_ANGLE: u8 = 60;

#[derive(Copy, Clone, PartialEq)]
pub enum PenPosition {
    Default,
    Angle(u8),
}

// #[repr(transparent)]
pub struct PenDriver {
    i2c: I2c<I2C1>,
    pos: PenPosition,
}

impl PenDriver {
    pub fn new(
        i2c: I2C1,
        scl: gpio::gpiob::PB8<Alternate<AF4>>,
        sda: gpio::gpiob::PB9<Alternate<AF4>>,
        prec: I2c1,
        clocks: &CoreClocks,
    ) -> Self {
        Self {
            i2c: i2c.i2c((scl, sda), 100.khz(), prec, clocks),
            pos: PenPosition::Default,
        }
    }

    #[inline]
    pub fn move_up(&mut self) {
        self.set_angle(UP_ANGLE);
    }

    #[inline]
    pub fn move_pen(&mut self, pen_pos: PenPosition) {
        match pen_pos {
            PenPosition::Default => {
                self.move_up();
                self.pos = PenPosition::Default;
            }
            PenPosition::Angle(a) => self.set_angle(a),
        }
    }

    #[inline]
    pub fn set_angle(&mut self, angle: u8) {
        self.write_pos(angle);
        self.pos = PenPosition::Angle(angle);
    }

    #[inline]
    fn write_pos(&mut self, angle: u8) {
        if let Err(err) = self.i2c.write(PEN_DRIVER_ADDR, &[angle]) {
            eth_send!("[pen_driver] i2c error: {:?}\n", err);
        }
    }
}
