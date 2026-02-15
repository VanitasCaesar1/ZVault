//! Prometheus metrics endpoint: `/v1/sys/metrics`
//!
//! Exposes vault health and operational metrics in Prometheus text format.
//! No authentication required — designed for Prometheus scraping.

use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;

use crate::state::AppState;

/// Build the `/v1/sys/metrics` router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(prometheus_metrics))
}

/// `GET /v1/sys/metrics` — Prometheus text format metrics.
///
/// Exposes:
/// - `zvault_sealed` (gauge): 1 if sealed, 0 if unsealed
/// - `zvault_initialized` (gauge): 1 if initialized
/// - `zvault_lease_count` (gauge): total active leases
/// - `zvault_lease_expired_count` (gauge): expired leases pending cleanup
/// - `zvault_mount_count` (gauge): number of mounted engines
/// - `zvault_info` (gauge): build info label
async fn prometheus_metrics(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut lines = Vec::with_capacity(32);

    // Seal status.
    let (initialized, sealed) = match state.seal_manager.status().await {
        Ok(s) => (s.initialized, s.sealed),
        Err(_) => (false, true),
    };

    lines.push("# HELP zvault_initialized Whether the vault has been initialized.".to_owned());
    lines.push("# TYPE zvault_initialized gauge".to_owned());
    lines.push(format!("zvault_initialized {}", u8::from(initialized)));

    lines.push("# HELP zvault_sealed Whether the vault is currently sealed.".to_owned());
    lines.push("# TYPE zvault_sealed gauge".to_owned());
    lines.push(format!("zvault_sealed {}", u8::from(sealed)));

    // Lease counts (only if unsealed).
    if !sealed {
        let (total, expired) = match state.lease_manager.list_all().await {
            Ok(leases) => {
                let expired = leases.iter().filter(|l| l.is_expired()).count();
                (leases.len(), expired)
            }
            Err(_) => (0, 0),
        };

        lines.push("# HELP zvault_lease_count Total number of active leases.".to_owned());
        lines.push("# TYPE zvault_lease_count gauge".to_owned());
        lines.push(format!("zvault_lease_count {total}"));

        lines.push("# HELP zvault_lease_expired_count Number of expired leases pending cleanup.".to_owned());
        lines.push("# TYPE zvault_lease_expired_count gauge".to_owned());
        lines.push(format!("zvault_lease_expired_count {expired}"));

        // Mount count.
        let mount_count = state.mount_manager.list().await.len();

        lines.push("# HELP zvault_mount_count Number of mounted secret engines.".to_owned());
        lines.push("# TYPE zvault_mount_count gauge".to_owned());
        lines.push(format!("zvault_mount_count {mount_count}"));
    }

    // Build info.
    lines.push("# HELP zvault_info ZVault build information.".to_owned());
    lines.push("# TYPE zvault_info gauge".to_owned());
    lines.push(format!(
        "zvault_info{{version=\"{}\"}} 1",
        env!("CARGO_PKG_VERSION")
    ));

    let body = lines.join("\n") + "\n";

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}
