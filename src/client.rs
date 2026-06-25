use crate::error::SeatError;
#[cfg(not(coverage))]
use crate::protocol::SOCKET_PATH;
use crate::protocol::{Request, Response, ServerMessage};
use peercred_ipc::Client;
use std::os::fd::OwnedFd;
use std::path::Path;

/// Open a seat and return the seat ID
#[cfg(not(coverage))]
pub fn open_seat() -> Result<u32, SeatError> {
    open_seat_at(SOCKET_PATH)
}

/// Open a seat at a custom socket path
pub fn open_seat_at(socket_path: &str) -> Result<u32, SeatError> {
    let response: ServerMessage = Client::call(socket_path, &Request::OpenSeat)?;
    decode_open_seat(response)
}

fn decode_open_seat(response: ServerMessage) -> Result<u32, SeatError> {
    match response {
        ServerMessage::Response(Response::SeatOpened { seat_id }) => Ok(seat_id),
        ServerMessage::Response(Response::Error { message }) => {
            Err(SeatError::PermissionDenied(message))
        }
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}

/// Close the current seat
#[cfg(not(coverage))]
pub fn close_seat() -> Result<(), SeatError> {
    close_seat_at(SOCKET_PATH)
}

/// Close the current seat at a custom socket path
pub fn close_seat_at(socket_path: &str) -> Result<(), SeatError> {
    let response: ServerMessage = Client::call(socket_path, &Request::CloseSeat)?;
    decode_close_seat(response)
}

fn decode_close_seat(response: ServerMessage) -> Result<(), SeatError> {
    match response {
        ServerMessage::Response(Response::SeatClosed) => Ok(()),
        ServerMessage::Response(Response::Error { message }) => {
            Err(SeatError::PermissionDenied(message))
        }
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}

/// Open a device and return (device_id, fd)
#[cfg(not(coverage))]
pub fn open_device(path: &Path) -> Result<(u32, OwnedFd), SeatError> {
    open_device_at(SOCKET_PATH, path)
}

/// Open a device at a custom socket path
pub fn open_device_at(socket_path: &str, path: &Path) -> Result<(u32, OwnedFd), SeatError> {
    let (response, fds): (ServerMessage, Vec<OwnedFd>) =
        Client::call_recv_fds(socket_path, &Request::OpenDevice { path: path.into() })?;

    decode_open_device(response, fds)
}

fn decode_open_device(
    response: ServerMessage,
    fds: Vec<OwnedFd>,
) -> Result<(u32, OwnedFd), SeatError> {
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
#[cfg(not(coverage))]
pub fn close_device(device_id: u32) -> Result<(), SeatError> {
    close_device_at(SOCKET_PATH, device_id)
}

/// Close a device at a custom socket path
pub fn close_device_at(socket_path: &str, device_id: u32) -> Result<(), SeatError> {
    let response: ServerMessage = Client::call(socket_path, &Request::CloseDevice { device_id })?;
    decode_close_device(response)
}

fn decode_close_device(response: ServerMessage) -> Result<(), SeatError> {
    match response {
        ServerMessage::Response(Response::DeviceClosed) => Ok(()),
        ServerMessage::Response(Response::Error { message }) => {
            Err(SeatError::DeviceNotFound(message))
        }
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}

/// Ping the server
#[cfg(not(coverage))]
pub fn ping() -> Result<(), SeatError> {
    ping_at(SOCKET_PATH)
}

/// Ping the server at a custom socket path
pub fn ping_at(socket_path: &str) -> Result<(), SeatError> {
    let response: ServerMessage = Client::call(socket_path, &Request::Ping)?;
    decode_ping(response)
}

fn decode_ping(response: ServerMessage) -> Result<(), SeatError> {
    match response {
        ServerMessage::Response(Response::Pong) => Ok(()),
        _ => Err(SeatError::PermissionDenied("unexpected response".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn assert_contains_error<T: std::fmt::Debug>(result: Result<T, SeatError>, needle: &str) {
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains(needle),
            "expected error containing {needle:?}, got {error:?}"
        );
    }

    #[test]
    fn decodes_open_seat_responses() {
        assert_eq!(
            decode_open_seat(ServerMessage::Response(Response::SeatOpened { seat_id: 9 })).unwrap(),
            9
        );
        assert_contains_error(
            decode_open_seat(ServerMessage::Response(Response::Error {
                message: "denied".to_string(),
            })),
            "denied",
        );
        assert_contains_error(
            decode_open_seat(ServerMessage::Response(Response::Pong)),
            "unexpected response",
        );
    }

    #[test]
    fn decodes_close_seat_responses() {
        assert!(decode_close_seat(ServerMessage::Response(Response::SeatClosed)).is_ok());
        assert_contains_error(
            decode_close_seat(ServerMessage::Response(Response::Error {
                message: "no seat".to_string(),
            })),
            "no seat",
        );
        assert_contains_error(
            decode_close_seat(ServerMessage::Response(Response::Pong)),
            "unexpected response",
        );
    }

    #[test]
    fn decodes_open_device_responses() {
        let fd: OwnedFd = File::open("/dev/null").unwrap().into();
        let (device_id, _fd) = decode_open_device(
            ServerMessage::Response(Response::DeviceOpened { device_id: 3 }),
            vec![fd],
        )
        .unwrap();
        assert_eq!(device_id, 3);

        assert_contains_error(
            decode_open_device(
                ServerMessage::Response(Response::DeviceOpened { device_id: 3 }),
                Vec::new(),
            ),
            "no fd received",
        );
        assert_contains_error(
            decode_open_device(
                ServerMessage::Response(Response::Error {
                    message: "missing".to_string(),
                }),
                Vec::new(),
            ),
            "missing",
        );
        assert_contains_error(
            decode_open_device(ServerMessage::Response(Response::Pong), Vec::new()),
            "unexpected response",
        );
    }

    #[test]
    fn decodes_close_device_responses() {
        assert!(decode_close_device(ServerMessage::Response(Response::DeviceClosed)).is_ok());
        assert_contains_error(
            decode_close_device(ServerMessage::Response(Response::Error {
                message: "missing".to_string(),
            })),
            "missing",
        );
        assert_contains_error(
            decode_close_device(ServerMessage::Response(Response::Pong)),
            "unexpected response",
        );
    }

    #[test]
    fn decodes_ping_responses() {
        assert!(decode_ping(ServerMessage::Response(Response::Pong)).is_ok());
        assert_contains_error(
            decode_ping(ServerMessage::Response(Response::SeatClosed)),
            "unexpected response",
        );
    }
}
