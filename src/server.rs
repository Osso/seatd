use crate::drm;
use crate::error::SeatError;
use crate::protocol::{Event, Request, Response, SOCKET_PATH, ServerMessage};
use peercred_ipc::{CallerInfo, Connection, Server};
use std::collections::HashMap;
use std::fs::File;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static DEVICE_ID_COUNTER: AtomicU32 = AtomicU32::new(1);
static SEAT_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Info about an opened device
struct DeviceInfo {
    fd: OwnedFd,
    path: PathBuf,
    is_drm: bool,
}

/// Active session state
struct Session {
    seat_id: u32,
    caller: CallerInfo,
    devices: HashMap<u32, DeviceInfo>,
    /// Whether the session is currently active (has DRM master)
    enabled: bool,
    /// Waiting for client to acknowledge disable
    pending_disable: bool,
}

/// Seat daemon server
pub struct SeatServer {
    server: Server,
    session: Option<Session>,
}

impl SeatServer {
    pub fn new() -> Result<Self, SeatError> {
        Self::new_with_path(SOCKET_PATH)
    }

    pub fn new_with_path(path: &str) -> Result<Self, SeatError> {
        let server = Server::bind_with_mode(path, 0o666)?;
        Ok(Self {
            server,
            session: None,
        })
    }

    pub async fn run(&mut self) -> Result<(), SeatError> {
        loop {
            let (conn, caller) = self.server.accept().await?;
            self.handle_client(conn, caller).await?;
        }
    }

    async fn handle_client(
        &mut self,
        mut conn: Connection,
        caller: CallerInfo,
    ) -> Result<(), SeatError> {
        println!(
            "Client connected: pid={} uid={} exe={:?}",
            caller.pid, caller.uid, caller.exe
        );

        loop {
            let request: Request = match conn.read().await {
                Ok(req) => req,
                Err(peercred_ipc::IpcError::ConnectionClosed) => {
                    println!("Client disconnected: pid={}", caller.pid);
                    self.cleanup_session(&caller);
                    break;
                }
                Err(e) => return Err(e.into()),
            };

            match self.handle_request(&mut conn, &caller, request).await {
                Ok(true) => continue,
                Ok(false) => break,
                Err(e) => {
                    let _ = conn
                        .write(&ServerMessage::Response(Response::Error {
                            message: e.to_string(),
                        }))
                        .await;
                }
            }
        }

        Ok(())
    }

    async fn handle_request(
        &mut self,
        conn: &mut Connection,
        caller: &CallerInfo,
        request: Request,
    ) -> Result<bool, SeatError> {
        match request {
            Request::OpenSeat => {
                let response = self.open_seat(caller)?;
                self.reply(conn, response).await?;
            }
            Request::CloseSeat => {
                let response = self.close_seat(caller)?;
                self.reply(conn, response).await?;
                return Ok(false);
            }
            Request::OpenDevice { path } => self.reply_open_device(conn, caller, &path).await?,
            Request::CloseDevice { device_id } => {
                let response = self.close_device(caller, device_id)?;
                self.reply(conn, response).await?;
            }
            Request::DisableSeat => {
                let response = self.disable_seat(caller)?;
                self.reply(conn, response).await?;
            }
            Request::SwitchSession { vt } => {
                let response = self.switch_session(caller, vt)?;
                self.reply(conn, response).await?;
            }
            Request::Ping => self.reply(conn, Response::Pong).await?,
        }
        Ok(true)
    }

    async fn reply(&self, conn: &mut Connection, response: Response) -> Result<(), SeatError> {
        conn.write(&ServerMessage::Response(response)).await?;
        Ok(())
    }

    async fn reply_open_device(
        &mut self,
        conn: &mut Connection,
        caller: &CallerInfo,
        path: &Path,
    ) -> Result<(), SeatError> {
        let (response, fd) = self.open_device(caller, path)?;
        if let Some(fd) = fd {
            conn.write_with_fds(&ServerMessage::Response(response), &[fd])
                .await?;
            return Ok(());
        }

        conn.write(&ServerMessage::Response(response)).await?;
        Ok(())
    }

