use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sandbox(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("tachyon-cli-{}-{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("sandbox");
    root
}

fn run(root: &Path, args: &[&str]) -> Output {
    run_env(root, args, &[])
}

fn run_env(root: &Path, args: &[&str], extra: &[(&str, String)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tachyon"));
    command
        .args(args)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("TACHYON_FETCH_CONFIG")
        .env_remove("TACHYON_FETCH_PALETTES")
        .env_remove("TACHYON_FETCH_QUOTES")
        .env_remove("TACHYON_FETCH_STATE")
        .env_remove("TACHYON_FETCH_LOGO")
        .env_remove("TACHYON_FETCH_ALBUMS");
    for (key, value) in extra {
        command.env(key, value);
    }
    command.output().expect("run tachyon")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn version_matches_the_crate() {
    let root = sandbox("version");
    let output = run(&root, &["--version"]);
    assert!(output.status.success());
    assert_eq!(
        stdout(&output).trim(),
        format!("tachyon {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_lists_usage() {
    let root = sandbox("help");
    let output = run(&root, &["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("usage: tachyon [options]"));
}

#[test]
fn unknown_arguments_fail_loudly() {
    let root = sandbox("unknown");
    let output = run(&root, &["--bogus"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unknown argument"));
}

#[test]
fn missing_values_fail_loudly() {
    let root = sandbox("missing");
    let output = run(&root, &["--theme"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("missing value for --theme"));
}

#[test]
fn invalid_seed_fails_loudly() {
    let root = sandbox("seed");
    let output = run(&root, &["--seed", "nope"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("invalid seed"));
}

#[test]
fn demo_render_is_deterministic_with_seed() {
    let root = sandbox("demo");
    let first = stdout(&run(&root, &["--demo", "--plain", "--seed", "42"]));
    let second = stdout(&run(&root, &["--demo", "--plain", "--seed", "42"]));
    assert_eq!(first, second);
    assert!(first.contains("TACHYON //"));
    assert!(first.contains("Memory"));
    assert!(first.contains("RTX 4060"));
}

#[test]
fn non_tty_render_contains_no_escape_codes() {
    let root = sandbox("plainpipe");
    let output = stdout(&run(&root, &["--demo", "--seed", "3"]));
    assert!(!output.contains('\x1b'));
}

#[test]
fn config_enabled_list_filters_modules() {
    let root = sandbox("filter");
    let config_dir = root.join("config/tachyon");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(config_dir.join("fetch.conf"), "enabled = memory,swap\n").expect("config");
    let output = stdout(&run(&root, &["--demo", "--plain", "--seed", "1"]));
    assert!(output.contains("Memory"));
    assert!(output.contains("Swap"));
    assert!(!output.contains("Kernel"));
    assert!(!output.contains("Battery"));
}

#[test]
fn config_env_override_is_honoured() {
    let root = sandbox("env-config");
    let path = root.join("elsewhere.conf");
    fs::write(&path, "enabled = uptime\n").expect("config");
    let override_path = ("TACHYON_FETCH_CONFIG", path.display().to_string());
    let output = stdout(&run_env(
        &root,
        &["--demo", "--plain", "--seed", "1"],
        &[override_path],
    ));
    assert!(output.contains("Uptime"));
    assert!(!output.contains("Kernel"));
}

#[test]
fn list_themes_reads_palette_table() {
    let root = sandbox("themes");
    let table = root.join("palettes.tsv");
    fs::write(
        &table,
        "Zulu\tdark\t#101315\t#cacccc\t#4b4e55\t#798186\t#707070\t#9fa5a9\t#d9dbdc\t#de6145\thud\treveal\n\
         Alpha\tlight\t#fdfcf8\t#23201c\t#8a857c\t#a8692b\t#5f7a8a\t#6f8e5f\t#b58b3a\t#b0524a\tstack\toff\n",
    )
    .expect("table");
    let output = stdout(&run(
        &root,
        &["--list-themes", "--palettes", table.to_str().unwrap()],
    ));
    let mut names: Vec<&str> = output.lines().collect();
    names.sort();
    let mut expected = vec![
        "Alpha",
        "Flexoki Light",
        "Hackerman",
        "Retro 82",
        "Rose Pine",
        "Solitude",
        "Vantablack",
        "Zulu",
    ];
    expected.sort();
    assert_eq!(names, expected);
}

#[test]
fn font_listing_succeeds_without_fontconfig() {
    let root = sandbox("fonts");
    let output = run(&root, &["--fonts"]);
    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
}

fn fake_album(root: &Path) {
    let dir = root.join("albums/night");
    fs::create_dir_all(&dir).expect("album dir");
    for name in ["a.png", "b.png"] {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes.extend_from_slice(&[0, 0, 0, 13]);
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&8u32.to_be_bytes());
        bytes.extend_from_slice(&8u32.to_be_bytes());
        fs::write(dir.join(name), bytes).expect("png");
    }
}

#[test]
fn rotation_advances_across_runs() {
    let root = sandbox("rotate");
    fake_album(&root);
    let config = root.join("config/tachyon/fetch.conf");
    fs::create_dir_all(config.parent().unwrap()).expect("config dir");
    fs::write(
        &config,
        "album = night\n\
         [rotate]\n\
         art = next\n\
         palette = next\n\
         every = run\n\
         themes = Solitude,Hackerman\n",
    )
    .expect("config");
    let albums = (
        "TACHYON_FETCH_ALBUMS",
        root.join("albums").display().to_string(),
    );
    let state = root.join("state/tachyon/rotation");
    let mut seen = Vec::new();
    for _ in 0..3 {
        let output = run_env(
            &root,
            &["--demo", "--plain", "--static", "--no-logo"],
            std::slice::from_ref(&albums),
        );
        assert!(output.status.success());
        let text = stdout(&output);
        seen.push(if text.contains("SOLITUDE") { 0 } else { 1 });
        assert!(text.contains("SOLITUDE") || text.contains("HACKERMAN"));
    }
    assert_eq!(seen, vec![0, 1, 0]);
    let state_text = fs::read_to_string(&state).expect("rotation state");
    assert!(state_text.contains("art = 0"), "{}", state_text);
    assert!(state_text.contains("stamp = "), "{}", state_text);
}

#[test]
fn convert_missing_source_fails() {
    let root = sandbox("convert");
    let destination = root.join("out.png");
    let output = run(
        &root,
        &[
            "--convert",
            "/nonexistent/input.png",
            destination.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("convert failed"));
}
