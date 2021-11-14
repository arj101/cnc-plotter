use crate::pwm::PWMState;
use crate::pwm_duty::PwmDutyCycle;

use stm32h7::stm32h743v::TIM3;
use stm32h7xx_hal::gpio::{self, Alternate};
use stm32h7xx_hal::pwm::{
    self, ActiveHigh, ComplementaryImpossible, FaultDisabled, Pwm, PwmControl,
};
use stm32h7xx_hal::rcc::rec::Tim3;
use stm32h7xx_hal::rcc::CoreClocks;

use stm32h7xx_hal::pac;
use stm32h7xx_hal::prelude::*;

use cortex_m_semihosting::hprintln;

pub struct ServoPwm {
    pin: Pwm<TIM3, pwm::C3, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
    angle: f32,
    duty_cycle: PwmDutyCycle,
    pwm_state: PWMState,
}

impl ServoPwm {
    pub fn new(
        servo_pin: gpio::gpioc::PC8<Alternate<gpio::AF2>>,
        tim3: TIM3,
        prec: Tim3,
        clocks: &CoreClocks,
        start_angle: f32,
    ) -> Self {
        let (_, mut pin) = tim3
            .pwm_advanced(servo_pin, prec, clocks)
            .frequency(50.hz())
            .left_aligned()
            .finalize();

        let mut duty_cycle = PwmDutyCycle::new(pin.get_max_duty());

        duty_cycle.set_duty_cycle(Self::angle_to_duty_cycle(start_angle));
        pin.set_duty(duty_cycle.get_duty_cycle_val());

        Self {
            pin,
            duty_cycle,
            angle: start_angle,
            pwm_state: PWMState::Disabled,
        }
    }

    pub fn enable(&mut self) {
        match self.pwm_state {
            PWMState::Disabled => {
                self.pin.enable();
                self.pwm_state = PWMState::Enabled;
            }
            PWMState::Enabled => (),
        }
    }

    pub fn disable(&mut self) {
        match self.pwm_state {
            PWMState::Enabled => {
                self.pin.disable();
                self.pwm_state = PWMState::Disabled;
            }
            PWMState::Disabled => (),
        }
    }

    pub fn set_angle(&mut self, angle: f32) {
        if self.angle == angle {
            return;
        }

        self.angle = angle;
        self.duty_cycle
            .set_duty_cycle(Self::angle_to_duty_cycle(angle));
        // self.duty_cycle.set_duty_cycle(1.0);
        self.pin.set_duty(self.duty_cycle.get_duty_cycle_val());

        self.angle = angle;
    }

    fn angle_to_duty_cycle(angle: f32) -> f32 {
        Self::t_on_to_duty_cycle(Self::angle_to_t_on(angle))
    }

    /// returns pwm on-time in ms
    fn angle_to_t_on(angle: f32) -> f32 {
        //TODO: figure out how this works
        let offset_time = 1.0; //apparently this is the zero value, idk (1ms is the minimum as per datasheet)

        if angle >= 180.0 {
            return offset_time + 1.0;
        }
        if angle <= 0.0 {
            return offset_time;
        }

        offset_time + (angle / 180.0) // 1-2ms 0-180deg (as per datasheet)
    }

    /// t_on: pwm on time in ms
    fn t_on_to_duty_cycle(t_on: f32) -> f32 {
        let frequency = 50.0; //hz
        let time_period = 1.0 / frequency;

        let t_on = t_on / 1000.0;

        if t_on >= time_period {
            return time_period;
        }
        if t_on <= 0.0 {
            return 0.0;
        }

        (t_on / time_period) * 100.0
    }
}
