use serde_json::Value;
use std::process::Command;

#[test]
#[ignore = "requires avahi-browse, ss, timeout, and full Linux networking"]
fn m0_smoke_json_reports_discovery_and_ap1_advertisement_checks() {
    let output = Command::new("bash")
        .arg("scripts/debian-smoke-test-json.sh")
        .env("BUILD_FIRST", "0")
        .env("AIRPLAY_MODE", "ap1")
        .env("AP1_CODECS", "pcm,alac")
        .env("AP1_ENCRYPTION", "none")
        .env("AP1_EXPECT_CN", "0,1")
        .env("AP1_EXPECT_ET", "0")
        .output()
        .expect("run debian-smoke-test-json.sh");

    assert!(
        output.status.success(),
        "smoke script exited non-zero: status={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("smoke script output must be UTF-8");
    let json: Value = serde_json::from_str(stdout.trim()).expect("smoke script must return JSON");

    assert_eq!(json["status"], "passed", "smoke JSON status should be passed");
    assert_eq!(json["checks"]["started"], true, "server should start");
    assert_eq!(
        json["checks"]["port_listening"],
        true,
        "port should be listening"
    );
    assert_eq!(json["checks"]["mdns_seen"], true, "mDNS should be discovered");
    assert_eq!(
        json["checks"]["mdns_name_match"],
        true,
        "service name should match"
    );
    assert_eq!(
        json["checks"]["mdns_port_match"],
        true,
        "service port should match"
    );
    assert_eq!(
        json["checks"]["ap1_cn_match"],
        true,
        "AP1 cn advertisement should match"
    );
    assert_eq!(
        json["checks"]["ap1_et_match"],
        true,
        "AP1 et advertisement should match"
    );
}
