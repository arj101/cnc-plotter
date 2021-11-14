#![no_std]
#![no_main]

mod buf_writer;
mod com;
mod motion_controller_2;
mod motion_controller_advanced;
pub mod speed_calc;
pub mod stop_timer;
pub mod ethernet;
mod command_handler;
pub mod interpolator;
pub mod opto;
pub mod opto_encoder;
pub mod pen;
pub mod pwm;
pub mod pwm_duty;
pub mod sequence;
mod sequence_data;
pub mod sequence_wrapper;
mod usb_com;
pub mod x_axis;
pub mod y_axis;

use buf_writer::BufWriter;
use ethernet::ethernet_wrapper::EthernetWrapper;
use opto::{Opto1Gpio, OptoDecoder};
use pwm::{MotorPwmX, MotorPwmY};
use x_axis::opto::Opto2Gpio;
use x_axis::x_driver::XDriver;
use y_axis::y_driver::YDriver;

use pwm::{MotorPwm, PwmPinX};
// use x_axis::x_pwm::XMotorPwm;

use pen::pen_driver::PenDriver;
use pen::PenPosition::*;

use interpolator::CircularInterpolationDir::*;
use interpolator::Interpolation;
use sequence_wrapper::SequenceWrapper;
// use motion_controller::MotionController;
use motion_controller_advanced::MotionController;

// use command_handler::CommandHandler;
use com::CommandHandler;
use command_handler::HandlerState;
use ethernet::global_ethernet;

// pick a panicking behavior
use panic_semihosting as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                            // use panic_abort as _; // requires nightly
                            // use panic_itm as _; // logs messages over ITM; requires ITM support
                            // use panic_semihosting as _; // logs messages to the host stderr; requires a debugge
use micromath::{F32Ext, F32};

use cortex_m_rt::entry;
use cortex_m_semihosting::{hprint, hprintln};

use core::borrow::{Borrow, BorrowMut};
use core::cell::Ref;
use core::cell::{Cell, RefCell};
use core::fmt::write;
use core::sync::atomic::{AtomicU32, Ordering};

use cortex_m::interrupt::{free, Mutex};
use cortex_m::peripheral::NVIC;
use stm32h7xx_hal::gpio::Speed::*;
use stm32h7xx_hal::gpio::{Edge, ExtiPin, Floating, Input, Output, PullUp, PushPull};
use stm32h7xx_hal::hal::digital::v2::OutputPin;
use stm32h7xx_hal::hal::digital::v2::ToggleableOutputPin;
use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::rcc::rec::ResetEnable;
use stm32h7xx_hal::{interrupt, pac, prelude::*, timer};
// LED pin
use stm32h7xx_hal::gpio::gpiob::PB0;

// Button pins
use stm32h7xx_hal::gpio::gpioc::PC13;

use crate::pwm::PwmPinY;

use opto_encoder::*;

static mut TICK_TIMER: Option<timer::Timer<pac::TIM5>> = None;
static OVERFLOWS: AtomicU32 = AtomicU32::new(0);

// static DRIVER: Mutex<RefCell<Option<(XDriver, YDriver)>>> = Mutex::new(RefCell::new(None));

// static SYNCHRONIZER: Mutex<RefCell<Option<MotionController>>> = Mutex::new(RefCell::new(None));
static mut SYNCHRONIZER: Option<MotionController> = None;

