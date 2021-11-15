use heapless::consts::*;
use heapless::Vec;

use crate::ethernet::ethernet_wrapper::EthernetWrapper;
use crate::BufWriter;
use cortex_m_semihosting::hprintln;

use crate::ethernet::global_ethernet;
use global_ethernet::eth_send;

pub enum HandlerState {
    Busy,
    WaitingRecv,
    CanStart,
}

use HandlerState::*;

pub struct CommandHandler {
    recv_buffer: [u8; 576],
    pub gcode_buffer: Vec<gcode::GCode, U512>,
    state: HandlerState,
    waiting_recv_sent: bool,
    calibration_request: bool,
}

impl CommandHandler {
    pub fn new(start_state: HandlerState) -> Self {
        Self {
            recv_buffer: [0u8; 576],
            gcode_buffer: Vec::new(),
            state: start_state,
            waiting_recv_sent: false,
            calibration_request: false,
        }
    }

    #[inline]
    pub fn tick(&mut self) {
        // if let Busy = self.state { return }

        if !self.waiting_recv_sent {
            eth_send!("Waiting for input...\n\r");
            self.waiting_recv_sent = true;
        }

        let data = global_ethernet::recv(&mut self.recv_buffer);

        if let None = data {
            return;
        }
        let data = data.unwrap();

        if let Ok(code_str) = core::str::from_utf8(data) {
            // if code_str.len() == 0 { return }
            match code_str.trim() {
                ">end_write" => {
                    self.state = CanStart;
                }

                ">calibrate" => {
                    self.calibration_request = true;
                }

                _ => {
                    let codes = gcode::parse(&code_str);
                    for code in codes {
                        if let Err(_) = self.gcode_buffer.push(code) {
                            break;
                        }
                    }
                }
            }
        }
    }

    #[inline]
    pub fn set_status_busy(&mut self) {
        self.state = Busy;
    }

    // #[inline]
    // pub fn set_waiting_recv(&mut self) {
    //     self.state = WaitingRecv
    // }

    #[inline]
    pub fn can_start(&self) -> bool {
        if let CanStart = self.state {
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn needs_calibration(&mut self) -> bool {
        let calib_rqst = self.calibration_request;
        self.calibration_request = false;
        calib_rqst
    }

    #[inline]
    pub fn clear_gcode_buffer(&mut self) {
        self.gcode_buffer.clear();
        self.waiting_recv_sent = false;
    }
}
