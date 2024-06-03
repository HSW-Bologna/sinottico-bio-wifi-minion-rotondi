pub mod app;

pub enum Message {
    ConnectToPort(String),
    SetSerialNumber(u32),
    ReadSerialNumber(u32),
    ReadFWVersion(u32),
    DeviceAddress(String),
    Test(u32),
}

