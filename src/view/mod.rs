pub mod app;


pub enum Message {
    ConnectToPort(String),
    Test(u32),
}