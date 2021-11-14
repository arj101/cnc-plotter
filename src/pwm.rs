use super::pwm_duty::PwmDutyCycle;
use super::timestamp;

use stm32h7::stm32h743v::{TIM1, TIM4};
use stm32h7xx_hal::gpio::{self, Alternate, Output, PushPull};
use stm32h7xx_hal::hal::digital::v2::OutputPin;
use stm32h7xx_hal::pwm::{self, ActiveHigh, ComplementaryDisabled, ComplementaryImpossible, Pwm};
use stm32h7xx_hal::rcc::rec::{Tim1, Tim4};
use stm32h7xx_hal::rcc::CoreClocks;

use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::time::Hertz;

/// `MotorPwm` wrapper for x axis
#[repr(transparent)]
pub struct MotorPwmX(pub MotorPwm<PwmPinX>);

impl MotorPwmX {
    #[inline]
    pub fn move_right(&mut self, duty_cycle: f32) {
        self.0.move_positive(duty_cycle)
    }

    #[inline]
    pub fn move_left(&mut self, duty_cycle: f32) {
        self.0.move_negative(duty_cycle)
    }
}

#[repr(transparent)]
/// `MotorPwm` wrapper for y axis
pub struct MotorPwmY(pub MotorPwm<PwmPinY>);

impl MotorPwmY {
    #[inline]
    pub fn move_up(&mut self, duty_cycle: f32) {
        self.0.move_positive(duty_cycle)
    }

    #[inline]
    pub fn move_down(&mut self, duty_cycle: f32) {
        self.0.move_negative(duty_cycle)
    }
}

pub struct MotorPwm<T: PwmPin> {
    pwm_pin: T,

    pwm_a: PwmDutyCycle,
    pwm_b: PwmDutyCycle,

    motor_dir: MotorDir,
}

enum MotorDir {
    Positive(f32),
    Negative(f32),
    Stopped(PWMState),
}
pub enum PWMState {
    Disabled,
    Enabled,
}

impl<T: PwmPin> MotorPwm<T> {
    pub fn new(pwm_pin: T) -> MotorPwm<T> {
        let pwm_a_max = pwm_pin.get_a_max_duty();
        let pwm_b_max = pwm_pin.get_b_max_duty();

        MotorPwm {
            pwm_pin,

            pwm_a: PwmDutyCycle::new(pwm_a_max),
            pwm_b: PwmDutyCycle::new(pwm_b_max),

            motor_dir: MotorDir::Stopped(PWMState::Disabled),
        }
    }

    //enable & disable pwm (global)
    #[inline]
    pub fn enable_pwm(&mut self) {
        match self.motor_dir {
            MotorDir::Negative(_) => {}
            MotorDir::Positive(_) => {}
            MotorDir::Stopped(PWMState::Enabled) => {}

            MotorDir::Stopped(PWMState::Disabled) => {
                Self::enable_pwm_a(&mut self.pwm_a, &mut self.pwm_pin);
                Self::enable_pwm_b(&mut self.pwm_b, &mut self.pwm_pin);
                self.pwm_pin.set_h_bridge_high();
                self.motor_dir = MotorDir::Stopped(PWMState::Enabled);
            }
        }
    }

    #[inline]
    pub fn disable_pwm(&mut self) {
        match self.motor_dir {
            MotorDir::Stopped(PWMState::Disabled) => {}
            _ => {
                Self::disable_pwm_a(&mut self.pwm_a, &mut self.pwm_pin);
                Self::disable_pwm_b(&mut self.pwm_b, &mut self.pwm_pin);
                self.pwm_pin.set_h_bridge_low();
                self.motor_dir = MotorDir::Stopped(PWMState::Disabled);
            }
        }
    }

    //set_pwm_*_duty
    fn set_pwm_a_duty(pwm_a: &mut PwmDutyCycle, pwm_pin: &mut T, duty: f32) {
        pwm_a.set_duty_cycle(duty);
        pwm_pin.set_a_duty(pwm_a.get_duty_cycle_val());
    }

    fn set_pwm_b_duty(pwm_b: &mut PwmDutyCycle, pwm_pin: &mut T, duty: f32) {
        pwm_b.set_duty_cycle(duty);
        pwm_pin.set_b_duty(pwm_b.get_duty_cycle_val());
    }

    //get_pwm_*_duty

