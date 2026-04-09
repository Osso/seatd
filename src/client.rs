use crate::error::SeatError;
use crate::protocol::{Request, Response, SOCKET_PATH, ServerMessage};
use peercred_ipc::Client;
use std::os::fd::OwnedFd;
use std::path::Path;

/// Open a seat and return the seat ID
pub fn open_seat() -> Result<u32, SeatError> {
    open_seat_at(SOCKET_PATH)
}

/// Open a seat at a custom socket path
pub fn open_seat_at(socket_path: &str) -> Result<u32, SeatError> {
    let response: ServerMessage = Client::call(socket_path, &Request::OpenSeat)?;
    match response {
        ServerMessage::Response(Response::SeatOpened { seat_id }) => Ok(seat_id),
        ServerMessage::Response(Response::Error { message }) => {
            Err(SeatError::PermissionDenied(message))
        }
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}

/// Close the current seat
pub fn close_seat() -> Result<(), SeatError> {
    close_seat_at(SOCKET_PATH)
}

/// Close the current seat at a custom socket path
pub fn close_seat_at(socket_path: &str) -> Result<(), SeatError> {
    let response: ServerMessage = Client::call(socket_path, &Request::CloseSeat)?;
    match response {
        ServerMessage::Response(Response::SeatClosed) => Ok(()),
        ServerMessage::Response(Response::Error { message }) => {
            Err(SeatError::PermissionDenied(message))
        }
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}

/// Open a device and return (device_id, fd)
pub fn open_device(path: &Path) -> Result<(u32, OwnedFd), SeatError> {
    open_device_at(SOCKET_PATH, path)
}

/// Open a device at a custom socket path
pub fn open_device_at(socket_path: &str, path: &Path) -> Result<(u32, OwnedFd), SeatError> {
    let (response, fds): (ServerMessage, Vec<OwnedFd>) =
        Client::call_recv_fds(socket_path, &Request::OpenDevice { path: path.into() })?;

    match response {
        ServerMessage::Response(Response::DeviceOpened { device_id }) => {
            let fd = fds
                .into_iter()
                .next()
                .ok_or_else(|| SeatError::DeviceNotFound("no fd received".into()))?;
            Ok((device_id, fd))
        }
        ServerMessage::Response(Response::Error { message }) => {
            Err(SeatError::DeviceNotFound(message))
        }
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}

/// Close a device by ID
pub fn close_device(device_id: u32) -> Result<(), SeatError> {
    close_device_at(SOCKET_PATH, device_id)
}

/// Close a device at a custom socket path
pub fn close_device_at(socket_path: &str, device_id: u32) -> Result<(), SeatError> {
    let response: ServerMessage = Client::call(socket_path, &Request::CloseDevice { device_id })?;
    match response {
        ServerMessage::Response(Response::DeviceClosed) => Ok(()),
        ServerMessage::Response(Response::Error { message }) => {
            Err(SeatError::DeviceNotFound(message))
        }
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}

/// Ping the server
pub fn ping() -> Result<(), SeatError> {
    ping_at(SOCKET_PATH)
}

/// Ping the server at a custom socket path
pub fn ping_at(socket_path: &str) -> Result<(), SeatError> {
    let response: ServerMessage = Client::call(socket_path, &Request::Ping)?;
    match response {
        ServerMessage::Response(Response::Pong) => Ok(()),
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}
