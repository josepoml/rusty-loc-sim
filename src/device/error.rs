use crate::usbmux::errors::UsbmuxOperationError;

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("UsbmuxOperation error: {0}")]
    UsbMuxOperationError(#[from] UsbmuxOperationError),
    #[error("Error: {0}")]
    Error(&'static str),
    #[error("XPC error: {0}")]
    XpcError(#[from] crate::xpc::errors::XpcError),
    #[error("DtService error: {0}")]
    DtServiceError(#[from] crate::dtservice::errors::DtServiceError),
}