    pub fn get_pwm_a_duty(&self) -> f32 {
        self.pwm_a.get_duty_cycle()
    }

    pub fn get_pwm_b_duty(&self) -> f32 {
        self.pwm_b.get_duty_cycle()
    }

    //disable_pwm_*
    fn disable_pwm_a(pwm_a: &mut PwmDutyCycle, pwm_pin: &mut T) {
        Self::set_pwm_a_duty(pwm_a, pwm_pin, 0.0);
        pwm_pin.disable_a();
    }

    fn disable_pwm_b(pwm_b: &mut PwmDutyCycle, pwm_pin: &mut T) {
        Self::set_pwm_b_duty(pwm_b, pwm_pin, 0.0);
        pwm_pin.disable_b();
    }

    //enable_pwm_*
    fn enable_pwm_a(pwm_a: &mut PwmDutyCycle, pwm_pin: &mut T) {
        Self::set_pwm_a_duty(pwm_a, pwm_pin, 0.0); //set pwm to zero before starting
        pwm_pin.enable_a();
    }

    fn enable_pwm_b(pwm_b: &mut PwmDutyCycle, pwm_pin: &mut T) {
        Self::set_pwm_b_duty(pwm_b, pwm_pin, 0.0); //set pwm to zero before starting
        pwm_pin.enable_b();
    }

    //move_[dir]
    #[inline]
    pub fn move_negative(&mut self, duty_cycle: f32) {
        match self.motor_dir {
            MotorDir::Negative(duty) if duty == duty_cycle => {}
            _ => {
                Self::enable_pwm_a(&mut self.pwm_a, &mut self.pwm_pin);
                Self::set_pwm_a_duty(&mut self.pwm_a, &mut self.pwm_pin, duty_cycle);
                Self::disable_pwm_b(&mut self.pwm_b, &mut self.pwm_pin);
                self.motor_dir = MotorDir::Negative(duty_cycle);
            }
        }
    }
    #[inline]
    pub fn move_positive(&mut self, duty_cycle: f32) {
        match self.motor_dir {
            MotorDir::Positive(duty) if duty == duty_cycle => {}
            _ => {
                Self::enable_pwm_b(&mut self.pwm_b, &mut self.pwm_pin);
                Self::set_pwm_b_duty(&mut self.pwm_b, &mut self.pwm_pin, duty_cycle);
                Self::disable_pwm_a(&mut self.pwm_a, &mut self.pwm_pin);
                self.motor_dir = MotorDir::Positive(duty_cycle);
            }
        }
    }

    #[inline]
    pub fn active_stop(&mut self) {
        match self.motor_dir {
            MotorDir::Stopped(PWMState::Enabled) => (),
            _ => {
                Self::set_pwm_a_duty(&mut self.pwm_a, &mut self.pwm_pin, 0.0);
                Self::set_pwm_b_duty(&mut self.pwm_b, &mut self.pwm_pin, 0.0);
                self.motor_dir = MotorDir::Stopped(PWMState::Enabled);
            }
        }
    }
}

pub trait PwmPin {
    fn enable_a(&mut self);
    fn enable_b(&mut self);

    fn disable_a(&mut self);
    fn disable_b(&mut self);

    fn set_a_duty(&mut self, duty: u16);
    fn set_b_duty(&mut self, duty: u16);

    fn get_a_max_duty(&self) -> u16;
    fn get_b_max_duty(&self) -> u16;

    fn set_h_bridge_high(&mut self);
    fn set_h_bridge_low(&mut self);
}

type MotorAX = gpio::gpioe::PE13<Alternate<gpio::AF1>>;
type MotorBX = gpio::gpioe::PE14<Alternate<gpio::AF1>>;
type HBridgeEnableX = gpio::gpiog::PG14<Output<PushPull>>;

type MotorAY = gpio::gpiob::PB6<Alternate<gpio::AF2>>;
type MotorBY = gpio::gpiob::PB7<Alternate<gpio::AF2>>;
type HBridgeEnableY = gpio::gpioe::PE8<Output<PushPull>>;

pub struct PwmPinX {
    pwm_pin_a: Pwm<TIM1, pwm::C3, ComplementaryDisabled, ActiveHigh, ActiveHigh>,
    pwm_pin_b: Pwm<TIM1, pwm::C4, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
    h_bridge_enable: HBridgeEnableX,
}

