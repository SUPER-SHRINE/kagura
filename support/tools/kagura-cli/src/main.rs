use std::process::ExitCode;

mod assembly;
mod coverage;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.message);
            ExitCode::from(error.code)
        }
    }
}

struct CliError {
    message: String,
    code: u8,
}

impl CliError {
    fn failure(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 1,
        }
    }

    fn suite(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 2,
        }
    }
}

fn run() -> Result<(), CliError> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        println!("{}", usage());
        return Ok(());
    };

    let rest = args.collect::<Vec<_>>();
    match command.as_str() {
        "assemble" | "assembler" => {
            if matches!(
                rest.first().map(String::as_str),
                Some("help" | "--help" | "-h")
            ) {
                println!("{}", assembly::usage("kagura assemble"));
                Ok(())
            } else {
                assembly::run_with_args(rest).map_err(CliError::failure)
            }
        }
        "cpu-test" | "cpu-tester" => {
            if matches!(
                rest.first().map(String::as_str),
                Some("help" | "--help" | "-h")
            ) {
                println!("{}", cpu_tester::usage("kagura cpu-test"));
                Ok(())
            } else {
                cpu_tester::run_with_args(rest).map_err(CliError::failure)
            }
        }
        "conformance" => run_conformance(rest),
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("kagura {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(CliError::failure(format!(
            "unknown command `{other}`\n\n{}",
            usage()
        ))),
    }
}

fn usage() -> String {
    [
        "Usage:",
        "  kagura assemble <input.asm> <output.bin>",
        "  kagura cpu-test <input.asm|input.bin> [--max-steps N]",
        "  kagura conformance <conformance-root> [--json]",
        "  kagura --version",
        "  kagura help",
    ]
    .join("\n")
}

fn run_conformance(args: Vec<String>) -> Result<(), CliError> {
    let mut root = None;
    let mut json = false;
    for argument in args {
        match argument.as_str() {
            "--json" => json = true,
            "help" | "--help" | "-h" => {
                println!("Usage: kagura conformance <conformance-root> [--json]");
                return Ok(());
            }
            _ if root.is_none() => root = Some(argument),
            _ => {
                return Err(CliError::failure(format!(
                    "unexpected argument `{argument}`"
                )));
            }
        }
    }
    let root = root.map(std::path::PathBuf::from).ok_or_else(|| {
        CliError::suite("conformance root is required; pass the Kagura suite directory explicitly")
    })?;
    let coverage_errors = coverage::validate(&root);
    if !coverage_errors.is_empty() {
        if json {
            for result in &coverage_errors {
                println!(
                    "{}",
                    serde_json::to_string(result).map_err(|error| CliError::suite(format!(
                        "cannot serialize result: {error}"
                    )))?
                );
            }
        } else {
            for result in &coverage_errors {
                let source = result.source.as_deref().unwrap_or("<coverage>");
                println!("error coverage {source}");
                if let Some(message) = &result.message {
                    println!("      {message}");
                }
            }
        }
        return Err(CliError::suite("coverage validation failed"));
    }
    let results = conformance_runner::run_all(&root);
    let summary = conformance_runner::Summary::from_results(&results);

    if json {
        for result in &results {
            println!(
                "{}",
                serde_json::to_string(result).map_err(|error| CliError::suite(format!(
                    "cannot serialize result: {error}"
                )))?
            );
        }
    } else {
        for result in &results {
            let case = result
                .case
                .as_deref()
                .or(result.source.as_deref())
                .unwrap_or("<unknown>");
            println!("{:<5} {:<3} {}", result.result, result.suite, case);
            for mismatch in &result.mismatches {
                println!(
                    "      {}: expected {}, actual {}",
                    mismatch.path, mismatch.expected, mismatch.actual
                );
            }
            if let Some(message) = &result.message {
                println!("      {message}");
            }
        }
        println!(
            "\n{} cases: {} passed, {} failed, {} errors",
            summary.total, summary.passed, summary.failed, summary.errors
        );
    }

    if summary.errors > 0 {
        return Err(CliError::suite("conformance suite contains errors"));
    }
    if summary.failed > 0 {
        return Err(CliError::failure(
            "reference implementation failed conformance",
        ));
    }
    Ok(())
}
