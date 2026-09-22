use std::fs;
use std::path::Path;

pub fn run_with_args<I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(input) = args.next() else {
        return Err(usage("kagura assemble"));
    };

    let Some(output) = args.next() else {
        return Err(usage("kagura assemble"));
    };

    if args.next().is_some() {
        return Err(usage("kagura assemble"));
    }

    let source =
        fs::read_to_string(&input).map_err(|error| format!("failed to read `{input}`: {error}"))?;
    let program = kagura_assembly::assemble(&source)
        .map_err(|error| format!("assembly failed for `{input}`: {error}"))?;

    if let Some(parent) = Path::new(&output).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create output directory `{}`: {error}",
                parent.display()
            )
        })?;
    }

    fs::write(&output, program.to_le_bytes())
        .map_err(|error| format!("failed to write `{output}`: {error}"))
}

pub fn usage(bin_name: &str) -> String {
    [
        "Usage:",
        &format!("  {bin_name} <input.asm> <output.bin>"),
        &format!("  {bin_name} help"),
    ]
    .join("\n")
}