impl PwmPinX {
    pub fn new<T: Into<Hertz> + Sized>(
        motor_a: MotorAX,
        motor_b: MotorBX,
        h_bridge_enable: HBridgeEnableX,
        tim1: TIM1,
        prec: Tim1,
        clocks: &CoreClocks,
        freq: T,
    ) -> PwmPinX {
        let (mut pwm_pin_a, mut pwm_pin_b) = tim1.pwm((motor_a, motor_b), freq, prec, clocks);

        let pwm_a_max = pwm_pin_a.get_max_duty();
        let pwm_b_max = pwm_pin_b.get_max_duty();

        let mut h_bridge_enable = h_bridge_enable;
        h_bridge_enable.set_low();

        PwmPinX {
            pwm_pin_a,
            pwm_pin_b,
            h_bridge_enable,
        }
    }
}

impl PwmPin for PwmPinX {
    #[inline]
    fn enable_a(&mut self) {
        self.pwm_pin_a.enable();
    }

    #[inline]
    fn enable_b(&mut self) {
        self.pwm_pin_b.enable();
    }

    #[inline]
    fn disable_a(&mut self) {
        self.pwm_pin_a.disable();
    }

    #[inline]
    fn disable_b(&mut self) {
        self.pwm_pin_b.disable();
    }

    #[inline]
    fn set_a_duty(&mut self, duty: u16) {
        self.pwm_pin_a.set_duty(duty);
    }

    #[inline]
    fn set_b_duty(&mut self, duty: u16) {
        self.pwm_pin_b.set_duty(duty);
    }

    #[inline]
    fn get_a_max_duty(&self) -> u16 {
        self.pwm_pin_a.get_max_duty()
    }

    #[inline]
    fn get_b_max_duty(&self) -> u16 {
        self.pwm_pin_b.get_max_duty()
    }

    #[inline]
    fn set_h_bridge_high(&mut self) {
        self.h_bridge_enable.set_high();
    }

    #[inline]
    fn set_h_bridge_low(&mut self) {
        self.h_bridge_enable.set_low();
    }
}

pub struct PwmPinY {
    pwm_pin_a: Pwm<TIM4, pwm::C1, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
    pwm_pin_b: Pwm<TIM4, pwm::C2, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
    h_bridge_enable: HBridgeEnableY,
}

impl PwmPinY {
    pub fn new<T: Into<Hertz> + Sized>(
        motor_a: MotorAY,
        motor_b: MotorBY,
        h_bridge_enable: HBridgeEnableY,
        tim4: TIM4,
        prec: Tim4,
        clocks: &CoreClocks,
        freq: T,
    ) -> PwmPinY {
        let (mut pwm_pin_a, mut pwm_pin_b) = tim4.pwm((motor_a, motor_b), freq, prec, clocks);

        let pwm_a_max = pwm_pin_a.get_max_duty();
        let pwm_b_max = pwm_pin_b.get_max_duty();

        let mut h_bridge_enable = h_bridge_enable;
        h_bridge_enable.set_low();

        PwmPinY {
            pwm_pin_a,
            pwm_pin_b,
            h_bridge_enable,
        }
    }
}

impl PwmPin for PwmPinY {
    #[inline]
    fn enable_a(&mut self) {
        self.pwm_pin_a.enable();
    }

    #[inline]
    fn enable_b(&mut self) {
        self.pwm_pin_b.enable();
    }

    #[inline]
    fn disable_a(&mut self) {
        self.pwm_pin_a.disable();
    }

    #[inline]
    fn disable_b(&mut self) {
        self.pwm_pin_b.disable();
    }

    #[inline]
    fn set_a_duty(&mut self, duty: u16) {
        self.pwm_pin_a.set_duty(duty);
    }

    #[inline]
    fn set_b_duty(&mut self, duty: u16) {
        self.pwm_pin_b.set_duty(duty);
    }

    #[inline]
    fn get_a_max_duty(&self) -> u16 {
        self.pwm_pin_a.get_max_duty()
    }

    #[inline]
    fn get_b_max_duty(&self) -> u16 {
        self.pwm_pin_b.get_max_duty()
    }

    #[inline]
    fn set_h_bridge_high(&mut self) {
        self.h_bridge_enable.set_high();
    }

    #[inline]
    fn set_h_bridge_low(&mut self) {
        self.h_bridge_enable.set_low();
    }
}
