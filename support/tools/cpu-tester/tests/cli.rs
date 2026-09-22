use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use kagura_assembly::assemble;

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("cpu-tester-cli-test-{nanos}"))
}

#[test]
fn cli_runs_program_and_streams_debug_output() {
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).unwrap();

    let input = temp_dir.join("run.asm");
    fs::write(
        &input,
        "\
LI r1, 0x2000
LI r2, 79
STB r2, r1, r0, 0
LI r2, 75
STB r2, r1, r0, 0
LI r3, 0x2010
LI r4, 1
STW r4, r3, r0, 0
",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cpu-tester"))
        .arg(&input)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, b"OK");
}

#[test]
fn cli_runs_binary_output_program_and_falls_back_to_hex() {
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).unwrap();

    let input = temp_dir.join("binary.asm");
    fs::write(
        &input,
        "\
LI r1, 0x2000
LI r2, 0
STB r2, r1, r0, 0
LI r2, 255
STB r2, r1, r0, 0
LI r3, 0x2010
LI r4, 1
STW r4, r3, r0, 0
",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cpu-tester"))
        .arg(&input)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, b"DebugIo output (hex): 00 FF\n");
}

#[test]
fn cli_runs_prebuilt_binary_input() {
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).unwrap();

    let asm = temp_dir.join("prebuilt.asm");
    let input = temp_dir.join("prebuilt.bin");
    fs::write(
        &asm,
        "\
LI r1, 0x2000
LI r2, 79
STB r2, r1, r0, 0
LI r2, 75
STB r2, r1, r0, 0
LI r3, 0x2010
LI r4, 1
STW r4, r3, r0, 0
",
    )
    .unwrap();

    let source = fs::read_to_string(&asm).unwrap();
    let bytes = assemble(&source).unwrap().to_le_bytes();
    fs::write(&input, bytes).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cpu-tester"))
        .arg(&input)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, b"OK");
}

#[test]
fn cli_help_exits_successfully() {
    let output = Command::new(env!("CARGO_BIN_EXE_cpu-tester"))
        .arg("help")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
}
