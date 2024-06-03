// MBSoft legacy protocol

pub const NORMAL_COMMAND: u8 = 0x01;
pub const PROBE_ID_COMMAND: u8 = 0x00;

pub const PREAMBLE: u8 = 0x02;
pub const HEADER_LENGTH: usize = 15;
pub const RESPONSE_HEADER_LENGTH: usize = 14;

const CODE_U16: [(u16, Code); 5] = [
    (0x0101, Code::ReadInput),
    (0xFF01, Code::SetOutput),
    (0xFF03, Code::SetAddress),
    (0xFF04, Code::ReadAddress),
    (0x400A, Code::ReadFWVersion),
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Code {
    ReadInput,
    SetOutput,
    SetAddress,
    ReadAddress,
    ReadFWVersion,
    Unknown(u8, u8),
}

pub fn expected_response_len(code: Code) -> u32 {
    match code {
        Code::SetAddress => 15,
        Code::ReadInput => 15,
        Code::SetOutput => 14,
        Code::ReadFWVersion => 18,
        Code::ReadAddress => 18,
        _ => 0,
    }
}

impl From<(u8, u8)> for Code {
    fn from(code: (u8, u8)) -> Self {
        let (first, second) = code;
        let code: u16 = ((first as u16) << 8) | (second as u16);
        Self::from(code)
    }
}

impl From<u16> for Code {
    fn from(num: u16) -> Self {
        for &(associated_num, code) in &CODE_U16 {
            if num == associated_num {
                return code;
            }
        }

        Self::Unknown((num >> 8) as u8, num as u8)
    }
}

impl Into<u16> for Code {
    fn into(self) -> u16 {
        for &(num, code) in &CODE_U16 {
            if code == self {
                return num;
            }
        }

        0xFFFF
    }
}

pub fn crc(data: &[u8]) -> u8 {
    data.iter()
        .fold(0 as u8, |sum, item| sum.wrapping_add(*item))
}

pub struct Command {
    pub data_len: u8,
    pub destination: [u8; 4],
    pub source: [u8; 4],
    pub code: Code,
    pub data: [u8; 256],
}

impl core::fmt::Display for Command {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Packet {:?} from {:?} to {:?}, {} bytes of data:\n\t",
            self.code, self.source, self.destination, self.data_len
        )?;

        for b in &self.data[0..self.data_len as usize] {
            write!(f, "0x{:02X} ", b)?;
        }

        write!(f, "\n")
    }
}

impl Command {
    pub const MIN_PACKET_LEN: usize = 0x0F;

    pub fn new(code: Code, source: [u8; 4], destination: [u8; 4], data: &[u8]) -> Self {
        Command {
            data_len: data.len() as u8,
            destination,
            source,
            code,
            data: array_init::array_init(|x| if x < data.len() { data[x] } else { 0 }),
        }
    }

    pub fn parse(buffer: &[u8]) -> Option<Command> {
        if buffer.len() < Self::MIN_PACKET_LEN {
            return None;
        }

        if buffer[0] == PREAMBLE {
            let start = 0;
            let len = buffer[start + 2];

            if (len as usize) < Self::MIN_PACKET_LEN {
                return None;
            }

            if buffer.len() >= len as usize {
                let crc = crc(&buffer[0..len as usize - 1]);
                if buffer[len as usize - 1] == crc {
                    // Caso speciale del comando scrivi ID CPU
                    let start: usize = 12;
                    return Some(Command {
                        data_len: len - Self::MIN_PACKET_LEN as u8,
                        destination: array_init::array_init(|x| buffer[4 + x]),
                        source: array_init::array_init(|x| buffer[8 + x]),
                        code: Code::from((buffer[start], buffer[start + 1])),
                        data: array_init::array_init(|x| {
                            let index = start + 2 + x;
                            if index < buffer.len() - 1 {
                                buffer[index]
                            } else {
                                0
                            }
                        }),
                    });
                }
            }
        }
        return None;
    }

    pub fn serialize(self, buffer: &mut [u8]) -> usize {
        buffer[0] = PREAMBLE;
        buffer[1] = 1;
        buffer[3] = 0;
        buffer[4..8].clone_from_slice(&self.destination[0..4]);
        buffer[8..12].clone_from_slice(&self.source[0..4]);

        let dlen = self.data_len as usize;
        buffer[2] = HEADER_LENGTH as u8 + self.data_len;

        let cmdcode: u16 = self.code.into();
        buffer[12] = ((cmdcode >> 8) & 0xFF) as u8;
        buffer[13] = (cmdcode & 0xFF) as u8;
        buffer[14..14 + dlen].clone_from_slice(&self.data[0..dlen]);
        let c = crc(&buffer[0..14 + dlen]);
        buffer[14 + dlen] = c;
        dlen + HEADER_LENGTH
    }
}

