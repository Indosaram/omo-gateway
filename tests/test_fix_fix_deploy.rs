use std::fs;
use std::path::Path;

fn read(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .unwrap_or_else(|error| panic!("read {name}: {error}"))
}

fn stage(dockerfile: &str, name: &str) -> String {
    let marker = format!(" AS {name}");
    let mut found = false;
    let mut body = String::new();
    for line in dockerfile.lines() {
        if line.starts_with("FROM ") {
            found = line.ends_with(&marker);
            continue;
        }
        if found {
            body.push_str(line);
            body.push('\n');
        }
    }
    assert!(found || !body.is_empty(), "stage {name} missing");
    body
}

fn version_key(version: &str) -> Vec<u32> {
    version
        .split('.')
        .map(|part| part.parse().expect("numeric version part"))
        .collect()
}

fn package_rust_version(manifest: &str) -> String {
    manifest
        .lines()
        .take_while(|line| !line.starts_with("[lib]"))
        .find_map(|line| {
            line.strip_prefix("rust-version = \"")
                .and_then(|rest| rest.strip_suffix('"'))
        })
        .expect("[package] rust-version missing")
        .to_string()
}

#[test]
fn builder_copies_every_top_level_dir_the_manifest_paths_need() {
    let dockerfile = read("Dockerfile");
    let builder = stage(&dockerfile, "builder");
    let manifest = read("Cargo.toml");
    let mut dirs: Vec<String> = manifest
        .lines()
        .filter_map(|line| line.strip_prefix("path = \""))
        .filter_map(|rest| rest.strip_suffix('"'))
        .filter_map(|path| path.split('/').next().map(str::to_string))
        .collect();
    dirs.sort();
    dirs.dedup();
    assert!(
        dirs.contains(&"tests".to_string()),
        "manifest declares [[test]] paths"
    );
    for dir in dirs {
        let copied = builder.lines().any(|line| {
            line.starts_with("COPY ")
                && line.split_whitespace().any(|tok| tok == format!("./{dir}"))
        });
        assert!(copied, "builder stage must COPY {dir} ./{dir}");
    }
}

#[test]
fn builder_rust_image_satisfies_package_rust_version_and_locked_floor() {
    let dockerfile = read("Dockerfile");
    let first = dockerfile.lines().next().expect("Dockerfile not empty");
    let image_version = first
        .strip_prefix("FROM rust:")
        .and_then(|rest| rest.split('-').next())
        .expect("builder must be a rust: image");
    let declared = package_rust_version(&read("Cargo.toml"));
    // time 0.3.55 (locked) requires rust 1.88.
    assert!(version_key(&declared) >= version_key("1.88"));
    assert!(
        version_key(image_version) >= version_key(&declared),
        "image rust {image_version} < package rust-version {declared}"
    );
}

#[test]
fn builder_installs_native_build_deps_for_audiopus_sys() {
    let builder = stage(&read("Dockerfile"), "builder");
    assert!(builder.contains("apt-get install"));
    for package in ["cmake", "pkg-config", "libopus-dev"] {
        assert!(
            builder.split_whitespace().any(|tok| tok == package),
            "builder must install {package}"
        );
    }
}

#[test]
fn runtime_installs_fonts_for_table_rendering() {
    let runtime = stage(&read("Dockerfile"), "runtime");
    for package in ["fonts-dejavu-core", "fonts-noto-cjk"] {
        assert!(
            runtime.split_whitespace().any(|tok| tok == package),
            "runtime must install {package}"
        );
    }
}

#[test]
fn compose_uses_named_volumes_not_root_owned_bind_mounts() {
    let compose = read("docker-compose.yml");
    for line in compose.lines().map(str::trim) {
        assert!(
            !line.starts_with("- ./data") && !line.starts_with("- ./workspace"),
            "bind mount would be root-owned on Linux: {line}"
        );
    }
    assert!(compose.contains("- omon-data:/app/data"));
    assert!(compose.contains("- omon-workspace:/app/workspace"));
    assert!(compose.lines().any(|line| line == "volumes:"));
    assert!(compose.lines().any(|line| line == "  omon-data:"));
    assert!(compose.lines().any(|line| line == "  omon-workspace:"));
}
