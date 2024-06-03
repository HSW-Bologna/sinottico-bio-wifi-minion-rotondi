use super::mblp::{expected_response_len, Code, Command, Response};
use serialport::{available_ports, SerialPort};
use std::thread;
use std::time::{Duration, Instant};
use std::vec::Vec;

pub fn get_serial_ports() -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    match available_ports() {
        Ok(ports) => {
            for p in ports {
                result.push(p.port_name);
            }
        }
        Err(e) => {
            log::error!("Error listing serial ports: {:?}", e);
        }
    }

    return result;
}

pub fn send_command(
    port: &mut Box<dyn SerialPort>,
    code: Code,
    destination: [u8; 4],
    data: &[u8],
) -> Result<Response, String> {
    port.clear(serialport::ClearBuffer::All).ok();
    thread::sleep(Duration::from_millis(20));

    let command = Command::new(code, [0, 0, 0, 0], destination, data);

    let mut buffer: [u8; 256] = [0; 256];
    let len = command.serialize(&mut buffer);

    port.write(&buffer[0..len])
        .map_err(|_| String::from("Errore sulla porta"))?;
    let now = Instant::now();
    let expected_len = expected_response_len(code) as usize;

    while port.bytes_to_read().unwrap() < expected_response_len(code) {
        if Instant::now().duration_since(now) > Duration::from_millis(200) {
            return Err(String::from("Timeout!"));
        }
        //log::info!("{}", port.bytes_to_read().unwrap());
        thread::sleep(Duration::from_millis(10));
    }
    let mut read_buffer: [u8; 32] = [0; 32];
    let read_len = port
        .read(&mut read_buffer[0..expected_len])
        .map_err(|e| format!("Errore sulla porta: {:?}", e))?;

    if let Some(resp) = Response::parse(&mut read_buffer[0..read_len]) {
        Ok(resp)
    } else {
        Err(String::from(format!(
            "Risposta non valida ({} byte)",
            read_len
        )))
    }
}