    fn open_seat(&mut self, caller: &CallerInfo) -> Result<Response, SeatError> {
        if self.session.is_some() {
            return Err(SeatError::SeatAlreadyOpen);
        }

        let seat_id = SEAT_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        self.session = Some(Session {
            seat_id,
            caller: caller.clone(),
            devices: HashMap::new(),
            enabled: true,
            pending_disable: false,
        });

        println!("Seat {} opened by pid={}", seat_id, caller.pid);
        Ok(Response::SeatOpened { seat_id })
    }

    fn close_seat(&mut self, caller: &CallerInfo) -> Result<Response, SeatError> {
        let session = self.session.as_ref().ok_or(SeatError::NoSeat)?;
        if session.caller.pid != caller.pid {
            return Err(SeatError::PermissionDenied("not seat owner".into()));
        }

        let seat_id = session.seat_id;
        self.session = None;
        println!("Seat {} closed", seat_id);
        Ok(Response::SeatClosed)
    }

    fn open_device(
        &mut self,
        caller: &CallerInfo,
        path: &Path,
    ) -> Result<(Response, Option<RawFd>), SeatError> {
        let session = self.session.as_mut().ok_or(SeatError::NoSeat)?;
        if session.caller.pid != caller.pid {
            return Err(SeatError::PermissionDenied("not seat owner".into()));
        }

        if !is_allowed_device(path) {
            return Err(SeatError::InvalidDevice(format!(
                "device not allowed: {:?}",
                path
            )));
        }

        let file = File::open(path)
            .map_err(|e| SeatError::DeviceNotFound(format!("{}: {}", path.display(), e)))?;

        let device_id = DEVICE_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let raw_fd = file.as_raw_fd();
        let is_drm = drm::is_drm_device(path);

        // If this is a DRM device and session is enabled, set master
        if is_drm && session.enabled {
            if let Err(e) = drm::set_master(raw_fd) {
                println!("Warning: failed to set DRM master on {:?}: {}", path, e);
            }
        }

        let owned_fd: OwnedFd = file.into();
        session.devices.insert(
            device_id,
            DeviceInfo {
                fd: owned_fd,
                path: path.to_path_buf(),
                is_drm,
            },
        );

        println!("Device {} opened: {:?} (drm={})", device_id, path, is_drm);
        Ok((Response::DeviceOpened { device_id }, Some(raw_fd)))
    }

    fn close_device(&mut self, caller: &CallerInfo, device_id: u32) -> Result<Response, SeatError> {
        let session = self.session.as_mut().ok_or(SeatError::NoSeat)?;
        if session.caller.pid != caller.pid {
            return Err(SeatError::PermissionDenied("not seat owner".into()));
        }

        let Some(device) = session.devices.remove(&device_id) else {
            return Err(SeatError::DeviceNotFound(format!(
                "device_id {}",
                device_id
            )));
        };

        println!("Device {} closed: {:?}", device_id, device.path);
        Ok(Response::DeviceClosed)
    }

    /// Client acknowledges it's ready to be disabled
    fn disable_seat(&mut self, caller: &CallerInfo) -> Result<Response, SeatError> {
        {
            let session = self.session.as_ref().ok_or(SeatError::NoSeat)?;
            if session.caller.pid != caller.pid {
                return Err(SeatError::PermissionDenied("not seat owner".into()));
            }
            if !session.pending_disable {
                return Err(SeatError::InvalidDevice("no pending disable".into()));
            }
        }

        // Drop DRM master on all DRM devices
        self.drop_drm_master_all();

        let session = self.session.as_mut().unwrap();
        session.enabled = false;
        session.pending_disable = false;

        println!("Seat {} disabled", session.seat_id);
        Ok(Response::SeatDisabled)
    }

