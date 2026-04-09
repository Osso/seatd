mod client;
mod drm;
mod error;
mod protocol;
mod server;
mod vt;

use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "server" => run_server(),
        "open-seat" => cmd_open_seat(),
        "close-seat" => cmd_close_seat(),
        "open-device" => cmd_open_device(&args[2..]),
        "close-device" => cmd_close_device(&args[2..]),
        "ping" => cmd_ping(),
        _ => print_usage(),
    }
}

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

fn run_server() {
    println!("Starting seatd server...");
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        let mut server = server::SeatServer::new().expect("Failed to create server");
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });
}

fn cmd_open_seat() {
    match client::open_seat() {
        Ok(seat_id) => println!("Seat opened: {}", seat_id),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_close_seat() {
    match client::close_seat() {
        Ok(()) => println!("Seat closed"),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_open_device(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: seatd open-device <path>");
        return;
    }

    let path = Path::new(&args[0]);
    match client::open_device(path) {
        Ok((device_id, fd)) => {
            println!("Device opened: id={} fd={:?}", device_id, fd);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_close_device(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: seatd close-device <device_id>");
        return;
    }

    let device_id: u32 = match args[0].parse() {
        Ok(id) => id,
        Err(_) => {
            eprintln!("Invalid device ID");
            return;
        }
    };

    match client::close_device(device_id) {
        Ok(()) => println!("Device {} closed", device_id),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_ping() {
    match client::ping() {
        Ok(()) => println!("Pong!"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
