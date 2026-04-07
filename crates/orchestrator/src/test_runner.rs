use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Detected test framework for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestFramework {
    CargoTest,
    Pytest,
    Jest,
    GoTest,
    Custom(String),
}

impl TestFramework {
    /// The shell command to run tests.
    #[must_use]
    pub fn command(&self) -> (&str, Vec<&str>) {
        match self {
            Self::CargoTest => ("cargo", vec!["test", "--workspace"]),
            Self::Pytest => ("python", vec!["-m", "pytest", "-v"]),
            Self::Jest => ("npx", vec!["jest", "--verbose"]),
            Self::GoTest => ("go", vec!["test", "./..."]),
            Self::Custom(cmd) => ("sh", vec!["-c", cmd.as_str()]),
        }
    }

    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::CargoTest => "cargo test",
            Self::Pytest => "pytest",
            Self::Jest => "jest",
            Self::GoTest => "go test",
            Self::Custom(cmd) => cmd.as_str(),
        }
    }
}

/// Result of running a test suite.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub passed: bool,
    pub total: u32,
    pub failed: u32,
    pub output: String,
    pub failed_tests: Vec<FailedTest>,
    pub duration_ms: u64,
}

/// Details about a single failed test.
#[derive(Debug, Clone)]
pub struct FailedTest {
    pub name: String,
    pub file: String,
    pub error: String,
}

/// Runs tests for a project, auto-detecting the framework.
pub struct TestRunner {
    project_root: PathBuf,
    framework: TestFramework,
}

impl TestRunner {
    /// Create a test runner, auto-detecting the framework.
    #[must_use]
    pub fn new(project_root: &Path) -> Self {
        let framework = detect_framework(project_root);
        Self {
            project_root: project_root.to_path_buf(),
            framework,
        }
    }

    /// Create with an explicit framework override.
    #[must_use]
    pub fn with_framework(project_root: &Path, framework: TestFramework) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            framework,
        }
    }

    #[must_use]
    pub fn framework(&self) -> &TestFramework {
        &self.framework
    }

    /// Run unit tests, optionally scoped to specific files/modules.
    pub fn run_unit_tests(&self, scope: &[PathBuf]) -> TestResult {
        let (program, mut args) = self.framework.command();

        // For cargo test, add scope filter if provided
        let scope_args: Vec<String>;
        if !scope.is_empty() && self.framework == TestFramework::CargoTest {
            scope_args = scope
                .iter()
                .filter_map(|p| p.file_stem())
                .filter_map(|s| s.to_str())
                .map(String::from)
                .collect();
            if let Some(first) = scope_args.first() {
                args.push("--");
                args.push(first.as_str());
            }
        }

        self.execute_tests(program, &args)
    }

    /// Run the full E2E / integration test suite.
    #[must_use]
    pub fn run_e2e_tests(&self) -> TestResult {
        let (program, args) = self.framework.command();
        self.execute_tests(program, &args)
    }

    fn execute_tests(&self, program: &str, args: &[&str]) -> TestResult {
        let start = Instant::now();

        let output = Command::new(program)
            .args(args)
            .current_dir(&self.project_root)
            .output();

        #[allow(clippy::cast_possible_truncation)]
        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let combined = format!("{stdout}\n{stderr}");
                let passed = output.status.success();

                let (total, failed, failed_tests) = parse_test_counts(&combined, &self.framework);

                TestResult {
                    passed,
                    total,
                    failed,
                    output: combined,
                    failed_tests,
                    duration_ms,
                }
            }
            Err(e) => TestResult {
                passed: false,
                total: 0,
                failed: 0,
                output: format!("Failed to run tests: {e}"),
                failed_tests: vec![],
                duration_ms,
            },
        }
    }
}

/// Auto-detect test framework from project files.
#[must_use]
pub fn detect_framework(project_root: &Path) -> TestFramework {
    if project_root.join("Cargo.toml").exists() {
        TestFramework::CargoTest
    } else if project_root.join("pytest.ini").exists()
        || project_root.join("pyproject.toml").exists()
        || project_root.join("setup.py").exists()
    {
        TestFramework::Pytest
    } else if project_root.join("jest.config.js").exists()
        || project_root.join("jest.config.ts").exists()
        || project_root.join("package.json").exists()
    {
        TestFramework::Jest
    } else if project_root.join("go.mod").exists() {
        TestFramework::GoTest
    } else {
        // Default to cargo test for Rust projects
        TestFramework::CargoTest
    }
}

/// Parse test output to extract pass/fail counts (best-effort).
fn parse_test_counts(output: &str, framework: &TestFramework) -> (u32, u32, Vec<FailedTest>) {
    let mut total = 0_u32;
    let mut failed = 0_u32;
    let mut failed_tests = Vec::new();

    match framework {
        TestFramework::CargoTest => {
            // Look for "test result: ok. N passed; M failed"
            for line in output.lines() {
                if line.starts_with("test result:") {
                    let counts = parse_cargo_test_result_line(line);
                    total += counts.0;
                    failed += counts.1;
                }
                // Collect failed test names
                if line.contains("FAILED") && line.starts_with("test ") {
                    let name = line
                        .trim_start_matches("test ")
                        .split(" ...")
                        .next()
                        .unwrap_or(line)
                        .to_string();
                    failed_tests.push(FailedTest {
                        name,
                        file: String::new(),
                        error: String::new(),
                    });
                }
            }
        }
        _ => {
            // Generic: count lines with "PASS" / "FAIL" / "ok" / "FAILED"
            for line in output.lines() {
                if line.contains("PASS") || line.contains("ok") {
                    total += 1;
                }
                if line.contains("FAIL") || line.contains("FAILED") {
                    total += 1;
                    failed += 1;
                }
            }
        }
    }

    (total, failed, failed_tests)
}

/// Parse "test result: ok. 14 passed; 0 failed; 0 ignored; ..."
fn parse_cargo_test_result_line(line: &str) -> (u32, u32) {
    let mut passed = 0_u32;
    let mut failed = 0_u32;

    for part in line.split(';') {
        let trimmed = part.trim();
        if trimmed.ends_with("passed") {
            if let Some(n) = trimmed.split_whitespace().next() {
                passed = n.parse().unwrap_or(0);
            }
        } else if trimmed.ends_with("failed") {
            if let Some(n) = trimmed.split_whitespace().next() {
                failed = n.parse().unwrap_or(0);
            }
        }
    }

    (passed + failed, failed)
}
