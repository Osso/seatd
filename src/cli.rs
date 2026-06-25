use crate::SeatError;
#[cfg(not(coverage))]
use crate::client;
#[cfg(not(coverage))]
use crate::server::SeatServer;
use std::path::Path;

pub trait Commands {
    fn run_server(&self) -> Result<(), SeatError>;
    fn open_seat(&self) -> Result<u32, SeatError>;
    fn close_seat(&self) -> Result<(), SeatError>;
    fn open_device(&self, path: &Path) -> Result<(u32, String), SeatError>;
    fn close_device(&self, device_id: u32) -> Result<(), SeatError>;
    fn ping(&self) -> Result<(), SeatError>;
}

#[cfg(not(coverage))]
pub struct RealCommands;

#[cfg(not(coverage))]
impl Commands for RealCommands {
    fn run_server(&self) -> Result<(), SeatError> {
        println!("Starting seatd server...");
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            let mut server = SeatServer::new().expect("Failed to create server");
            server.run().await
        })
    }

    fn open_seat(&self) -> Result<u32, SeatError> {
        client::open_seat()
    }

    fn close_seat(&self) -> Result<(), SeatError> {
        client::close_seat()
    }

    fn open_device(&self, path: &Path) -> Result<(u32, String), SeatError> {
        client::open_device(path).map(|(device_id, fd)| (device_id, format!("{fd:?}")))
    }

    fn close_device(&self, device_id: u32) -> Result<(), SeatError> {
        client::close_device(device_id)
    }

    fn ping(&self) -> Result<(), SeatError> {
        client::ping()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Stdout(String),
    Stderr(String),
    Usage,
}

pub fn dispatch(args: &[String], commands: &impl Commands) -> Outcome {
    let Some(command) = args.get(1).map(String::as_str) else {
        return Outcome::Usage;
    };

    match command {
        "server" => result_to_outcome(commands.run_server(), |_| "Server stopped".to_string()),
        "open-seat" => result_to_outcome(commands.open_seat(), |seat_id| {
            format!("Seat opened: {seat_id}")
        }),
        "close-seat" => result_to_outcome(commands.close_seat(), |()| "Seat closed".to_string()),
        "open-device" => open_device(args, commands),
        "close-device" => close_device(args, commands),
        "ping" => result_to_outcome(commands.ping(), |()| "Pong!".to_string()),
        _ => Outcome::Usage,
    }
}

#[cfg(not(coverage))]
pub fn print_outcome(outcome: Outcome) {
    match outcome {
        Outcome::Stdout(message) => println!("{message}"),
        Outcome::Stderr(message) => eprintln!("{message}"),
        Outcome::Usage => print_usage(),
    }
}

fn open_device(args: &[String], commands: &impl Commands) -> Outcome {
    let Some(path) = args.get(2) else {
        return Outcome::Stderr("Usage: seatd open-device <path>".to_string());
    };

    result_to_outcome(commands.open_device(Path::new(path)), |(device_id, fd)| {
        format!("Device opened: id={device_id} fd={fd}")
    })
}

fn close_device(args: &[String], commands: &impl Commands) -> Outcome {
    let Some(device_id) = args.get(2) else {
        return Outcome::Stderr("Usage: seatd close-device <device_id>".to_string());
    };

    match device_id.parse() {
        Ok(device_id) => result_to_outcome(commands.close_device(device_id), |()| {
            format!("Device {device_id} closed")
        }),
        Err(_) => Outcome::Stderr("Invalid device ID".to_string()),
    }
}

fn result_to_outcome<T>(
    result: Result<T, SeatError>,
    success: impl FnOnce(T) -> String,
) -> Outcome {
    match result {
        Ok(value) => Outcome::Stdout(success(value)),
        Err(error) => Outcome::Stderr(format!("Error: {error}")),
    }
}

