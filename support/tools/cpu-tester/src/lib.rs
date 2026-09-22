#![forbid(unsafe_code)]

//! `test-runner` を CLI から実行するための薄い tool。

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use test_runner::{MachineStatus, TestRunnerMachine};

pub fn run_with_args<I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(input) = args.next() else {
        return Err(usage("cpu-tester"));
    };

    let mut max_steps = 100_000usize;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-steps" => {
                let Some(value) = args.next() else {
                    return Err(usage("cpu-tester"));
                };
                max_steps = value
                    .parse()
                    .map_err(|_| format!("invalid step count `{value}`"))?;
            }
            _ => return Err(usage("cpu-tester")),
        }
    }

    let words = load_program(&input)?;
    let mut machine = TestRunnerMachine::new();
    machine
        .load_words(&words)
        .map_err(|_| "failed to load program into RAM".to_string())?;

    match machine.run_steps(max_steps) {
        Ok(MachineStatus::Passed) => {
            flush_debug_output(&mut machine)?;
            Ok(())
        }
        Ok(MachineStatus::Failed { test_id, detail }) => {
            flush_debug_output(&mut machine)?;
            Err(format!(
                "program reported FAIL: test_id={} detail={}",
                test_id, detail
            ))
        }
        Ok(MachineStatus::Running) => {
            flush_debug_output(&mut machine)?;
            Err(format!("step limit exceeded after {max_steps} step(s)"))
        }
        Err(fault) => {
            flush_debug_output(&mut machine)?;
            Err(format!(
                "runtime fault: {:?} (code: {:?})",
                fault,
                fault.code()
            ))
        }
    }
}

pub fn usage(bin_name: &str) -> String {
    [
        "Usage:",
        &format!("  {bin_name} <input.asm|input.bin> [--max-steps N]"),
        &format!("  {bin_name} help"),
    ]
    .join("\n")
}

fn load_program(input: &str) -> Result<Vec<u32>, String> {
    let path = Path::new(input);
    let is_binary = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("bin"));

    if is_binary {
        let bytes = fs::read(input).map_err(|err| format!("failed to read `{input}`: {err}"))?;
        if bytes.len() % 4 != 0 {
            return Err(format!(
                "binary input `{input}` has invalid length {}; expected a multiple of 4 bytes",
                bytes.len()
            ));
        }

        let words = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        return Ok(words);
    }

    let source =
        fs::read_to_string(input).map_err(|err| format!("failed to read `{input}`: {err}"))?;
    let program = kagura_assembly::assemble(&source)
        .map_err(|err| format!("assembly failed for `{input}`: {err}"))?;
    Ok(program.words().to_vec())
}

fn flush_debug_output(machine: &mut TestRunnerMachine) -> Result<(), String> {
    let output = machine.take_debug_output();
    if !output.is_empty() {
        let rendered = render_debug_output(&output);
        io::stdout()
            .write_all(rendered.as_bytes())
            .map_err(|err| format!("failed to write stdout: {err}"))?;
        io::stdout()
            .flush()
            .map_err(|err| format!("failed to flush stdout: {err}"))?;
    }
    Ok(())
}

fn render_debug_output(output: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(output) {
        return text.to_string();
    }

    let hex = output
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("DebugIo output (hex): {hex}\n")
}
