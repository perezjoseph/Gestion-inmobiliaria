use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorSuite {
    Rust,
    TypeScript,
    Kotlin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopDecision {
    Continue,
    Stop { exit_code: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FailureType {
    Compilation = 0,
    Lint = 1,
    Test = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub failure_type: FailureType,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorResult {
    pub name: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixStatus {
    Clean,
    Partial,
}

pub fn dispatch_formatter(file_path: &str) -> Option<String> {
    let ext = Path::new(file_path).extension().and_then(|e| e.to_str());
    if matches!(ext, Some(e) if e.eq_ignore_ascii_case("rs")) {
        Some(format!("cargo fmt -- {file_path}"))
    } else if matches!(ext, Some(e) if e.eq_ignore_ascii_case("ts") || e.eq_ignore_ascii_case("tsx"))
        && file_path.contains("baileys-service/")
    {
        Some(format!("npx eslint --fix {file_path}"))
    } else if matches!(ext, Some(e) if e.eq_ignore_ascii_case("kt"))
        && file_path.contains("android/")
    {
        Some("./gradlew spotlessApply".to_string())
    } else {
        None
    }
}

pub fn select_sensors(modified_files: &[&str]) -> HashSet<SensorSuite> {
    let mut suites = HashSet::new();

    for path in modified_files {
        let ext = Path::new(path).extension().and_then(|e| e.to_str());
        if matches!(ext, Some(e) if e.eq_ignore_ascii_case("rs")) {
            suites.insert(SensorSuite::Rust);
        } else if path.contains("baileys-service/")
            && matches!(ext, Some(e) if e.eq_ignore_ascii_case("ts") || e.eq_ignore_ascii_case("tsx") || e.eq_ignore_ascii_case("json"))
        {
            suites.insert(SensorSuite::TypeScript);
        } else if path.contains("android/")
            && matches!(ext, Some(e) if e.eq_ignore_ascii_case("kt") || e.eq_ignore_ascii_case("kts"))
        {
            suites.insert(SensorSuite::Kotlin);
        }
    }

    suites
}

pub fn parse_max_iterations(env_value: Option<&str>) -> u32 {
    const DEFAULT: u32 = 3;

    env_value.map_or(DEFAULT, |s| {
        s.parse::<i64>()
            .ok()
            .filter(|&n| n > 0)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(DEFAULT)
    })
}

pub fn should_continue_loop(
    iteration: u32,
    max: u32,
    sensor_results: &[SensorResult],
) -> LoopDecision {
    let all_pass = sensor_results.iter().all(|r| r.passed);

    if all_pass {
        return LoopDecision::Stop { exit_code: 0 };
    }

    if iteration < max {
        return LoopDecision::Continue;
    }

    let progress_made = sensor_results.iter().any(|r| r.passed);
    let exit_code = if progress_made { 1 } else { 2 };
    LoopDecision::Stop { exit_code }
}

pub const fn determine_exit_code(all_pass: bool, max_reached: bool, progress_made: bool) -> u8 {
    if all_pass {
        0
    } else if max_reached && progress_made {
        1
    } else if max_reached {
        2
    } else {
        0
    }
}

pub fn format_commit_message(
    scope: &str,
    subject: &str,
    root_cause: &str,
    changes: &[String],
    sensors: &[SensorResult],
    status: FixStatus,
    iteration: u32,
    max_iterations: u32,
) -> String {
    let prefix = format!("fix({scope}): ");
    let max_subject_len = 70usize.saturating_sub(prefix.len());
    let truncated_subject = if subject.len() > max_subject_len {
        &subject[..max_subject_len]
    } else {
        subject
    };
    let first_line = format!("{prefix}{truncated_subject}");

    let mut msg = first_line;
    msg.push_str("\n\nRoot cause:\n");
    let _ = writeln!(msg, "- {root_cause}");

    msg.push_str("\nChanges:\n");
    for change in changes {
        let _ = writeln!(msg, "- {change}");
    }

    msg.push_str("\nVerification:\n");
    for sensor in sensors {
        let status_str = if sensor.passed { "PASS" } else { "FAIL" };
        let _ = writeln!(msg, "- {}: {status_str}", sensor.name);
    }

    let status_str = match status {
        FixStatus::Clean => "CLEAN",
        FixStatus::Partial => "PARTIAL",
    };
    let _ = writeln!(msg, "\nStatus: {status_str}");
    let _ = writeln!(msg, "Iteration: {iteration}/{max_iterations}");

    msg
}

pub fn prioritize_failures(failures: &mut [Failure]) {
    failures.sort_by_key(|f| f.failure_type);
}

#[cfg(test)]
mod harness_pbt;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_formatter_rust_file() {
        let result = dispatch_formatter("backend/src/main.rs");
        assert_eq!(result, Some("cargo fmt -- backend/src/main.rs".to_string()));
    }

    #[test]
    fn dispatch_formatter_ts_in_baileys() {
        let result = dispatch_formatter("baileys-service/src/index.ts");
        assert_eq!(
            result,
            Some("npx eslint --fix baileys-service/src/index.ts".to_string())
        );
    }

    #[test]
    fn dispatch_formatter_tsx_in_baileys() {
        let result = dispatch_formatter("baileys-service/components/App.tsx");
        assert_eq!(
            result,
            Some("npx eslint --fix baileys-service/components/App.tsx".to_string())
        );
    }

    #[test]
    fn dispatch_formatter_kotlin_in_android() {
        let result = dispatch_formatter("android/app/src/main/MyActivity.kt");
        assert_eq!(result, Some("./gradlew spotlessApply".to_string()));
    }

    #[test]
    fn dispatch_formatter_ts_outside_baileys_returns_none() {
        let result = dispatch_formatter("frontend/src/utils.ts");
        assert_eq!(result, None);
    }

    #[test]
    fn dispatch_formatter_unknown_extension_returns_none() {
        let result = dispatch_formatter("README.md");
        assert_eq!(result, None);
    }

    #[test]
    fn select_sensors_rust_only() {
        let files = vec!["backend/src/main.rs", "backend/src/lib.rs"];
        let result = select_sensors(&files);
        assert_eq!(result, HashSet::from([SensorSuite::Rust]));
    }

    #[test]
    fn select_sensors_typescript_only() {
        let files = vec![
            "baileys-service/src/index.ts",
            "baileys-service/package.json",
        ];
        let result = select_sensors(&files);
        assert_eq!(result, HashSet::from([SensorSuite::TypeScript]));
    }

    #[test]
    fn select_sensors_kotlin_only() {
        let files = vec!["android/app/src/main/MyActivity.kt"];
        let result = select_sensors(&files);
        assert_eq!(result, HashSet::from([SensorSuite::Kotlin]));
    }

    #[test]
    fn select_sensors_mixed() {
        let files = vec![
            "backend/src/main.rs",
            "baileys-service/src/index.ts",
            "android/app/src/main/MyActivity.kt",
        ];
        let result = select_sensors(&files);
        assert_eq!(
            result,
            HashSet::from([
                SensorSuite::Rust,
                SensorSuite::TypeScript,
                SensorSuite::Kotlin
            ])
        );
    }

    #[test]
    fn select_sensors_non_code_only() {
        let files = vec!["README.md", ".github/workflows/ci.yml", "Dockerfile"];
        let result = select_sensors(&files);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_max_iterations_valid() {
        assert_eq!(parse_max_iterations(Some("5")), 5);
        assert_eq!(parse_max_iterations(Some("1")), 1);
        assert_eq!(parse_max_iterations(Some("100")), 100);
    }

    #[test]
    fn parse_max_iterations_defaults() {
        assert_eq!(parse_max_iterations(None), 3);
        assert_eq!(parse_max_iterations(Some("")), 3);
        assert_eq!(parse_max_iterations(Some("abc")), 3);
        assert_eq!(parse_max_iterations(Some("0")), 3);
        assert_eq!(parse_max_iterations(Some("-1")), 3);
    }

    #[test]
    fn should_continue_loop_all_pass() {
        let results = vec![
            SensorResult {
                name: "fmt".into(),
                passed: true,
            },
            SensorResult {
                name: "clippy".into(),
                passed: true,
            },
        ];
        assert_eq!(
            should_continue_loop(1, 3, &results),
            LoopDecision::Stop { exit_code: 0 }
        );
    }

    #[test]
    fn should_continue_loop_below_max_with_failures() {
        let results = vec![
            SensorResult {
                name: "fmt".into(),
                passed: true,
            },
            SensorResult {
                name: "clippy".into(),
                passed: false,
            },
        ];
        assert_eq!(should_continue_loop(1, 3, &results), LoopDecision::Continue);
    }

    #[test]
    fn should_continue_loop_max_reached_with_progress() {
        let results = vec![
            SensorResult {
                name: "fmt".into(),
                passed: true,
            },
            SensorResult {
                name: "test".into(),
                passed: false,
            },
        ];
        assert_eq!(
            should_continue_loop(3, 3, &results),
            LoopDecision::Stop { exit_code: 1 }
        );
    }

    #[test]
    fn should_continue_loop_max_reached_no_progress() {
        let results = vec![
            SensorResult {
                name: "fmt".into(),
                passed: false,
            },
            SensorResult {
                name: "clippy".into(),
                passed: false,
            },
        ];
        assert_eq!(
            should_continue_loop(3, 3, &results),
            LoopDecision::Stop { exit_code: 2 }
        );
    }

    #[test]
    fn determine_exit_code_all_pass() {
        assert_eq!(determine_exit_code(true, false, false), 0);
        assert_eq!(determine_exit_code(true, true, true), 0);
    }

    #[test]
    fn determine_exit_code_partial_progress() {
        assert_eq!(determine_exit_code(false, true, true), 1);
    }

    #[test]
    fn determine_exit_code_no_progress() {
        assert_eq!(determine_exit_code(false, true, false), 2);
    }

    #[test]
    fn format_commit_message_first_line_max_70() {
        let msg = format_commit_message(
            "backend",
            "fix type error in handler that was causing compilation failure in CI",
            "TS2304 missing import",
            &["src/handler.rs".to_string()],
            &[SensorResult {
                name: "clippy".into(),
                passed: true,
            }],
            FixStatus::Clean,
            1,
            3,
        );
        let first_line = msg.lines().next().unwrap();
        assert!(
            first_line.len() <= 70,
            "First line was {} chars",
            first_line.len()
        );
    }

    #[test]
    fn format_commit_message_contains_sections() {
        let msg = format_commit_message(
            "backend",
            "resolve clippy warning",
            "unused import",
            &["src/main.rs".to_string(), "src/lib.rs".to_string()],
            &[
                SensorResult {
                    name: "fmt".into(),
                    passed: true,
                },
                SensorResult {
                    name: "clippy".into(),
                    passed: false,
                },
            ],
            FixStatus::Partial,
            2,
            3,
        );
        assert!(msg.contains("Root cause:"));
        assert!(msg.contains("Changes:"));
        assert!(msg.contains("- src/main.rs"));
        assert!(msg.contains("- src/lib.rs"));
        assert!(msg.contains("Verification:"));
        assert!(msg.contains("- fmt: PASS"));
        assert!(msg.contains("- clippy: FAIL"));
        assert!(msg.contains("Status: PARTIAL"));
        assert!(msg.contains("Iteration: 2/3"));
    }

    #[test]
    fn prioritize_failures_ordering() {
        let mut failures = vec![
            Failure {
                failure_type: FailureType::Test,
                message: "test1".into(),
            },
            Failure {
                failure_type: FailureType::Compilation,
                message: "comp1".into(),
            },
            Failure {
                failure_type: FailureType::Lint,
                message: "lint1".into(),
            },
            Failure {
                failure_type: FailureType::Compilation,
                message: "comp2".into(),
            },
            Failure {
                failure_type: FailureType::Test,
                message: "test2".into(),
            },
        ];
        prioritize_failures(&mut failures);

        assert_eq!(failures[0].failure_type, FailureType::Compilation);
        assert_eq!(failures[1].failure_type, FailureType::Compilation);
        assert_eq!(failures[2].failure_type, FailureType::Lint);
        assert_eq!(failures[3].failure_type, FailureType::Test);
        assert_eq!(failures[4].failure_type, FailureType::Test);
    }

    #[test]
    fn prioritize_failures_stable_within_type() {
        let mut failures = vec![
            Failure {
                failure_type: FailureType::Lint,
                message: "first".into(),
            },
            Failure {
                failure_type: FailureType::Lint,
                message: "second".into(),
            },
            Failure {
                failure_type: FailureType::Lint,
                message: "third".into(),
            },
        ];
        prioritize_failures(&mut failures);

        assert_eq!(failures[0].message, "first");
        assert_eq!(failures[1].message, "second");
        assert_eq!(failures[2].message, "third");
    }
}
