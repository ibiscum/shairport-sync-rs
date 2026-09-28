use serde_json::Value;
use std::process::Command;

#[test]
#[ignore = "requires avahi-browse, ss, timeout, and Linux networking"]
fn ap1_soak_harness_single_iteration_smoke() {
    let artifact_dir = format!(
        "/tmp/shairport-sync-rs-ap1-soak-test-{}",
        std::process::id()
    );

    let output = Command::new("bash")
        .arg("scripts/ap1-soak.sh")
        .env("ITERATIONS", "1")
        .env("HOLD_SECONDS", "1")
        .env("BUILD_FIRST", "0")
        .env("BASE_PORT", "5900")
        .env("ARTIFACT_DIR", &artifact_dir)
        .output()
        .expect("run scripts/ap1-soak.sh");

    assert!(
        output.status.success(),
        "soak harness failed: status={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let summary_path = format!("{artifact_dir}/summary.json");
    let summary_text = std::fs::read_to_string(&summary_path)
        .expect("soak harness should write summary.json artifact");
    let summary: Value =
        serde_json::from_str(&summary_text).expect("summary.json should be valid JSON");

    assert_eq!(summary["iterations"], 1, "expected one iteration");
    assert_eq!(summary["failed"], 0, "expected zero failed iterations");
}