#[entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let mut dp = pac::Peripherals::take().unwrap();

    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    let rcc = dp.RCC.constrain();
    let mut ccdr = rcc
        .sys_ck(400.mhz())
        .hclk(200.mhz())
        .pll1_r_ck(100.mhz())
        .freeze(pwrcfg, &dp.SYSCFG);

    cp.SCB.invalidate_icache();
    cp.SCB.enable_icache();
    cp.DWT.enable_cycle_counter();

    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioa = dp.GPIOA.split(ccdr.peripheral.GPIOA);
    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);
    let gpiog = dp.GPIOG.split(ccdr.peripheral.GPIOG);
    let gpiof = dp.GPIOF.split(ccdr.peripheral.GPIOF);
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);
    let gpiod = dp.GPIOD.split(ccdr.peripheral.GPIOD);

    let timeout_timer = dp
        .TIM17
        .timer(100.hz(), ccdr.peripheral.TIM17, &ccdr.clocks);

    let mut link_led_low = gpiob.pb14.into_push_pull_output();
    let mut link_led_high = gpioe.pe1.into_push_pull_output();

    // let mut eth_wrapper = EthernetWrapper::new(link_led_low, link_led_high);

    let mut syscfg = dp.SYSCFG;
    let mut exti = dp.EXTI;

    let mut timer = dp
        .TIM5
        .tick_timer(1.mhz(), ccdr.peripheral.TIM5, &ccdr.clocks);
    timer.listen(timer::Event::TimeOut);

    free(|cs| unsafe { TICK_TIMER = Some(timer) });

    // let tim1 = ccdr.peripheral.TIM1.enable().reset();
    // dp.TIM1.smcr.write(|w| w.sms().encoder_mode_3());
    // let counter_val = dp.TIM1.cnt.read().cnt().bits();

    let mut motor_pwm_y = MotorPwm::new(PwmPinY::new(
        gpiob.pb6.into_alternate_af2(),
        gpiob.pb7.into_alternate_af2(),
        gpioe.pe8.into_push_pull_output(),
        dp.TIM4,
        ccdr.peripheral.TIM4,
        &ccdr.clocks,
        60.hz(), //45Hz
    ));

    motor_pwm_y.enable_pwm();

    let encoder_y = EncoderY::new(
        dp.TIM2,
        ccdr.peripheral.TIM2,
        (
            gpioa.pa5.into_alternate_af1(),
            gpiob.pb3.into_alternate_af1(),
        ),
    );
    let encoder_x = EncoderX::new(
        dp.TIM8,
        ccdr.peripheral.TIM8,
        (
            gpioc.pc6.into_alternate_af3(),
            gpioc.pc7.into_alternate_af3(),
        ),
    );

    let mut delay = cp.SYST.delay(ccdr.clocks);

    let motor_pwm_x = MotorPwm::new(PwmPinX::new(
        gpioe.pe13.into_alternate_af1(),
        gpioe.pe14.into_alternate_af1(),
        gpiog.pg14.into_push_pull_output(),
        dp.TIM1,
        ccdr.peripheral.TIM1,
        &ccdr.clocks,
        200.hz(), //45Hz
    ));


    let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
    let sda = gpiob.pb9.into_alternate_af4().set_open_drain();
    let mut pen_driver = PenDriver::new(
        dp.I2C1,
        scl,
        sda,
        ccdr.peripheral.I2C1,
        &ccdr.clocks,
    );

    let synchronizer = MotionController::new(
        MotorPwmX(motor_pwm_x),
        MotorPwmY(motor_pwm_y),
        encoder_x,
        encoder_y,
        pen_driver,
    );

    free(|_cs| {
        // SYNCHRONIZER.borrow(cs).replace(Some(synchronizer));
        unsafe {
            SYNCHRONIZER = Some(synchronizer);
        }

        // let mut synchronizer = SYNCHRONIZER.borrow(cs).borrow_mut();
        // let mut synchronizer = synchronizer.as_mut().unwrap();
        // synchronizer.calibrate(&mut delay);

        // let mm_per_unit = 0.0211;

        // synchronizer.start_sequence();
    });

    let synchronizer = unsafe { &mut SYNCHRONIZER };
    let synchronizer = synchronizer.as_mut().unwrap();
    synchronizer.calibrate(&mut delay);

    global_ethernet::init(
        link_led_low,
        link_led_high,
        gpioa.pa1.into_alternate_af11().set_speed(VeryHigh),
        gpioa.pa2.into_alternate_af11().set_speed(VeryHigh),
        gpioc.pc1.into_alternate_af11().set_speed(VeryHigh),
        gpioa.pa7.into_alternate_af11().set_speed(VeryHigh),
        gpioc.pc4.into_alternate_af11().set_speed(VeryHigh),
        gpioc.pc5.into_alternate_af11().set_speed(VeryHigh),
        gpiog.pg11.into_alternate_af11().set_speed(VeryHigh),
        gpiog.pg13.into_alternate_af11().set_speed(VeryHigh),
        gpiob.pb13.into_alternate_af11().set_speed(VeryHigh),
        timeout_timer,
        &ccdr.clocks,
        ccdr.peripheral.ETH1MAC,
    );

    synchronizer.start_sequence();

    unsafe {
        cp.NVIC.set_priority(interrupt::TIM2, 2);
        NVIC::unmask(interrupt::TIM2);
    }

    let mut fmt_buf = [0u8; 64];
    let mut buf_writer = BufWriter::new(&mut fmt_buf);

    // let mut cmd_handler = CommandHandler::new(HandlerState::Busy);
    let mut cmd_handler = CommandHandler::new();

    loop {
        // if let Err(e) = i2c.write(0x08, &[angle]) {
        //     eth_send!("i2c error: {:?}\n", e);
        // }
        // angle += 1;
        // angle %= 255;

        // free(|cs| {
        //     if let Some(sync) = SYNCHRONIZER.borrow(cs).borrow_mut().as_mut() {
        //         sync.tick(&mut cmd_handler);
        //         let (_pos_x, _pos_y) = sync.curr_pos();
        //         pos_x = _pos_x;
        //         pos_y = _pos_y;
        //     }
        // });

        synchronizer.tick(&mut cmd_handler);

        let _now = global_ethernet::poll();

        cmd_handler.tick();
    }
}

#[interrupt]
fn TIM2() {
    OVERFLOWS.fetch_add(1, Ordering::SeqCst);
    let mut rc = unsafe { &mut TICK_TIMER };
    let timer = rc.as_mut().unwrap();
    timer.clear_irq();
}

pub fn timestamp() -> u64 {
    let overflows = OVERFLOWS.load(Ordering::SeqCst) as u64;
    let mut rc = unsafe { &mut TICK_TIMER };
    let timer = rc.as_mut().unwrap();
    let ctr = timer.counter() as u64;

    (overflows << 32) + ctr
}

use core::fmt::Write;
use core::ptr;
use cortex_m_rt::{exception, ExceptionFrame};
use cortex_m_semihosting::hio;

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    if let Ok(mut hstdout) = hio::hstdout() {
        writeln!(hstdout, "{:#?}", ef).ok();
    }

    loop {}
}
