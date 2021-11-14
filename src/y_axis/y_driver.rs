use super::pwm::{MotorPwm, MotorPwmY, PwmPinY};
use crate::opto_encoder::Encoder;
use crate::opto_encoder::EncoderY;

use crate::motion_controller_2::PathSlope;

use cortex_m_semihosting::hprintln;

pub struct YDriver {
    motor: MotorPwmY,
    pub opto: EncoderY,
}

impl YDriver {
    pub fn new(motor_pwm: MotorPwm<PwmPinY>, opto: EncoderY) -> YDriver {
        let motor = MotorPwmY(motor_pwm);

        YDriver { motor, opto }
    }

    #[inline]
    pub fn move_towards(&mut self, pos: i32, slope: PathSlope) -> bool {
        const MIN_SPEED: f32 = 14.0; //14
        const MAX_SPEED: f32 = 80.0; //80
        const SPEED_DIFF: f32 = MAX_SPEED - MIN_SPEED;

        let speed = match slope {
            PathSlope::NoSlope => MIN_SPEED,
            PathSlope::Slope(slope) => {
                let mut slope = slope;

                // hprintln!("slope: {}", slope);

                if slope > 1.0 || slope < 0.0 {
                    hprintln!("[y_driver.rs] slope > 1.0 || < 0.0");
                }

                if slope > 1.0 {
                    slope = 1.0
                }
                if slope < 0.0 {
                    slope = 0.0
                }

                MIN_SPEED + (slope * SPEED_DIFF)
            }
        };

        // hprintln!("[y_driver.rs] speed: {}", speed);

        if pos > self.opto.pos() {
            self.motor.move_up(speed);
            false
        } else if pos < self.opto.pos() {
            self.motor.move_down(speed);
            false
        } else {
            true
        }
    }

    #[inline]
    pub fn stop(&mut self) {
        self.motor.0.active_stop();
    }
}
