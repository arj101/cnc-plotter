use stm32h7xx_hal::usb_hs::USB1;
use synopsys_usb_otg::bus::UsbBus;
use usbd_serial::SerialPort;

use usb_device::bus::UsbBusAllocator;
use usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usb_device::UsbError;

use core::fmt::Debug;
use core::fmt::{self, Arguments};

use super::BufWriter;

#[derive(Debug)]
pub enum UsbWriteError {
    DeviceBusy,
    FormatError,
    WriteError(UsbError),
}

pub struct UsbSerial<'a> {
    serial_port: SerialPort<'a, UsbBus<USB1>>,
    usb_dev: UsbDevice<'a, UsbBus<USB1>>,
    fmt_buf: [u8; 128],
}

impl<'a> UsbSerial<'a> {
    pub fn new(usb_bus: &'a UsbBusAllocator<UsbBus<USB1>>) -> UsbSerial<'a> {
        let serial_port = SerialPort::new(usb_bus);

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x0483, 0x27dd))
            .manufacturer("Lbl")
            .product("eeeeeeeeeeeeeeeee")
            .serial_number("AAAAAAAAAAAAAAAA")
            .device_class(usbd_serial::USB_CLASS_CDC)
            .self_powered(true)
            .max_power(0)
            .build();

        UsbSerial {
            serial_port,
            usb_dev,
            fmt_buf: [0u8; 128],
        }
    }

    pub fn write(&mut self, args: Arguments) -> Result<(), UsbWriteError> {
        if !self.rts() {
            return Err(UsbWriteError::DeviceBusy);
        }

        let mut buf_writer = BufWriter::new(&mut self.fmt_buf);

        if let Err(_) = fmt::write(&mut buf_writer, args) {
            return Err(UsbWriteError::FormatError);
        }

        let bytes = buf_writer.get_bytes();

        let count = bytes.len();
        let mut write_offset = 0;

        while write_offset < count {
            match self.serial_port.write(&bytes[write_offset..count]) {
                Ok(len) if len > 0 => {
                    write_offset += len;
                }
                Ok(_) => {}
                Err(e) => match e {
                    UsbError::WouldBlock => {}
                    _ => return Err(UsbWriteError::WriteError(e)),
                },
            }
        }

        Ok(())
    }

    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<(), UsbWriteError> {
        if !self.rts() {
            return Err(UsbWriteError::DeviceBusy);
        }

        let count = buf.len();
        let mut write_offset = 0;

        while write_offset < count {
            match self.serial_port.write(&buf[write_offset..count]) {
                Ok(len) if len > 0 => {
                    write_offset += len;
                }
                Ok(_) => {}
                Err(e) => match e {
                    UsbError::WouldBlock => {}
                    _ => return Err(UsbWriteError::WriteError(e)),
                },
            }
        }

        Ok(())
    }

    pub fn wait_for_write(&self) {
        while !self.serial_port.rts() {}
    }

    pub fn rts(&self) -> bool {
        self.serial_port.rts()
    }

    pub fn poll(&mut self) -> bool {
        self.usb_dev.poll(&mut [&mut self.serial_port])
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, UsbError> {
        match self.serial_port.read(buf) {
            Ok(count) => Ok(count),
            Err(error) => Err(error),
        }
    }
}
