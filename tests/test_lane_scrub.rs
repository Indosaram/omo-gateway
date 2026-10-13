use omon_gateway::lanes::scrub::{scrub, scrub_with_hosts};

#[test]
fn test_lane_scrub_gh_token_fake_36_chars() {
    let token = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
    let message = format!("[REPORT] qa-1 | milestone | published with token {token} to remote");
    let scrubbed = scrub(&message);
    assert!(!scrubbed.contains(token));
    assert!(scrubbed.contains("<redacted>"));
    assert_eq!(
        scrubbed,
        "[REPORT] qa-1 | milestone | published with token <redacted> to remote"
    );
}

#[test]
fn test_lane_scrub_home_paths_and_korean() {
    let input = "진행 상황: /Users/indo/code/project/main.rs 파일 수정 완료";
    assert_eq!(
        scrub(input),
        "진행 상황: ~/code/project/main.rs 파일 수정 완료"
    );

    let input2 = "홈 디렉터리는 /Users/indo 입니다.";
    assert_eq!(scrub(input2), "홈 디렉터리는 ~ 입니다.");

    let input3 = "작업 경로: /home/runner/work/repo";
    assert_eq!(scrub(input3), "작업 경로: ~/work/repo");
}

#[test]
fn test_lane_scrub_hosts() {
    let hosts = ["secret.internal", "db.corp.local"];
    let input = "Connecting to https://secret.internal:443/api and db.corp.local";
    let scrubbed = scrub_with_hosts(input, &hosts);
    assert_eq!(
        scrubbed,
        "Connecting to https://<redacted>:443/api and <redacted>"
    );
}

#[test]
fn test_lane_scrub_credentials() {
    let input = "Authorization: Bearer secret-token-xyz";
    assert_eq!(scrub(input), "Authorization: <redacted>");

    let url = "https://user:password@secret.internal/v1";
    assert_eq!(scrub(url), "https://<redacted>@secret.internal/v1");
}

#[test]
fn test_lane_scrub_benign() {
    let benign = "cargo test --lib lanes::scrub";
    assert_eq!(scrub(benign), benign);
}

#[test]
fn test_lane_scrub_literal_home() {
    assert_eq!(scrub("$HOME/src/main.rs"), "~/src/main.rs");
    assert_eq!(scrub("${HOME}/src/main.rs"), "~/src/main.rs");
    assert_eq!(scrub("Path $HOME"), "Path ~");
    assert_eq!(scrub("Path ${HOME}"), "Path ~");
    assert_eq!(
        scrub("작업 디렉터리: $HOME/code 입니다"),
        "작업 디렉터리: ~/code 입니다"
    );
}
