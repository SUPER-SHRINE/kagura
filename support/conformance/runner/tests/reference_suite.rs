use kagura_conformance_runner::{Summary, run_all};
use std::path::PathBuf;

#[test]
fn reference_implementation_passes_all_normative_cases() {
    let root = std::env::var_os("KAGURA_CONFORMANCE_SUITE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../specs/kagura/v1/conformance")
        });
    let results = run_all(&root);
    let summary = Summary::from_results(&results);

    if summary.passed != summary.total {
        let failures = results
            .iter()
            .filter(|result| result.result != "pass")
            .map(|result| {
                format!(
                    "{}: {}",
                    result.case.as_deref().unwrap_or("<invalid case>"),
                    serde_json::to_string(result).unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "reference conformance failed: {} passed, {} failed, {} errors\n{}",
            summary.passed, summary.failed, summary.errors, failures
        );
    }

    assert_eq!(summary.total, 180, "unexpected conformance case count");
}
