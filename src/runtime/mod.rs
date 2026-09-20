pub mod pairing_store;

use shairplay::RaopServer;
use tracing::info;

pub async fn run_until_shutdown(server: &mut RaopServer) -> Result<(), String> {
    info!("server is running; press Ctrl+C to stop");
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| format!("failed to listen for Ctrl+C: {e}"))?;
    info!("shutdown signal received");
    server.stop().await;
    Ok(())
}