    /// Request to switch to a different VT
    fn switch_session(&mut self, caller: &CallerInfo, vt: u32) -> Result<Response, SeatError> {
        let session = self.session.as_ref().ok_or(SeatError::NoSeat)?;
        if session.caller.pid != caller.pid {
            return Err(SeatError::PermissionDenied("not seat owner".into()));
        }

        // TODO: Actually switch VT using vt module
        // For now, just acknowledge the request
        println!("Session switch requested to VT {}", vt);
        Ok(Response::SessionSwitched)
    }

    /// Drop DRM master on all DRM devices
    fn drop_drm_master_all(&mut self) {
        self.for_each_drm_device(drm::drop_master, "Dropped", "drop");
    }

    /// Set DRM master on all DRM devices
    fn set_drm_master_all(&mut self) {
        self.for_each_drm_device(drm::set_master, "Set", "set");
    }

    /// Send disable event to client and mark pending
    pub async fn send_disable(&mut self, conn: &mut Connection) -> Result<(), SeatError> {
        if let Some(session) = &mut self.session {
            session.pending_disable = true;
            conn.write(&ServerMessage::Event(Event::Disable)).await?;
            println!("Sent Disable event to session {}", session.seat_id);
        }
        Ok(())
    }

    /// Send enable event to client and restore DRM master
    pub async fn send_enable(&mut self, conn: &mut Connection) -> Result<(), SeatError> {
        if self.session.is_some() {
            self.set_drm_master_all();
            let session = self.session.as_mut().unwrap();
            session.enabled = true;
            let seat_id = session.seat_id;
            conn.write(&ServerMessage::Event(Event::Enable)).await?;
            println!("Sent Enable event to session {}", seat_id);
        }
        Ok(())
    }

    fn cleanup_session(&mut self, caller: &CallerInfo) {
        if let Some(session) = &self.session {
            if session.caller.pid == caller.pid {
                println!(
                    "Cleaning up session for disconnected client pid={}",
                    caller.pid
                );
                self.session = None;
            }
        }
    }

    fn for_each_drm_device(
        &self,
        action: fn(RawFd) -> std::io::Result<()>,
        success_verb: &str,
        failure_verb: &str,
    ) {
        let Some(session) = &self.session else {
            return;
        };

        for (device_id, info) in &session.devices {
            if !info.is_drm {
                continue;
            }

            if let Err(error) = action(info.fd.as_raw_fd()) {
                println!(
                    "Warning: failed to {} DRM master on device {} ({:?}): {}",
                    failure_verb, device_id, info.path, error
                );
                continue;
            }

            println!(
                "{} DRM master on device {} ({:?})",
                success_verb, device_id, info.path
            );
        }
    }
}

fn is_allowed_device(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // DRM devices (GPU)
    if path_str.starts_with("/dev/dri/") {
        return true;
    }

    // Input devices
    if path_str.starts_with("/dev/input/") {
        return true;
    }

    // TTY/VT devices
    if path_str.starts_with("/dev/tty") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_allowed_device_drm() {
        assert!(is_allowed_device(Path::new("/dev/dri/card0")));
        assert!(is_allowed_device(Path::new("/dev/dri/renderD128")));
    }

    #[test]
    fn test_is_allowed_device_input() {
        assert!(is_allowed_device(Path::new("/dev/input/event0")));
        assert!(is_allowed_device(Path::new("/dev/input/mouse0")));
    }

    #[test]
    fn test_is_allowed_device_tty() {
        assert!(is_allowed_device(Path::new("/dev/tty1")));
        assert!(is_allowed_device(Path::new("/dev/tty")));
    }

    #[test]
    fn test_is_allowed_device_blocked() {
        assert!(!is_allowed_device(Path::new("/dev/sda")));
        assert!(!is_allowed_device(Path::new("/dev/null")));
        assert!(!is_allowed_device(Path::new("/etc/passwd")));
        assert!(!is_allowed_device(Path::new("/dev/mem")));
    }
}
