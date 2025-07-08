use plist::Dictionary;
use util_macro::UsbMuxPlist;

pub const USBMUX_VERSION: u32 = 1;
pub const USBMUX_MSGTYPE: u32 = 8;

#[derive(UsbMuxPlist)]
pub struct UsbmuxMessageHeader {
    pub version: u32,
    pub message: u32,
    pub tag: u32,
}
#[derive(UsbMuxPlist)]
pub struct UsbmuxMessageData {
    pub MessageType: String,
    pub PairRecordID: Option<String>,
    pub ClientVersionString: String,
    pub ProgName: String,
    pub kLibUSBMuxVersion: i64,
    pub DeviceID: Option<u16>,
    pub PortNumber: Option<u16>,
}
#[derive(UsbMuxPlist)]
pub struct UsbmuxMessage {
    pub header: UsbmuxMessageHeader,
    pub data: UsbmuxMessageData,
}

pub trait UsbMuxPlist {
    fn to_plist(&self) -> Dictionary;
}

#[derive(UsbMuxPlist)]
pub struct LockdownMessage {
    pub Label: Option<String>,
    pub Request: Option<String>,
    pub HostID: Option<String>,
    pub SystemBUID: Option<String>,
    pub Service: Option<String>,
    pub action: Option<u32>,
    pub Domain: Option<String>,
    pub Key: Option<String>,
}

#[cfg(test)]
mod test {
    use super::UsbmuxMessageHeader;
    use crate::usbmux::message::{
        UsbMuxPlist, UsbmuxMessage, UsbmuxMessageData, USBMUX_MSGTYPE, USBMUX_VERSION,
    };
    #[test]
    fn test_macro() {
        // let msg = UsbmuxMessage {
        //     header: UsbmuxMessageHeader {
        //         version: USBMUX_VERSION,
        //         message: USBMUX_MSGTYPE,
        //         tag: 1,
        //     },
        //     data: UsbmuxMessageData {
        //         MessageType: "ListDevices".to_string(),
        //         PairRecordID: None,
        //         ClientVersionString: "my-usbmuxd-client".to_string(),
        //         ProgName: "pymobiledevice-like".to_string(),
        //         kLibUSBMuxVersion: 3,
        //     },
        // };
        // println!("{:#?}", msg.to_plist())
    }
}
