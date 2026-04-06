mod common;

use common::{spawn_server_with_rate_limit_per_ip, test_client};
use reqwest::StatusCode;

// ───────────────────── Per-IP rate limiting (e2e) ─────────────────────

#[tokio::test]
async fn e2e_per_ip_rate_limit_isolates_x_forwarded_for() {
    let (base, max_requests) = spawn_server_with_rate_limit_per_ip(2, 60).await;
    let client = test_client();

    // IP-A exhausts its quota
    for i in 0..max_requests {
        let resp = client
            .get(format!("{base}/health"))
            .header("X-Forwarded-For", "10.0.0.1")
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "IP-A request {i} should be allowed"
        );
    }

    // IP-A is now rate-limited
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.0.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "IP-A should be rate-limited after exhausting quota"
    );

    // IP-B still has its own independent quota
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.0.0.2")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "IP-B should still be allowed (independent bucket)"
    );
    assert!(
        resp.headers().contains_key("x-ratelimit-limit"),
        "IP-B response should have rate limit headers"
    );
}

#[tokio::test]
async fn e2e_per_ip_rate_limit_isolates_x_real_ip() {
    let (base, max_requests) = spawn_server_with_rate_limit_per_ip(2, 60).await;
    let client = test_client();

    // IP-C exhausts its quota via X-Real-IP
    for _ in 0..max_requests {
        let resp = client
            .get(format!("{base}/health"))
            .header("X-Real-IP", "192.168.1.100")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // IP-C is now limited
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Real-IP", "192.168.1.100")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "IP via X-Real-IP should be rate-limited after exhausting quota"
    );

    // IP-D via X-Real-IP is unaffected
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Real-IP", "192.168.1.200")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "different IP via X-Real-IP should still have quota"
    );
}

#[tokio::test]
async fn e2e_per_ip_rate_limit_x_forwarded_for_takes_precedence() {
    // When both X-Forwarded-For and X-Real-IP are present,
    // X-Forwarded-For should take precedence per extract_client_ip logic.
    let (base, max_requests) = spawn_server_with_rate_limit_per_ip(2, 60).await;
    let client = test_client();

    // Exhaust quota for IP identified by X-Forwarded-For: 10.1.1.1
    for _ in 0..max_requests {
        let resp = client
            .get(format!("{base}/health"))
            .header("X-Forwarded-For", "10.1.1.1")
            .header("X-Real-IP", "10.2.2.2")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Next request with same X-Forwarded-For but different X-Real-IP → still limited
    // because X-Forwarded-For takes precedence
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.1.1.1")
        .header("X-Real-IP", "10.9.9.9")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "X-Forwarded-For should take precedence over X-Real-IP"
    );

    // Request with different X-Forwarded-For → allowed (different bucket)
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.3.3.3")
        .header("X-Real-IP", "10.2.2.2")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "different X-Forwarded-For should have its own bucket"
    );
}

#[tokio::test]
async fn e2e_per_ip_rate_limit_remaining_tracks_per_ip() {
    let (base, _max) = spawn_server_with_rate_limit_per_ip(10, 60).await;
    let client = test_client();

    // IP-E sends one request
    let resp_e1 = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "172.16.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(resp_e1.status(), StatusCode::OK);
    let remaining_e1: u64 = resp_e1.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    // IP-E sends second request
    let resp_e2 = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "172.16.0.1")
        .send()
        .await
        .unwrap();
    let remaining_e2: u64 = resp_e2.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        remaining_e2 < remaining_e1,
        "remaining should decrement for same IP: {remaining_e1} -> {remaining_e2}"
    );

    // IP-F sends first request — should have full quota (independent of IP-E)
    let resp_f1 = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "172.16.0.2")
        .send()
        .await
        .unwrap();
    assert_eq!(resp_f1.status(), StatusCode::OK);
    let remaining_f1: u64 = resp_f1.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    // IP-F should have more remaining than IP-E (since IP-E used 2, IP-F used 1)
    assert!(
        remaining_f1 > remaining_e2,
        "new IP should have more remaining tokens: IP-F={remaining_f1}, IP-E={remaining_e2}"
    );
}

#[tokio::test]
async fn e2e_per_ip_rate_limit_multiple_ips_in_x_forwarded_for() {
    // X-Forwarded-For can contain multiple IPs separated by commas.
    // The middleware should use the first one (the original client IP).
    let (base, max_requests) = spawn_server_with_rate_limit_per_ip(2, 60).await;
    let client = test_client();

    // Exhaust quota for client IP 10.0.0.50 (first in chain)
    for _ in 0..max_requests {
        let resp = client
            .get(format!("{base}/health"))
            .header("X-Forwarded-For", "10.0.0.50, 10.0.0.99, 10.0.0.1")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Same first IP in a different proxy chain → still limited
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.0.0.50, 192.168.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "same first IP in X-Forwarded-For chain should share bucket"
    );

    // Different first IP → allowed
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.0.0.51, 10.0.0.99, 10.0.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "different first IP in chain should have its own bucket"
    );
}
