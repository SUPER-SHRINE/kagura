use super::*;

fn cpu_case(initial: &str, execution: &str, expected: &str) -> CpuCase {
    toml::from_str(&format!(
        r#"
format = "kagura-conformance-case"
version = 1
id = "RUNNER-REGRESSION"
level = 3
binary = "program.bin"
spec = ["cpu.md#9-fault処理"]
[image]
load_address = 0
{initial}
[execution]
{execution}
{expected}
"#
    ))
    .unwrap()
}

fn run_cpu(case: &CpuCase, words: &[u32]) -> (ReferenceMachine, ActualTermination, Vec<Mismatch>) {
    validate_cpu_case(case).unwrap();
    let binary: Vec<_> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    let mut machine = ReferenceMachine::new().unwrap();
    machine.setup_cpu(case, &binary).unwrap();
    let actual = machine.execute(case).unwrap();
    let mismatches = machine.compare_cpu(case, &actual).unwrap();
    (machine, actual, mismatches)
}

#[test]
fn omitted_bus_trace_allows_fetch_at_every_level() {
    for level in 1..=3 {
        let mut case = cpu_case(
            "",
            "until = 'steps'\nsteps = 1\nmax_steps = 1",
            "[expected.termination]\nkind = 'steps'\nretired = 1",
        );
        case.level = level;
        let (machine, _, mismatches) = run_cpu(&case, &[0]);
        assert_eq!(machine.machine.trace().len(), 1);
        assert!(mismatches.is_empty(), "{mismatches:?}");
    }
}

#[test]
fn explicit_empty_bus_trace_rejects_fetch() {
    let case = cpu_case(
        "",
        "until = 'steps'\nsteps = 1\nmax_steps = 1",
        "[expected]\nbus = []\n[expected.termination]\nkind = 'steps'",
    );
    let (_, _, mismatches) = run_cpu(&case, &[0]);
    assert_eq!(mismatches.len(), 1, "{mismatches:?}");
    assert_eq!(mismatches[0].path, "bus.length");
    assert_eq!(mismatches[0].expected, json!(0));
    assert_eq!(mismatches[0].actual, json!(1));
}

