// use super::pwm_duty::PwmDutyCycle;
// use super::timestamp;
// use stm32h7::stm32h743v::TIM1;
// use stm32h7xx_hal::gpio::{self, Alternate, Output, PushPull};
// use stm32h7xx_hal::hal::digital::v2::OutputPin;
// use stm32h7xx_hal::pwm::{self, Pwm};
// use stm32h7xx_hal::rcc::rec::Tim1;
// use stm32h7xx_hal::rcc::CoreClocks;

// use stm32h7xx_hal::prelude::*;
// use stm32h7xx_hal::time::Hertz;

// type MotorA = gpio::gpioe::PE13<Alternate<gpio::AF1>>;
// type MotorB = gpio::gpioe::PE14<Alternate<gpio::AF1>>;
// type HBridgeEnable = gpio::gpiog::PG14<Output<PushPull>>;

// pub struct XMotorPwm {
//     pwm_pin_a: Pwm<TIM1, pwm::C3>,
//     pwm_pin_b: Pwm<TIM1, pwm::C4>,
//     h_bridge_enable: HBridgeEnable,

//     pwm_a: PwmDutyCycle,
//     pwm_b: PwmDutyCycle,

//     motor_dir: MotorDir,
// }

// enum MotorDir {
//     Right(f32),
//     Left(f32),
//     Stopped(PWMState),
// }

// enum PWMState {
//     Disabled,
//     Enabled,
// }

// impl XMotorPwm {
//     pub fn new<T: Into<Hertz> + Sized>(
//         motor_a: MotorA,
//         motor_b: MotorB,
//         h_bridge_enable: HBridgeEnable,
//         tim1: TIM1,
//         prec: Tim1,
//         clocks: &CoreClocks,
//         freq: T,
//     ) -> XMotorPwm {
//         let (mut pwm_pin_a, mut pwm_pin_b) = tim1.pwm((motor_a, motor_b), freq, prec, clocks);

//         let pwm_a_max = pwm_pin_a.get_max_duty();
//         let pwm_b_max = pwm_pin_b.get_max_duty();

//         let mut h_bridge_enable = h_bridge_enable;
//         h_bridge_enable.set_low();

//         XMotorPwm {
//             pwm_pin_a,
//             pwm_pin_b,
//             h_bridge_enable,

//             pwm_a: PwmDutyCycle::new(pwm_a_max),
//             pwm_b: PwmDutyCycle::new(pwm_b_max),

//             motor_dir: MotorDir::Stopped(PWMState::Disabled),
//         }
//     }

//     //enable & disable pwm (global)
//     #[inline]
//     pub fn enable_pwm(&mut self) {
//         match self.motor_dir {
//             MotorDir::Left(_) => {}
//             MotorDir::Right(_) => {}
//             MotorDir::Stopped(PWMState::Enabled) => {}

//             MotorDir::Stopped(PWMState::Disabled) => {
//                 Self::enable_pwm_a(&mut self.pwm_a, &mut self.pwm_pin_a);
//                 Self::enable_pwm_b(&mut self.pwm_b, &mut self.pwm_pin_b);
//                 self.h_bridge_enable.set_high();
//                 self.motor_dir = MotorDir::Stopped(PWMState::Enabled);
//             }
//         }
//     }

//     #[inline]
//     pub fn disable_pwm(&mut self) {
//         match self.motor_dir {
//             MotorDir::Stopped(PWMState::Disabled) => {}
//             _ => {
//                 Self::disable_pwm_a(&mut self.pwm_a, &mut self.pwm_pin_a);
//                 Self::disable_pwm_b(&mut self.pwm_b, &mut self.pwm_pin_b);
//                 self.h_bridge_enable.set_low();
//                 self.motor_dir = MotorDir::Stopped(PWMState::Disabled);
//             }
//         }
//     }

//     //set_pwm_*_duty
//     fn set_pwm_a_duty(pwm_a: &mut PwmDutyCycle, pwm_pin_a: &mut Pwm<TIM1, pwm::C3>, duty: f32) {
//         pwm_a.set_duty_cycle(duty);
//         pwm_pin_a.set_duty(pwm_a.get_duty_cycle_val());
//     }

//     fn set_pwm_b_duty(pwm_b: &mut PwmDutyCycle, pwm_pin_b: &mut Pwm<TIM1, pwm::C4>, duty: f32) {
//         pwm_b.set_duty_cycle(duty);
//         pwm_pin_b.set_duty(pwm_b.get_duty_cycle_val());
//     }

//     //get_pwm_*_duty

//     pub fn get_pwm_a_duty(&self) -> f32 {
//         self.pwm_a.get_duty_cycle()
//     }

//     pub fn get_pwm_b_duty(&self) -> f32 {
//         self.pwm_b.get_duty_cycle()
//     }

//     //disable_pwm_*
//     fn disable_pwm_a(pwm_a: &mut PwmDutyCycle, pwm_pin_a: &mut Pwm<TIM1, pwm::C3>) {
//         Self::set_pwm_a_duty(pwm_a, pwm_pin_a, 0.0);
//         pwm_pin_a.disable();
//     }

//     fn disable_pwm_b(pwm_b: &mut PwmDutyCycle, pwm_pin_b: &mut Pwm<TIM1, pwm::C4>) {
//         Self::set_pwm_b_duty(pwm_b, pwm_pin_b, 0.0);
//         pwm_pin_b.disable();
//     }

//     //enable_pwm_*
//     fn enable_pwm_a(pwm_a: &mut PwmDutyCycle, pwm_pin_a: &mut Pwm<TIM1, pwm::C3>) {
//         Self::set_pwm_a_duty(pwm_a, pwm_pin_a, 0.0); //set pwm to zero before starting
//         pwm_pin_a.enable();
//     }

//     fn enable_pwm_b(pwm_b: &mut PwmDutyCycle, pwm_pin_b: &mut Pwm<TIM1, pwm::C4>) {
//         Self::set_pwm_b_duty(pwm_b, pwm_pin_b, 0.0); //set pwm to zero before starting
//         pwm_pin_b.enable();
//     }

//     //move_[dir]
//     #[inline]
//     pub fn move_left(&mut self, duty_cycle: f32) {
//         match self.motor_dir {
//             MotorDir::Left(duty) if duty == duty_cycle => {}
//             _ => {
//                 Self::enable_pwm_a(&mut self.pwm_a, &mut self.pwm_pin_a);
//                 Self::set_pwm_a_duty(&mut self.pwm_a, &mut self.pwm_pin_a, duty_cycle);
//                 Self::disable_pwm_b(&mut self.pwm_b, &mut self.pwm_pin_b);
//                 self.motor_dir = MotorDir::Left(duty_cycle);
//             }
//         }
//     }
//     #[inline]
//     pub fn move_right(&mut self, duty_cycle: f32) {
//         match self.motor_dir {
//             MotorDir::Right(duty) if duty == duty_cycle => {}
//             _ => {
//                 Self::enable_pwm_b(&mut self.pwm_b, &mut self.pwm_pin_b);
//                 Self::set_pwm_b_duty(&mut self.pwm_b, &mut self.pwm_pin_b, duty_cycle);
//                 Self::disable_pwm_a(&mut self.pwm_a, &mut self.pwm_pin_a);
//                 self.motor_dir = MotorDir::Right(duty_cycle);
//             }
//         }
//     }
// }