#[derive(Copy, Clone)]
pub enum ResponseType {
    Usual,
    CpuID,
    LegacyCipher,
}

#[derive(Copy, Clone)]
pub struct Response {
    pub data_len: u8,
    pub destination: [u8; 4],
    pub source: [u8; 4],
    pub error: bool,
    pub response_type: ResponseType,
    pub data: [u8; 256],
}

impl Response {
    pub const MIN_PACKET_LEN: usize = 0x0E;

    pub fn ok(destination: [u8; 4], source: [u8; 4], data: &[u8]) -> Self {
        assert!(data.len() < 256);
        Response {
            data_len: data.len() as u8,
            destination,
            source,
            response_type: ResponseType::Usual,
            error: false,
            data: array_init::array_init(|i| if i < data.len() { data[i] } else { 0 }),
        }
    }

    pub fn err(destination: [u8; 4], source: [u8; 4], data: &[u8]) -> Self {
        assert!(data.len() < 256);
        Response {
            data_len: data.len() as u8,
            destination,
            source,
            error: true,
            response_type: ResponseType::Usual,
            data: array_init::array_init(|i| if i < data.len() { data[i] } else { 0 }),
        }
    }

    pub fn serialize(self, buffer: &mut [u8]) -> usize {
        buffer[0] = PREAMBLE;
        buffer[3] = 0;
        buffer[4..8].clone_from_slice(&self.destination[0..4]);
        buffer[8..12].clone_from_slice(&self.source[0..4]);

        match self.response_type {
            ResponseType::Usual => {
                buffer[1] = 1;
                let dlen = self.data_len as usize;
                buffer[2] = 14 + self.data_len;
                buffer[12] = if self.error { 1 } else { 0 };
                buffer[13..13 + dlen].clone_from_slice(&self.data[0..dlen]);
                let c = crc(&buffer[0..13 + dlen]);
                buffer[13 + dlen] = c;
                dlen + RESPONSE_HEADER_LENGTH
            }

            ResponseType::CpuID => {
                buffer[1] = 0;
                buffer[2] = 18;
                buffer[12] = 0;
                buffer[13..17].clone_from_slice(&self.source[0..4]);
                buffer[17] = crc(&buffer[0..17]);
                18
            }

            ResponseType::LegacyCipher => {
                let zeros: [u8; 9] = [0, 33, 1, 5, 5, 4, 2, 255, 37];
                //0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0x21, 1, 5, 5, 4, 2, 0xff, 0x25, 0x70, ];
                buffer[1] = 1;
                buffer[2] = 22;
                buffer[12..21].clone_from_slice(&zeros);
                buffer[21] = crc(&buffer[0..22]);
                22
            }
        }
    }

    pub fn parse(buffer: &[u8]) -> Option<Response> {
        if buffer.len() < Self::MIN_PACKET_LEN {
            return None;
        }

        if buffer[0] == PREAMBLE && buffer[1] == 1 {
            let start = 0;
            let len = buffer[start + 2];

            if (len as usize) < Self::MIN_PACKET_LEN {
                return None;
            }

            if buffer.len() >= len as usize {
                let data_len = len - Self::MIN_PACKET_LEN as u8;
                let crc = crc(&buffer[0..len as usize - 1]);

                if buffer[len as usize - 1] == crc {
                    return Some(Response {
                        error: false, //buffer[12] != 1,
                        response_type: ResponseType::Usual,
                        data_len,
                        destination: array_init::array_init(|x| buffer[4 + x]),
                        source: array_init::array_init(|x| buffer[8 + x]),
                        data: array_init::array_init(|x| {
                            if x < data_len as usize {
                                let index = 13 + x;
                                buffer[index]
                            } else {
                                0
                            }
                        }),
                    });
                } else {
                    log::warn!("Invalid CRC ({} - {})!", buffer[len as usize - 1], crc);
                }
            } else {
                log::warn!("Invalid len ({} - {})!", buffer.len(), len);
            }
        }
        return None;
    }
}

impl core::fmt::Display for Response {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Response from {:?} to {:?}, (error: {}) {} bytes of data:\n\t",
            self.source, self.destination, self.error, self.data_len
        )?;

        for b in &self.data[0..self.data_len as usize] {
            write!(f, "0x{:02X} ", b)?;
        }

        write!(f, "\n")
    }
}
