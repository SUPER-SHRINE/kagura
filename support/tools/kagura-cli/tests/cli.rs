use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("kagura-cli-test-{nanos}"))
}

fn conformance_root() -> PathBuf {
    env::var_os("KAGURA_CONFORMANCE_SUITE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../specs/kagura/v1/conformance")
        })
}

#[test]
fn assemble_builds_a_binary() {
    let temp_dir = unique_temp_dir();
    fs::create_dir_all(&temp_dir).unwrap();
    let input = temp_dir.join("sample.asm");
    let output = temp_dir.join("sample.bin");
    fs::write(&input, "LI r1, 42\nADD r2, r1, r0, 1\n").unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_kagura"))
        .arg("assemble")
        .arg(&input)
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        fs::read(output).unwrap(),
        vec![0x2A, 0, 0, 1, 1, 0, 0x10, 2]
    );
}

#[test]
fn help_lists_only_kagura_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_kagura"))
        .arg("help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("kagura assemble"));
    assert!(stdout.contains("kagura cpu-test"));
    assert!(!stdout.contains("compiler"));
    assert!(!stdout.contains("cart"));
}

#[test]
fn version_matches_package_version() {
    for argument in ["--version", "-V", "version"] {
        let output = Command::new(env!("CARGO_BIN_EXE_kagura"))
            .arg(argument)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("kagura {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn conformance_help_exits_successfully() {
    let output = Command::new(env!("CARGO_BIN_EXE_kagura"))
        .args(["conformance", "--help"])
        .output()
        .expect("kagura CLI should run");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("kagura conformance"));
}

#[test]
fn invalid_coverage_stops_before_case_execution_and_emits_json_errors() {
    let suite_root = conformance_root();
    let container = unique_temp_dir();
    let root = container.join("conformance");
    fs::create_dir_all(root.join("coverage")).unwrap();
    fs::create_dir_all(root.join("cases/cpu/add/basic")).unwrap();
    fs::create_dir_all(root.join("cases/bus")).unwrap();
    let source_case = suite_root.join("cases/cpu/add/basic");
    for file in ["case.toml", "program.bin", "program.words"] {
        fs::copy(
            source_case.join(file),
            root.join("cases/cpu/add/basic").join(file),
        )
        .unwrap();
    }
    fs::copy(
        suite_root.parent().unwrap().join("cpu.md"),
        root.parent().unwrap().join("cpu.md"),
    )
    .unwrap();
    fs::write(
        root.join("coverage/broken.toml"),
        "[[requirements]]\nid=\"CPU-A\"\nspec=\"cpu.md#3-arithmetic\"\nstatement=\"x\"\ncases=[\"MISSING\"]\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_kagura"))
        .args(["conformance", root.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let results = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(!results.is_empty());
    for result in &results {
        assert_eq!(result["suite"], "coverage");
        assert!(result["case"].is_null());
        assert!(result["level"].is_null());
        assert_eq!(result["result"], "error");
    }
    assert!(results.iter().all(|result| result["suite"] != "cpu"));
    assert!(results.iter().all(|result| result["suite"] != "bus"));
    let _ = fs::remove_dir_all(container);
}

#[test]
fn current_conformance_manifest_validates_before_running_cases() {
    let root = conformance_root();
    let output = Command::new(env!("CARGO_BIN_EXE_kagura"))
        .args(["conformance", root.to_str().unwrap(), "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 180);
    assert!(
        lines
            .iter()
            .all(|line| line.contains("\"result\":\"pass\""))
    );
}
