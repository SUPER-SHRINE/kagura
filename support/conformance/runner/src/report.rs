use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct Mismatch {
    pub path: String,
    pub expected: Value,
    pub actual: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub format: &'static str,
    pub version: u32,
    pub suite: &'static str,
    pub case: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub level: Option<u8>,
    pub result: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mismatches: Vec<Mismatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl CaseResult {
    pub fn finished(
        suite: &'static str,
        case: String,
        level: Option<u8>,
        mismatches: Vec<Mismatch>,
    ) -> Self {
        Self {
            format: "kagura-conformance-result",
            version: 1,
            suite,
            case: Some(case),
            source: None,
            level,
            result: if mismatches.is_empty() {
                "pass"
            } else {
                "fail"
            },
            mismatches,
            message: None,
        }
    }

    pub fn error(suite: &'static str, source: String, message: String) -> Self {
        Self {
            format: "kagura-conformance-result",
            version: 1,
            suite,
            case: None,
            source: Some(source),
            level: None,
            result: "error",
            mismatches: Vec::new(),
            message: Some(message),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
}

impl Summary {
    pub fn from_results(results: &[CaseResult]) -> Self {
        Self {
            total: results.len(),
            passed: results.iter().filter(|r| r.result == "pass").count(),
            failed: results.iter().filter(|r| r.result == "fail").count(),
            errors: results.iter().filter(|r| r.result == "error").count(),
        }
    }
}