#[cfg(not(coverage))]
fn print_usage() {
    eprintln!("Usage: seatd <command>");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  server                  Run the seat daemon");
    eprintln!("  open-seat               Open a seat");
    eprintln!("  close-seat              Close the current seat");
    eprintln!("  open-device <path>      Open a device (e.g., /dev/dri/card0)");
    eprintln!("  close-device <id>       Close a device by ID");
    eprintln!("  ping                    Ping the server");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SeatError;
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    struct FakeCommands {
        calls: RefCell<Vec<String>>,
        error: Option<SeatError>,
    }

    impl FakeCommands {
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }

        fn result<T>(&self, value: T) -> Result<T, SeatError> {
            match &self.error {
                Some(error) => Err(SeatError::PermissionDenied(error.to_string())),
                None => Ok(value),
            }
        }
    }

    impl Commands for FakeCommands {
        fn run_server(&self) -> Result<(), SeatError> {
            self.calls.borrow_mut().push("server".to_string());
            self.result(())
        }

        fn open_seat(&self) -> Result<u32, SeatError> {
            self.calls.borrow_mut().push("open-seat".to_string());
            self.result(42)
        }

        fn close_seat(&self) -> Result<(), SeatError> {
            self.calls.borrow_mut().push("close-seat".to_string());
            self.result(())
        }

        fn open_device(&self, path: &Path) -> Result<(u32, String), SeatError> {
            self.calls
                .borrow_mut()
                .push(format!("open-device:{}", path.display()));
            self.result((7, "fd(3)".to_string()))
        }

        fn close_device(&self, device_id: u32) -> Result<(), SeatError> {
            self.calls
                .borrow_mut()
                .push(format!("close-device:{device_id}"));
            self.result(())
        }

        fn ping(&self) -> Result<(), SeatError> {
            self.calls.borrow_mut().push("ping".to_string());
            self.result(())
        }
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn dispatch_without_command_prints_usage() {
        let commands = FakeCommands::default();

        assert_eq!(dispatch(&args(&["seatd"]), &commands), Outcome::Usage);
        assert!(commands.calls().is_empty());
    }

    #[test]
    fn dispatch_unknown_command_prints_usage() {
        let commands = FakeCommands::default();

        assert_eq!(
            dispatch(&args(&["seatd", "bogus"]), &commands),
            Outcome::Usage
        );
        assert!(commands.calls().is_empty());
    }

    #[test]
    fn dispatch_runs_simple_commands() {
        let commands = FakeCommands::default();

        assert_eq!(
            dispatch(&args(&["seatd", "open-seat"]), &commands),
            Outcome::Stdout("Seat opened: 42".to_string())
        );
        assert_eq!(
            dispatch(&args(&["seatd", "close-seat"]), &commands),
            Outcome::Stdout("Seat closed".to_string())
        );
        assert_eq!(
            dispatch(&args(&["seatd", "ping"]), &commands),
            Outcome::Stdout("Pong!".to_string())
        );
        assert_eq!(commands.calls(), vec!["open-seat", "close-seat", "ping"]);
    }

    #[test]
    fn dispatch_runs_server() {
        let commands = FakeCommands::default();

        assert_eq!(
            dispatch(&args(&["seatd", "server"]), &commands),
            Outcome::Stdout("Server stopped".to_string())
        );
        assert_eq!(commands.calls(), vec!["server"]);
    }

    #[test]
    fn dispatch_opens_device() {
        let commands = FakeCommands::default();

        assert_eq!(
            dispatch(&args(&["seatd", "open-device", "/dev/tty1"]), &commands),
            Outcome::Stdout("Device opened: id=7 fd=fd(3)".to_string())
        );
        assert_eq!(commands.calls(), vec!["open-device:/dev/tty1"]);
    }

    #[test]
    fn dispatch_requires_open_device_path() {
        let commands = FakeCommands::default();

        assert_eq!(
            dispatch(&args(&["seatd", "open-device"]), &commands),
            Outcome::Stderr("Usage: seatd open-device <path>".to_string())
        );
        assert!(commands.calls().is_empty());
    }

    #[test]
    fn dispatch_closes_device() {
        let commands = FakeCommands::default();

        assert_eq!(
            dispatch(&args(&["seatd", "close-device", "9"]), &commands),
            Outcome::Stdout("Device 9 closed".to_string())
        );
        assert_eq!(commands.calls(), vec!["close-device:9"]);
    }

    #[test]
    fn dispatch_rejects_invalid_close_device_id() {
        let commands = FakeCommands::default();

        assert_eq!(
            dispatch(&args(&["seatd", "close-device", "not-a-number"]), &commands),
            Outcome::Stderr("Invalid device ID".to_string())
        );
        assert!(commands.calls().is_empty());
    }

    #[test]
    fn dispatch_requires_close_device_id() {
        let commands = FakeCommands::default();

        assert_eq!(
            dispatch(&args(&["seatd", "close-device"]), &commands),
            Outcome::Stderr("Usage: seatd close-device <device_id>".to_string())
        );
        assert!(commands.calls().is_empty());
    }

    #[test]
    fn dispatch_reports_command_errors() {
        let commands = FakeCommands {
            calls: RefCell::new(Vec::new()),
            error: Some(SeatError::PermissionDenied("denied".to_string())),
        };

        assert_eq!(
            dispatch(&args(&["seatd", "ping"]), &commands),
            Outcome::Stderr("Error: permission denied: permission denied: denied".to_string())
        );
        assert_eq!(commands.calls(), vec!["ping"]);
    }

    #[test]
    fn fake_open_device_records_owned_path() {
        let commands = FakeCommands::default();
        let path = PathBuf::from("/dev/input/event0");

        assert_eq!(
            commands.open_device(&path).unwrap(),
            (7, "fd(3)".to_string())
        );
        assert_eq!(commands.calls(), vec!["open-device:/dev/input/event0"]);
    }
}
