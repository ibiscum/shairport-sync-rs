use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn pick_free_port() -> u16 {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
    let port = listener
        .local_addr()
        .expect("read local addr for ephemeral port")
        .port();
    drop(listener);
    port
}

fn wait_for_tcp_listen(port: u16, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("server did not listen on 127.0.0.1:{port} before timeout");
}

fn wait_for_exit(child: &mut std::process::Child, timeout: Duration) -> std::process::ExitStatus {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(status) = child.try_wait().expect("poll child exit status") {
            return status;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("child process did not exit before timeout");
}

#[test]
fn m0_runtime_starts_in_ap1_and_stops_cleanly_on_sigint() {
    let exe = env!("CARGO_BIN_EXE_shairport-sync-rs");
    let port = pick_free_port();

    let mut child = Command::new(exe)
        .args([
            "--name",
            "SSR M0 Integration",
            "--port",
            &port.to_string(),
            "--backend",
            "null",
            "--airplay-mode",
            "ap1",
            "--ap1-codecs",
            "pcm,alac",
            "--ap1-encryption",
            "none",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn shairport-sync-rs test instance");

    wait_for_tcp_listen(port, Duration::from_secs(15));

    let status = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("send SIGINT to test instance");
    assert!(status.success(), "kill command failed with status: {status}");

    let exit_status = wait_for_exit(&mut child, Duration::from_secs(10));
    assert!(
        exit_status.success(),
        "server did not exit cleanly after SIGINT: {exit_status}"
    );
}
