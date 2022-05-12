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
                            .timeout(Duration::from_millis(20))
                            .stop_bits(serialport::StopBits::One)
                            .data_bits(serialport::DataBits::Eight);
                        match builder.open() {
                            Ok(opened_port) => {
                                self.port.replace(Some(opened_port));
                                self.modify_model(|m| {
                                    m.connection = Connection::Connected(port.clone());
                                });
                                self.notify("Connesso!".into());
                            }
                            Err(e) => {
                                log::warn!("Port connection error: {:?}", e);
                                self.notify("Errore di connessione!".into());
                            }
                        }
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
            serial::send_command(port, Code::SetAddress, destination, &destination)
                .map_err(|e| format!("Imposta codice: {}", e))?;

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
