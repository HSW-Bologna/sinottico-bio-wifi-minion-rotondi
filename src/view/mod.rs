pub mod app;


pub enum Message {
    ConnectToPort(String),
    SetSerialNumber(u32),
    ReadFWVersion(u32),
    Test(u32),
}