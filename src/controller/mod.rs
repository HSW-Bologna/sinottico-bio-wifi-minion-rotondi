use std::cell::RefCell;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod mblp;
mod serial;

use crate::model::{Connection, Model};
use crate::view;
use mblp::Code;

pub struct Controller {
    model: Arc<Mutex<Model>>,
    ctx: egui::Context,
    rx: mpsc::Receiver<view::Message>,
    tx: mpsc::Sender<view::Message>,

    port: RefCell<Option<Box<dyn serialport::SerialPort>>>,
}

impl Controller {
    pub fn new(model: Arc<Mutex<Model>>, ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        Controller {
            model,
            ctx,
            rx,
            tx,
            port: RefCell::new(None),
        }
    }

    pub fn start(self: Self) {
        thread::spawn(move || self.task());
    }

    pub fn get_command_channel(self: &Self) -> mpsc::Sender<view::Message> {
        self.tx.clone()
    }

    fn modify_model<F>(self: &Self, mut op: F)
    where
        F: FnMut(&mut Model),
    {
        let mut model = self.model.lock().unwrap();
        op(&mut model);
        self.ctx.request_repaint();
    }

    fn task(self: Self) {
        let mut portts: Instant = Instant::now();

        loop {
            if let Some(msg) = self.rx.recv_timeout(Duration::from_millis(100)).ok() {
                use view::Message::*;
                match msg {
                    ConnectToPort(port) => {
                        let builder = serialport::new(port.clone(), 9600)
                            .timeout(Duration::from_millis(100))
                            .stop_bits(serialport::StopBits::One)
                            .data_bits(serialport::DataBits::Eight);
                        match builder.open() {
                            Ok(opened_port) => {
                                self.port.replace(Some(opened_port));
                                self.modify_model(|m| {
                                    m.connection = Connection::Connected(port.clone());
                                });
                                self.notify("Connesso!".into());

                                match self.read_serial_number([0; 4]) {
                                    Ok(sn) => {
                                        self.modify_model(|m| {
                                            m.device_address = format!("{:08X}", sn)
                                        });
                                        self.notify(format!("Indirizzo 0x{:08X}", sn));
                                    }
                                    Err(e) => {
                                        self.notify(e);
                                        self.notify("Indirizzo non recuperata".into());
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("Port connection error: {:?}", e);
                                self.notify("Errore di connessione!".into());
                            }
                        }
                    }

                    ReadFWVersion(address) => {
                        let destination = u32::to_be_bytes(address);
                        match self.read_firmware_version(destination) {
                            Ok(fw @ (fw1, fw2, fw3)) => {
                                self.modify_model(|m| m.version = Some(fw));
                                self.notify(format!("Versione firmware {}.{}.{}", fw1, fw2, fw3));
                            }
                            Err(e) => {
                                self.notify(e);
                                self.notify("Versione firmware non recuperata".into());
                            }
                        }
                    }

                    ReadSerialNumber(address) => {
                        let destination = u32::to_be_bytes(address);
                        match self.read_serial_number(destination) {
                            Ok(sn) => {
                                self.modify_model(|m| m.device_address = format!("{:08X}", sn));
                                self.notify(format!("Indirizzo 0x{:08X}", sn));
                            }
                            Err(e) => {
                                self.notify(e);
                                self.notify("Indirizzo non recuperata".into());
                            }
                        }
                    }

                    SetSerialNumber(address) => {
                        let destination = u32::to_be_bytes(address);
                        match self.set_serial_number(destination) {
                            Ok(()) => self.notify("Numero di matricola impostato".into()),
                            Err(e) => {
                                self.notify(e);
                                self.notify("Impostazione fallita".into());
                            }
                        }
                    }

                    DeviceAddress(address) => {
                        self.modify_model(|m| m.device_address = address.clone());
                    }

                    Test(address) => {
                        let destination = u32::to_be_bytes(address);
                        match self.test_device(destination) {
                            Ok(()) => self.notify("Collaudo concluso con successo".into()),
                            Err(e) => {
                                self.notify(e);
                                self.notify("Collaudo fallito".into());
                            }
                        }
                    }
                }
            }

            if Instant::now().duration_since(portts) > Duration::from_millis(500) {
                self.modify_model(|m| m.ports = serial::get_serial_ports());
                portts = Instant::now();
            }
        }
    }

    fn notify(self: &Self, msg: String) {
        self.modify_model(|m| m.message(msg.clone()))
    }

    fn read_firmware_version(self: &Self, destination: [u8; 4]) -> Result<(u8, u8, u8), String> {
        if let Some(ref mut port) = self.port.borrow_mut().as_mut() {
            let resp = serial::send_command(port, Code::ReadFWVersion, destination, &destination)
                .map_err(|e| format!("Leggi firmware: {}", e))?;

            if resp.data_len > 3 {
                Ok((resp.data[0], resp.data[1], resp.data[2]))
            } else {
                Err(String::from("Risposta non valida"))
            }
        } else {
            Err(String::from("Nessuna porta connessa!"))
        }
    }

    fn read_serial_number(self: &Self, destination: [u8; 4]) -> Result<u32, String> {
        if let Some(ref mut port) = self.port.borrow_mut().as_mut() {
            let resp = serial::send_command(port, Code::ReadAddress, destination, &destination)
                .map_err(|e| format!("Leggi indirizzo: {}", e))?;

            if resp.data_len > 3 {
                Ok(u32::from_be_bytes([
                    resp.data[0],
                    resp.data[1],
                    resp.data[2],
                    resp.data[3],
                ]))
            } else {
                Err(String::from("Risposta non valida"))
            }
        } else {
            Err(String::from("Nessuna porta connessa!"))
        }
    }

    fn set_serial_number(self: &Self, destination: [u8; 4]) -> Result<(), String> {
        if let Some(ref mut port) = self.port.borrow_mut().as_mut() {
            serial::send_command(port, Code::SetAddress, destination, &destination)
                .map_err(|e| format!("Imposta codice: {}", e))?;
            Ok(())
        } else {
            Err(String::from("Nessuna porta connessa!"))
        }
    }

    fn test_device(self: &Self, destination: [u8; 4]) -> Result<(), String> {
        fn check_input(
            port: &mut Box<dyn serialport::SerialPort>,
            destination: [u8; 4],
            step: u16,
            expected: u8,
        ) -> Result<(), String> {
            let response = serial::send_command(port, Code::ReadInput, destination, &[])
                .map_err(|e| format!("Leggi ingressi {}: {}", step, e))?;

            if response.data_len < 1 {
                return Err(String::from("Ingressi non ottenuti"));
            }

            if response.data[0] != expected {
                Err(String::from(format!(
                    "Ingressi non validi: mi aspettavo 0x{:02X}, e' arrivato 0x{:02X}",
                    expected, response.data[0]
                )))
            } else {
                Ok(())
            }
        }

        if let Some(ref mut port) = self.port.borrow_mut().as_mut() {
            for i in 0..4 {
                serial::send_command(port, Code::SetOutput, destination, &[i, 0])
                    .map_err(|e| format!("Spegni rele {}: {}", i, e))?;
            }

            check_input(port, destination, 0, 0x00)?;

            for i in 0..4 {
                serial::send_command(port, Code::SetOutput, destination, &[i, 1])
                    .map_err(|e| format!("Accendi rele {}: {}", i, e))?;
                thread::sleep(Duration::from_millis(100));
                check_input(port, destination, 0, 1 << i)?;
                serial::send_command(port, Code::SetOutput, destination, &[i, 0])
                    .map_err(|e| format!("Spegni rele {}: {}", i, e))?;
                thread::sleep(Duration::from_millis(100));
            }

            Ok(())
        } else {
            Err(String::from("Nessuna porta connessa!"))
        }
    }
}
