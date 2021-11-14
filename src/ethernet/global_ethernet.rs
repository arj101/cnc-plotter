use core::{borrow::Borrow, cell::RefCell};
use cortex_m::interrupt::Mutex;

use stm32h7xx_hal::{
    device::TIM17,
    gpio::{
        self,
        gpioa::{PA1, PA2, PA7},
        gpiob::{PB13, PB14},
        gpioc::{PC1, PC4, PC5},
        gpioe::PE1,
        gpiog::{PG11, PG13},
        Alternate, Output, PushPull, AF11,
    },
    rcc::{rec::Eth1Mac, CoreClocks},
    timer::Timer,
};

use crate::buf_writer::BufWriter;

use super::ethernet_wrapper::EthernetWrapper;

pub static GLOBAL_ETHERNET: Mutex<RefCell<Option<EthernetWrapper>>> =
    Mutex::new(RefCell::new(None));
static mut FORMATTER_BUFF: [u8; 512] = [0u8; 512];
pub static FORMATTER: Mutex<RefCell<Option<BufWriter>>> = Mutex::new(RefCell::new(None));

pub fn init(
    link_led_low: PB14<Output<PushPull>>,
    link_led_high: PE1<Output<PushPull>>,
    ref_clk: PA1<Alternate<AF11>>,
    md_io: PA2<Alternate<AF11>>,
    md_clk: PC1<Alternate<AF11>>,
    crs: PA7<Alternate<AF11>>,
    rx_d0: PC4<Alternate<AF11>>,
    rx_d1: PC5<Alternate<AF11>>,
    tx_en: PG11<Alternate<AF11>>,
    tx_d0: PG13<Alternate<AF11>>,
    tx_d1: PB13<Alternate<AF11>>,
    timeout_timer: Timer<TIM17>,
    core_clocks: &CoreClocks,
    eth1_mac: Eth1Mac,
) {
    cortex_m::interrupt::free(|cs| {
        let mut eth = EthernetWrapper::new(link_led_low, link_led_high);
        eth.init(
            ref_clk,
            md_io,
            md_clk,
            crs,
            rx_d0,
            rx_d1,
            tx_en,
            tx_d0,
            tx_d1,
            timeout_timer,
            core_clocks,
            eth1_mac,
        );
        GLOBAL_ETHERNET.borrow(cs).replace(Some(eth));
        FORMATTER
            .borrow(cs)
            .replace(Some(BufWriter::new(unsafe { &mut FORMATTER_BUFF })));
    });
}

pub fn poll() -> i64 {
    cortex_m::interrupt::free(|cs| {
        let mut eth = GLOBAL_ETHERNET.borrow(cs).borrow_mut();
        let mut eth = eth.as_mut().unwrap();
        eth.poll()
    })
}

pub fn recv<'a>(buf: &'a mut [u8]) -> Option<&'a [u8]> {
    cortex_m::interrupt::free(move |cs| {
        let mut eth = GLOBAL_ETHERNET.borrow(cs).borrow_mut();
        let eth = eth.as_mut().unwrap();
        eth.recv(buf)
    })
}

/// # Panics
/// Panics if called before calling `init`
#[macro_export]
macro_rules! eth_send {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        use crate::ethernet::global_ethernet::GLOBAL_ETHERNET;
        use crate::ethernet::global_ethernet::FORMATTER;
        cortex_m::interrupt::free(|cs| {
            let mut formatter = FORMATTER.borrow(cs).borrow_mut();
            let mut formatter = formatter.as_mut().unwrap();
            let _ = write!(formatter, $($arg)*);
            GLOBAL_ETHERNET.borrow(cs).borrow_mut().as_mut().unwrap().send(&mut formatter)
        })
    }};
}

pub(crate) use eth_send;