#[test]
fn explicit_empty_bus_trace_accepts_fault_before_fetch() {
    let case = cpu_case(
        "[initial]\npc = 2",
        "until = 'fault'\nmax_steps = 1",
        "[expected]\nbus = []\n[expected.termination]\nkind = 'fault'\nfault = 'UNALIGNED_PC'\nfaulting_pc = 2\naddress = 2\nretired = 0",
    );
    let (machine, _, mismatches) = run_cpu(&case, &[0]);
    assert!(machine.machine.trace().is_empty());
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

fn reporter_initial(status: u32, reserved: u32) -> String {
    format!(
        "[[initial.devices]]\nid = 'reporter'\nstate = {{ status = {status}, case_id = 7, detail = 9, reserved = {reserved} }}"
    )
}

#[test]
fn initial_reporter_status_does_not_terminate_unrelated_instructions() {
    let case = cpu_case(
        &reporter_initial(1, 0),
        "until = 'steps'\nsteps = 2\nmax_steps = 2",
        "[expected.termination]\nkind = 'steps'\nretired = 2\n[expected.state]\npc = 8",
    );
    let (_, _, mismatches) = run_cpu(&case, &[0, 0]);
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

#[test]
fn successful_status_writes_terminate_even_when_value_is_unchanged() {
    for (initial_status, status, expected_status) in
        [(0, 1, "pass"), (1, 1, "pass"), (2, 2, "fail")]
    {
        let initial = format!(
            "[initial.registers]\nr1 = 0xFFFFF000\nr2 = {status}\nr3 = 11\n{}",
            reporter_initial(initial_status, 0)
        );
        let expected = format!(
            "[expected.termination]\nkind = 'reporter'\nstatus = '{expected_status}'\nretired = 2\ncase_id = 11\ndetail = 9\n[expected.state]\npc = 8\n[[expected.state.memory]]\naddress = 0x100\nbytes = [0]"
        );
        let case = cpu_case(&initial, "until = 'reporter'\nmax_steps = 3", &expected);
        // CASE_ID write, STATUS write, then a RAM write that must not execute.
        let (_, _, mismatches) = run_cpu(&case, &[0xB310_0004, 0xB210_0000, 0x8200_0100]);
        assert!(mismatches.is_empty(), "{mismatches:?}");
    }
}

#[test]
fn invalid_status_writes_stop_after_commit_and_fail_the_case() {
    for status in [0, 3, u32::MAX] {
        let initial = format!(
            "[initial.registers]\nr1 = 0xFFFFF000\nr2 = {status}\nr3 = 0xAA\n{}",
            reporter_initial(0, 0)
        );
        let case = cpu_case(
            &initial,
            "until = 'reporter'\nmax_steps = 2",
            "[expected.termination]\nkind = 'reporter'\nstatus = 'pass'\nretired = 1\ncase_id = 7\ndetail = 9\n[expected.state]\npc = 4\n[[expected.state.memory]]\naddress = 0x100\nbytes = [0]",
        );
        // STW to STATUS, then STB to RAM that must not execute.
        let (machine, actual, mismatches) = run_cpu(&case, &[0xB210_0000, 0x8300_0100]);
        assert_eq!(actual.kind, "reporter");
        assert_eq!(actual.retired, 1);
        assert_eq!(machine.machine.reporter().status, status);
        assert_eq!(machine.machine.pc(), 4);
        assert_eq!(machine.machine.read_memory(0x100, 1).unwrap(), [0]);
        assert_eq!(machine.machine.trace().len(), 2);
        assert_eq!(mismatches.len(), 1, "{mismatches:?}");
        assert_eq!(mismatches[0].path, "reporter.status");
        assert_eq!(mismatches[0].actual, json!("invalid"));
    }
}

#[test]
fn invalid_report_fails_even_without_an_expected_status() {
    let case = cpu_case(
        "[initial.registers]\nr1 = 0xFFFFF000\nr2 = 3",
        "until = 'reporter'\nmax_steps = 1",
        "[expected.termination]\nkind = 'reporter'\nretired = 1",
    );
    let (_, _, mismatches) = run_cpu(&case, &[0xB210_0000]);
    assert!(
        mismatches
            .iter()
            .any(|mismatch| mismatch.path == "reporter.status")
    );
}

#[test]
fn faulted_reporter_writes_do_not_terminate() {
    for (word, address, width) in [
        (0x8210_0000, 0xFFFF_F000u32, 8),
        (0x9210_0000, 0xFFFF_F000u32, 16),
        (0xB210_000C, 0xFFFF_F00Cu32, 32),
    ] {
        let initial = format!(
            "[initial.registers]\nr1 = 0xFFFFF000\nr2 = 1\n{}",
            reporter_initial(1, 0)
        );
        let expected = format!(
            "[expected.termination]\nkind = 'fault'\nfault = 'BUS_FAULT'\nfaulting_pc = 0\naddress = {address}\nwidth = {width}\noperation = 'store'\nretired = 0\n[expected.state]\npc = 0"
        );
        let case = cpu_case(&initial, "until = 'fault'\nmax_steps = 1", &expected);
        let (machine, _, mismatches) = run_cpu(&case, &[word]);
        assert_eq!(machine.machine.reporter().status, 1);
        assert!(mismatches.is_empty(), "{mismatches:?}");
    }
}

fn bus_case(initial: &str, expected: &str) -> Result<BusCase, toml::de::Error> {
    toml::from_str(&format!(
        r#"
format = "kagura-bus-conformance-case"
version = 1
id = "BUS-RUNNER-REGRESSION"
spec = ["bus.md#アクセス"]
{initial}
[[transactions]]
operation = "read"
address = 0
width = 8
[[expected.transactions]]
result = "ok"
value = 0
{expected}
"#
    ))
}

#[test]
fn bus_device_states_require_complete_known_unique_fields() {
    let partial = "id = 'reporter'\nstate = { status = 0 }";
    let initial = bus_case(&format!("[[initial.devices]]\n{partial}"), "").unwrap();
    assert!(
        validate_bus_case(&initial)
            .unwrap_err()
            .contains("invalid state fields")
    );
    let expected = bus_case("", &format!("[[expected.devices]]\n{partial}")).unwrap();
    assert!(
        validate_bus_case(&expected)
            .unwrap_err()
            .contains("invalid state fields")
    );
    let unknown = bus_case(
        "[[initial.devices]]\nid = 'high-rom'\nstate = { word = 0, unknown = 1 }",
        "",
    )
    .unwrap();
    assert!(validate_bus_case(&unknown).is_err());
    let duplicate = bus_case("[[initial.devices]]\nid = 'high-rom'\nstate = { word = 0 }\n[[initial.devices]]\nid = 'high-rom'\nstate = { word = 0 }", "").unwrap();
    assert!(
        validate_bus_case(&duplicate)
            .unwrap_err()
            .contains("duplicate device")
    );
    assert!(
        bus_case(
            "[[initial.devices]]\nid = 'high-rom'\nstate = { word = 0, word = 1 }",
            ""
        )
        .is_err()
    );
}

#[test]
fn host_setup_accepts_nonzero_reporter_reserved_and_snapshot_matches() {
    let state =
        "id = 'reporter'\nstate = { status = 3, case_id = 7, detail = 9, reserved = 0xFFFFFFFF }";
    let case = bus_case(
        &format!("[[initial.devices]]\n{state}"),
        &format!("[[expected.devices]]\n{state}"),
    )
    .unwrap();
    validate_bus_case(&case).unwrap();
    let mut machine = ReferenceMachine::new().unwrap();
    machine.setup_common(&case.initial).unwrap();
    let actual = machine.execute_transactions(&case.transactions).unwrap();
    assert!(machine.compare_bus(&case, &actual).unwrap().is_empty());
    assert_eq!(machine.machine.reporter().reserved, u32::MAX);
    assert!(machine.machine.trace().is_empty());

    let cpu = cpu_case(
        &reporter_initial(0, u32::MAX),
        "until = 'steps'\nsteps = 1\nmax_steps = 1",
        &format!(
            "[expected.termination]\nkind = 'steps'\n[[expected.devices]]\n{}",
            state.replace("status = 3", "status = 0")
        ),
    );
    assert!(run_cpu(&cpu, &[0]).2.is_empty());
}
