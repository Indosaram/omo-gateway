use omon_gateway::security::{
    detect_dangerous_command, detect_hardline_command, redact_approval_display,
};

#[test]
fn hardline_rm_protected_path_after_other_operands() {
    for command in [
        "rm -rf build ~",
        "rm -rf ./x /etc",
        "rm -rf tmp /usr/*",
        "rm -rf build ./tmp /",
        "sudo rm -rf a b $HOME",
    ] {
        assert!(
            detect_hardline_command(command).is_some(),
            "protected path after other operands must be hardline: {command}"
        );
    }
    for command in ["rm -rf build ./tmp", "rm -rf build /tmp/x ~/proj"] {
        assert!(
            detect_hardline_command(command).is_none(),
            "non-protected operands must not be hardline: {command}"
        );
    }
}

#[test]
fn dangerous_git_branch_long_flags_with_n_in_branch_name() {
    for command in [
        "git branch --delete main --force",
        "git branch --force --delete main",
    ] {
        assert!(
            detect_dangerous_command(command).is_some(),
            "long-flag force delete must be detected: {command}"
        );
    }
    assert!(detect_dangerous_command("git branch --delete main\necho --force").is_none());
}

#[test]
fn approval_display_redacts_any_authorization_scheme() {
    for input in [
        "curl -H 'Authorization: token ghp_sentinelvalue' https://api.github.com",
        "Authorization: Digest sentinelvalue",
        "Authorization: ghp_sentinelvalue trailing",
        r#"requests.get(u, headers={"Authorization": "Bearer sentinelvalue"})"#,
        r#"curl -d '{"Authorization":"Bearer sentinelvalue"}' https://x"#,
        r#"Authorization: Bearer "sentinelvalue""#,
        r#"Authorization: "Bearer sentinelvalue""#,
    ] {
        let rendered = redact_approval_display(input).unwrap();
        assert!(!rendered.contains("sentinelvalue"), "{rendered}");
        assert!(rendered.contains("[REDACTED]"), "{rendered}");
    }
}

#[test]
fn variant_budget_exhaustion_is_a_finding() {
    // Passes every parser length limit but exhausts the variant budget.
    let command = "'true';".repeat(16_000);
    assert!(
        detect_hardline_command(&command).is_some(),
        "variant budget exhaustion must not silently pass hardline detection"
    );
    assert!(
        detect_dangerous_command(&command).is_some(),
        "variant budget exhaustion must not silently pass dangerous detection"
    );
}

#[test]
fn apostrophe_in_echo_comment_is_not_hardline() {
    for command in ["echo hi # don't", "printf 'done\\n' # can't fail"] {
        assert!(
            detect_hardline_command(command).is_none(),
            "apostrophe prose in an echo/printf comment must not be hardline: {command}"
        );
    }
    assert!(
        detect_hardline_command("echo \"it's\"; rm -rf /").is_some(),
        "a real hardline command after echo must still be caught"
    );
}

#[test]
fn hardline_catches_path_qualified_and_wrapped_programs() {
    for command in [
        "/bin/rm -rf /etc",
        "/bin/rm -rf /",
        "sudo -u root rm -rf /",
        "env -i rm -rf ~",
        "/sbin/reboot",
        "/sbin/shutdown -h now",
        "reboot>/dev/null",
        "halt</dev/null",
    ] {
        assert!(
            detect_hardline_command(command).is_some(),
            "path-qualified or wrapped hardline command must be blocked: {command}"
        );
    }
    assert!(detect_hardline_command("/bin/rm -rf target").is_none());
    for command in ["env -u HOME rm -rf /", "env -C /tmp rm -rf /etc"] {
        assert!(
            detect_hardline_command(command).is_some(),
            "env option with an argument must not hide rm: {command}"
        );
    }
    for command in [
        "./shutdown.sh",
        "scripts/reboot-test",
        "/usr/local/bin/halt.sh",
    ] {
        assert!(
            detect_hardline_command(command).is_none(),
            "a script merely named after shutdown/reboot is not hardline: {command}"
        );
    }
}

#[test]
fn escaped_hash_does_not_hide_execution_flags() {
    assert!(
        detect_dangerous_command(r"rg \# --pre=./evil.sh file").is_some(),
        "an escaped # is a literal argument; --pre after it must still be detected"
    );
}
