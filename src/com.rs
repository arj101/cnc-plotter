//command_handler.rs v2

use heapless::consts::*;
use heapless::Vec;

use crate::ethernet::ethernet_wrapper::EthernetWrapper;
use crate::BufWriter;
use cortex_m_semihosting::hprintln;

use crate::ethernet::global_ethernet;
use global_ethernet::eth_send;

pub struct CommandHandler {
    recv_buffer: [u8; 576],
    gcode_buffer: Vec<gcode::GCode, U512>,

    buffer_read_offset: usize,

    page_count: usize,

    buffer_empty_sent: bool,
    buffer_full_sent: bool,

    calibration_request: bool,
}

impl CommandHandler {
    pub fn new() -> Self {
        Self {
            recv_buffer: [0u8; 576],
            gcode_buffer: Vec::new(),

            page_count: 0,

            buffer_read_offset: 0,

            buffer_empty_sent: false,
            buffer_full_sent: false,

            calibration_request: false,
        }
    }

    #[inline]
    pub fn tick(&mut self) {
        if !self.buffer_empty_sent && self.gcode_buffer.len() == 0 {
            eth_send!("[{}] buffer empty\n\r", self.page_count);
            // eth.send(buf_writer);
            self.buffer_empty_sent = true;
            self.buffer_full_sent = false;
        }

        let data = global_ethernet::recv(&mut self.recv_buffer);

        if let None = data {
            return;
        }
        let data = data.unwrap();

        if let Ok(code_str) = core::str::from_utf8(data) {
            let codes = gcode::parse(&code_str);
            for code in codes {
                if let Err(_) = self.gcode_buffer.push(code) {
                    eth_send!("write failed, buffer full\n\r");
                    break;
                }
            }
            if self.gcode_buffer.len() >= 511 {
                eth_send!("[{}] buffer full", self.page_count);
                if !self.buffer_full_sent {
                    self.page_count += 1;
                }
                self.buffer_full_sent = true;
                self.buffer_empty_sent = false;
            }
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
        // if self.gcode_buffer.len() >= 500 { //dont wanna fill the entire thing up or else it will send `buffer full` message
        self.gcode_buffer.clear();
        // self.buffer_read_offset = 0;
        // } else if self.gcode_buffer.len() != 0 {
        //     self.buffer_read_offset = self.gcode_buffer.len() - 1;
        // }
    }

    #[inline]
    pub fn get_gcode_buffer(&self) -> &[gcode::GCode] {
        &self.gcode_buffer
    }
}
