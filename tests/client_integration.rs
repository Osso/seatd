use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use seatd::client;
use seatd::server::SeatServer;

static PORT_COUNTER: AtomicU32 = AtomicU32::new(0);

fn test_socket_path() -> String {
    let id = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    format!(
        "{}/seatd-client-test-{}-{}.sock",
        base,
        std::process::id(),
        id
    )
}

#[tokio::test]
async fn test_client_ping() {
    let socket_path = test_socket_path();

    let mut server = SeatServer::new_with_path(&socket_path).unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let path = socket_path.clone();
    let result = tokio::task::spawn_blocking(move || client::ping_at(&path))
        .await
        .unwrap();

    assert!(result.is_ok());

    server_handle.abort();
    let _ = std::fs::remove_file(&socket_path);
}

#[tokio::test]
async fn test_client_open_seat() {
    let socket_path = test_socket_path();

    let mut server = SeatServer::new_with_path(&socket_path).unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let path = socket_path.clone();
    let result = tokio::task::spawn_blocking(move || client::open_seat_at(&path))
        .await
        .unwrap();

    assert!(result.is_ok());
    assert!(result.unwrap() > 0);

    server_handle.abort();
    let _ = std::fs::remove_file(&socket_path);
}

#[tokio::test]
async fn test_client_close_seat() {
    let socket_path = test_socket_path();

    let mut server = SeatServer::new_with_path(&socket_path).unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let path = socket_path.clone();
    let result = tokio::task::spawn_blocking(move || client::close_seat_at(&path))
        .await
        .unwrap();

    assert!(result.is_err());

    server_handle.abort();
    let _ = std::fs::remove_file(&socket_path);
}

#[tokio::test]
async fn test_client_close_device_error() {
    let socket_path = test_socket_path();

    let mut server = SeatServer::new_with_path(&socket_path).unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let path = socket_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let _ = client::open_seat_at(&path);
        client::close_device_at(&path, 9999)
    })
    .await
    .unwrap();

    assert!(result.is_err());

    server_handle.abort();
    let _ = std::fs::remove_file(&socket_path);
}

#[tokio::test]
async fn test_client_open_device_no_seat() {
    let socket_path = test_socket_path();

    let mut server = SeatServer::new_with_path(&socket_path).unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let path = socket_path.clone();
    let result =
        tokio::task::spawn_blocking(move || client::open_device_at(&path, Path::new("/dev/tty")))
            .await
            .unwrap();

    assert!(result.is_err());

    server_handle.abort();
    let _ = std::fs::remove_file(&socket_path);
}
