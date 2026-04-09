use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Request from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Open a seat and become the active client
    OpenSeat,

    /// Close the seat and release all devices
    CloseSeat,

    /// Request access to a device (fd returned via SCM_RIGHTS)
    OpenDevice { path: PathBuf },

    /// Release a previously opened device
    CloseDevice { device_id: u32 },

    /// Acknowledge that the client is ready to be disabled.
    /// Sent in response to a Disable event after releasing resources.
    DisableSeat,

    /// Request to switch to a different VT/session
    SwitchSession { vt: u32 },

    /// Ping to check connection
    Ping,
}

/// Response from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Seat opened successfully
    SeatOpened { seat_id: u32 },

    /// Seat closed
    SeatClosed,

    /// Device opened (fd sent via SCM_RIGHTS)
    DeviceOpened { device_id: u32 },

    /// Device closed
    DeviceClosed,

    /// Seat disabled acknowledged
    SeatDisabled,

    /// Session switch completed
    SessionSwitched,

    /// Pong response
    Pong,

    /// Error occurred
    Error { message: String },
}

/// Event pushed from server to client (unsolicited)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Session activated (VT switched to this session)
    Enable,

    /// Session deactivated (VT switched away)
    Disable,
}

/// Combined message type for server-to-client communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Response(Response),
    Event(Event),
}

pub const SOCKET_PATH: &str = "/run/seatd.sock";
