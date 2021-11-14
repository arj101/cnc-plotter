use core::fmt::{write, Arguments};

use crate::buf_writer::BufWriter;
use embedded_timeout_macros::embedded_hal::digital::v2::OutputPin;
use smoltcp::{
    socket::{SocketHandle, UdpSocket},
    wire::{IpEndpoint, Ipv4Address},
};
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

use super::ethernet;
use super::timer;

use cortex_m_semihosting::hprintln;

const MAC_LOCAL: [u8; 6] = [0x02, 0x00, 0x11, 0x22, 0x33, 0x44];
const IP_LOCAL: [u8; 4] = [192, 168, 20, 99];
const IP_REMOTE: [u8; 4] = [192, 168, 20, 114];
const IP_REMOTE_PORT: u16 = 34254;

pub struct EthernetWrapper {
    fmt_buf: [u8; 256],
    remote_ep: IpEndpoint,
    local_ep: IpEndpoint,
    socket_handle: Option<SocketHandle>,
    link_led_low: PB14<Output<PushPull>>,
    link_led_high: PE1<Output<PushPull>>,
}

impl EthernetWrapper {
    pub fn new(link_led_low: PB14<Output<PushPull>>, link_led_high: PE1<Output<PushPull>>) -> Self {
        let local_ep = IpEndpoint::new(Ipv4Address::from_bytes(&IP_LOCAL).into(), 1234);
        let remote_ep = IpEndpoint::new(Ipv4Address::from_bytes(&IP_REMOTE).into(), IP_REMOTE_PORT);

        let mut link_led_low = link_led_low;
        let mut link_led_high = link_led_high;

        link_led_low.set_high();
        link_led_high.set_low();

        Self {
            fmt_buf: [0u8; 256],
            remote_ep,
            local_ep,
            socket_handle: None,
            link_led_high,
            link_led_low,
        }
    }

    pub fn init(
        &mut self,
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
        let pins = ethernet::Pins {
            ref_clk,
            md_io,
            md_clk,
            crs,
            rx_d0,
            rx_d1,
            tx_d0,
            tx_d1,
            tx_en,
        };

        let timeout_timer = timer::CountDownTimer::new(timeout_timer);
        let _timeout_timer = match ethernet::Interface::start(
            pins,
            &MAC_LOCAL,
            &IP_LOCAL,
            eth1_mac,
            core_clocks,
            timeout_timer,
        ) {
            Ok(tim17) => tim17,
            Err(e) => {
                hprintln!("Failed to start ethernet interface: {:?}", e).unwrap();
                loop {}
            }
        };
        hprintln!("Waiting for link to come up").unwrap();
        ethernet::Interface::interrupt_free(
            |ethernet_interface| {
                while !ethernet_interface.poll_link() {}
            },
        );
        hprintln!("Link alive").unwrap();

        let socket_handle = ethernet::Interface::interrupt_free(|ethernet_interface| {
            let socket_handle = ethernet_interface.new_udp_socket();
            let mut socket = ethernet_interface
                .sockets
                .as_mut()
                .unwrap()
                .get::<UdpSocket>(socket_handle);
            match socket.bind(self.local_ep) {
                Ok(()) => return socket_handle,
                Err(e) => {
                    hprintln!("Failed to bind socket to endpoint: {:?}", self.local_ep).unwrap();
                    loop {}
                }
            }
        });
        self.socket_handle = Some(socket_handle)
    }

    #[inline]
    pub fn poll(&mut self) -> i64 {
        ethernet::Interface::interrupt_free(|ethernet_interface| {
            match ethernet_interface.poll() {
                Ok(result) => {} // packets were processed or emitted
                Err(smoltcp::Error::Exhausted) => (),
                Err(smoltcp::Error::Unrecognized) => (),
                Err(e) => hprintln!("ethernet::Interface.poll() -> {:?}", e).unwrap(),
            }

            if ethernet_interface.poll_link() {
                self.link_led_high.set_high();
                self.link_led_low.set_low();
            } else {
                self.link_led_high.set_low();
                self.link_led_low.set_high();
            }

            ethernet_interface.now()
        })
    }

    #[inline]
    /// returns `Err(())` when `self.socket_handle` is `None`
    pub fn send(&mut self, buf_writer: &mut BufWriter) -> Result<(), ()> {
        let socket_handle = if let Some(handle) = self.socket_handle {
            handle
        } else {
            return Err(());
        };

        let data = buf_writer.get_bytes();

        ethernet::Interface::interrupt_free(|ethernet_interface| {
            let mut socket = ethernet_interface
                .sockets
                .as_mut()
                .unwrap()
                .get::<UdpSocket>(socket_handle);
            match socket.send_slice(data, self.remote_ep) {
                Ok(()) => (),
                Err(smoltcp::Error::Exhausted) => (),
                Err(e) => hprintln!("UdpSocket::send error: {:?}", e).unwrap(),
            };
        });

        buf_writer.clear_buf();

        Ok(())
    }

    #[inline]
    /// returns `None` when `self.socket_handle` is `None` or nothing is read
    pub fn recv<'a>(&mut self, data: &'a mut [u8]) -> Option<&'a [u8]> {
        let socket_handle = if let Some(handle) = self.socket_handle {
            handle
        } else {
            return None;
        };

        let mut write_len = 0;

        ethernet::Interface::interrupt_free(|ethernet_interface| {
            let mut socket = ethernet_interface
                .sockets
                .as_mut()
                .unwrap()
                .get::<UdpSocket>(socket_handle);
            match socket.recv_slice(data) {
                Ok((len, _)) => write_len = len,
                Err(smoltcp::Error::Exhausted) => (),
                Err(e) => hprintln!("UdpSocket::recv error: {:?}", e).unwrap(),
            };
        });

        if write_len > 0 {
            Some(&data[..write_len])
        } else {
            None
        }
    }
}
