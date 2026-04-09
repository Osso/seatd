use thiserror::Error;

#[derive(Error, Debug)]
pub enum SeatError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("ipc: {0}")]
    Ipc(#[from] peercred_ipc::IpcError),

    #[error("no active seat")]
    NoSeat,

    #[error("seat already open")]
    SeatAlreadyOpen,

    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("invalid device: {0}")]
    InvalidDevice(String),
}
