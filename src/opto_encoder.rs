use crate::timestamp;
use stm32h7::stm32h743v::{TIM2, TIM8};
use stm32h7xx_hal::rcc::rec::ResetEnable;
use stm32h7xx_hal::rcc::rec::{Tim2, Tim8};

use stm32h7xx_hal::gpio::{gpioa::PA5, gpiob::PB3, gpioc::PC6, gpioc::PC7, Alternate, AF1, AF3};

pub struct EncoderY {
    tim2: TIM2,
    prec: Tim2,
    zero_value: u32,
}

impl EncoderY {
    pub fn new(tim2: TIM2, prec: Tim2, _pins: (PA5<Alternate<AF1>>, PB3<Alternate<AF1>>)) -> Self {
        let prec = prec.enable().reset();
        let zero_value = u32::MAX / 4;
        tim2.psc.write(|w| w.psc().bits(0u16));
        tim2.cnt.write(|w| w.cnt().bits(zero_value));
        tim2.ccmr1_input().write(|w| w.cc1s().ti1().cc2s().ti2());
        tim2.ccer.write(|w| {
            w.cc1p()
                .bit(true)
                .cc2p()
                .bit(false)
                .cc1np()
                .bit(false)
                .cc2np()
                .bit(false)
        });
        tim2.smcr.write(|w| w.sms().encoder_mode_3());
        tim2.cr1.write(|w| w.cen().enabled());
        Self {
            tim2,
            prec,
            zero_value,
        }
    }

    pub fn calibrate(&self) {
        self.tim2.cnt.write(|w| w.cnt().bits(self.zero_value));
    }
}

pub struct EncoderX {
    tim8: TIM8,
    prec: Tim8,
    zero_value: u16,
}

impl EncoderX {
    pub fn new(tim8: TIM8, prec: Tim8, _pins: (PC6<Alternate<AF3>>, PC7<Alternate<AF3>>)) -> Self {
        let prec = prec.enable().reset();
        let zero_value = u16::MAX / 4;
        tim8.psc.write(|w| w.psc().bits(0u16));
        tim8.cnt.write(|w| w.cnt().bits(zero_value));
        tim8.ccmr1_input().write(|w| w.cc1s().ti1().cc2s().ti2());
        tim8.ccer.write(|w| {
            w.cc1p()
                .bit(true)
                .cc2p()
                .bit(false)
                .cc1np()
                .bit(false)
                .cc2np()
                .bit(false)
        });
        tim8.smcr.write(|w| w.sms().encoder_mode_3());
        tim8.cr1.write(|w| w.cen().enabled());
        Self {
            tim8,
            prec,
            zero_value,
        }
    }

    pub fn pos(&self) -> i32 {
        let real_count = self.tim8.cnt.read().cnt().bits();
        real_count as i32 - self.zero_value as i32
    }

    pub fn dir(&self) -> bool {
        self.tim8.cr1.read().dir().bit()
    }

    pub fn calibrate(&self) {
        self.tim8.cnt.write(|w| w.cnt().bits(self.zero_value));
    }
}

pub trait Encoder {
    fn pos(&self) -> i32;
    fn dir(&self) -> bool;
    fn calibrate(&self);
}

impl Encoder for EncoderX {
    fn pos(&self) -> i32 {
        let real_count = self.tim8.cnt.read().cnt().bits();
        real_count as i32 - self.zero_value as i32
    }

    fn dir(&self) -> bool {
        self.tim8.cr1.read().dir().bit()
    }

    fn calibrate(&self) {
        self.tim8.cnt.write(|w| w.cnt().bits(self.zero_value));
    }
}

impl Encoder for EncoderY {
    fn pos(&self) -> i32 {
        let real_count = self.tim2.cnt.read().cnt().bits();
        real_count as i32 - self.zero_value as i32
    }

    fn dir(&self) -> bool {
        self.tim2.cr1.read().dir().bit()
    }

    fn calibrate(&self) {
        self.tim2.cnt.write(|w| w.cnt().bits(self.zero_value));
    }
}
