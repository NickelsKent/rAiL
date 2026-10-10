//! S0: the walking-skeleton smoke suite (BR8.1, FR27). The seven steps of
//! the walking-skeleton check, driven through `rail`'s command line and its
//! RAP server only, against fresh copies of `fixtures/` and the golden
//! files shared by both platforms (`crates/rail/tests/golden/`).
//!
//! Each step is one input; a volume of `count n` runs the seven steps `n`
//! times. Steps record what differs from the expected output as findings,
//! and the judge fails a step with findings. Step 7 passes when steps 1-4
//! of the same round passed.

use std::path::{Path, PathBuf};

use crate::harness::process::{Proc, RunContext, RunError};
use crate::harness::settings::{Driver, SuiteId, Volume};
use crate::suites::{Input, Observation, Suite};
use crate::{copy_dir, frame};

/// The fixtures S0 copies into each fresh workspace.
pub const FIXTURES: &str = "fixtures";

/// The golden files shared with `crates/rail/tests/e2e.rs`.
pub const GOLDEN: &str = "crates/rail/tests/golden";

/// The seven steps, as printed in PASS and FAIL lines.
pub const STEPS: [&str; 7] = [
    "check skeleton.answer: no diagnostics",
    "run skeleton.answer: prints 42, exit 0",
    "protocol session on skeleton.answer: run.run returns 42",
    "skeleton.broken refused with one TY001 (command line and protocol)",
    "server answers after a malformed message",
    "two builds of skeleton.answer are byte-identical",
    "outputs of steps 1-4 equal the shared golden files",
];

const INITIALIZED: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"rap_version":"0.1","capabilities":["tree.get","check.run","build.run","run.run"]}}"#;

/// The walking-skeleton check.
pub struct SkeletonSmoke;

/// The step number (1-7) of an input.
fn step_of(input: &Input) -> Result<usize, RunError> {
    let label = String::from_utf8_lossy(&input.data).into_owned();
    let step = label
        .split(' ')
        .next()
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|n| (1..=7).contains(n))
        .ok_or_else(|| RunError::Suite(format!("not an S0 step: {label:?}")))?;
    Ok(step)
}

/// Records a finding when `actual` is not `expected`.
fn expect_eq(findings: &mut Vec<String>, what: &str, expected: &[u8], actual: &[u8]) {
    if expected != actual {
        findings.push(format!(
            "{what}: expected {:?}, got {:?}",
            String::from_utf8_lossy(expected),
            String::from_utf8_lossy(actual)
        ));
    }
}

fn expect_code(findings: &mut Vec<String>, what: &str, expected: i32, actual: Option<i32>) {
    if actual != Some(expected) {
        findings.push(format!(
            "{what}: expected exit code {expected}, got {actual:?}"
        ));
    }
}

/// The framed response bodies in a server's output, in order.
fn unframe(bytes: &[u8]) -> Vec<String> {
    let mut bodies = Vec::new();
    let mut rest = bytes;
    while let Some(after) = rest.strip_prefix(b"Content-Length: ") {
        let Some(end) = after.windows(4).position(|w| w == b"\r\n\r\n") else {
            break;
        };
        let Some(length) = std::str::from_utf8(&after[..end])
            .ok()
            .and_then(|d| d.parse::<usize>().ok())
        else {
            break;
        };
        let body_start = end + 4;
        let Some(body) = after.get(body_start..body_start + length) else {
            break;
        };
        bodies.push(String::from_utf8_lossy(body).into_owned());
        rest = &after[body_start + length..];
    }
    bodies
}

struct Step<'a> {
    ctx: &'a RunContext,
    dir: PathBuf,
    golden: PathBuf,
    obs: Observation,
}

impl Step<'_> {
    fn golden(&self, name: &str) -> Result<Vec<u8>, RunError> {
        let path = self.golden.join(name);
        std::fs::read(&path)
            .map_err(|e| RunError::Suite(format!("cannot read {}: {e}", path.display())))
    }

    /// A golden file as one line (the form RAP results take).
    fn golden_line(&self, name: &str) -> Result<String, RunError> {
        Ok(String::from_utf8_lossy(&self.golden(name)?)
            .trim_end()
            .to_string())
    }

    /// A fresh workspace holding the fixtures.
    fn workspace(&self, name: &str) -> Result<PathBuf, RunError> {
        let ws = self.dir.join(name);
        copy_dir(&self.ctx.repo_root().join(FIXTURES), &ws)
            .map_err(|e| RunError::Suite(format!("cannot copy {FIXTURES}: {e}")))?;
        Ok(ws)
    }

    /// Runs `rail <args>` in `ws` with `stdin`, recording the output.
    fn rail(
        &mut self,
        ws: &Path,
        args: &[&str],
        stdin: Vec<u8>,
    ) -> Result<(Option<i32>, Vec<u8>), RunError> {
        let result = self.ctx.run(
            &Proc::new(self.ctx.rail())
                .args(args.iter().copied())
                .cwd(ws)
                .stdin(stdin),
        )?;
        self.obs.timed_out |= result.timed_out;
        self.obs.exit_status = result.code;
        self.obs.signal = result.signal;
        self.obs.stdout = result.stdout.clone();
        self.obs.stderr = result.stderr;
        Ok((result.code, result.stdout))
    }

    /// Runs a RAP session in `ws` with the given requests after
    /// `initialize`, returning the response bodies.
    fn rap(&mut self, ws: &Path, requests: &[String]) -> Result<Vec<String>, RunError> {
        self.obs.driver = Some(Driver::Rap);
        let root = rail_json::to_string(&rail_json::Value::str(&ws.display().to_string()));
        let mut input = frame(
            format!(r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"client_versions":["0.1"],"workspace_root":{root}}}}}"#)
                .as_bytes(),
        );
        for request in requests {
            input.extend(frame(request.as_bytes()));
        }
        let (code, stdout) = self.rail(ws, &["rap"], input)?;
        expect_code(&mut self.obs.findings, "rail rap", 0, code);
        let bodies = unframe(&stdout);
        self.obs.messages = bodies.clone();
        Ok(bodies)
    }
}

fn request(id: u32, method: &str, module: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{"module":"{module}"}}}}"#)
}

fn result_line(id: u32, result: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"result":{result}}}"#)
}

fn build_target() -> &'static str {
    if cfg!(target_os = "macos") {
        "aarch64-macos"
    } else {
        "x86_64-linux"
    }
}

fn expect_lines(findings: &mut Vec<String>, what: &str, expected: &[String], actual: &[String]) {
    if expected != actual {
        findings.push(format!("{what}: expected {expected:?}, got {actual:?}"));
    }
}

const ANSWER: &str = "skeleton/answer.rlc";
const BROKEN: &str = "skeleton/broken.rlc";
const BINARY: &str = ".rail/build/dev/skeleton/answer";

impl Step<'_> {
    fn check_answer(&mut self) -> Result<(), RunError> {
        let ws = self.workspace("ws")?;
        let (code, out) = self.rail(&ws, &["check", "--json", ANSWER], Vec::new())?;
        let golden = self.golden("check-answer.json")?;
        expect_code(&mut self.obs.findings, "rail check --json", 0, code);
        expect_eq(&mut self.obs.findings, "check-answer.json", &golden, &out);
        Ok(())
    }

    fn run_answer(&mut self) -> Result<(), RunError> {
        let ws = self.workspace("ws")?;
        let (code, out) = self.rail(&ws, &["run", ANSWER], Vec::new())?;
        expect_code(&mut self.obs.findings, "rail run", 0, code);
        expect_eq(&mut self.obs.findings, "rail run output", b"42\n", &out);
        Ok(())
    }

    fn protocol_session(&mut self) -> Result<(), RunError> {
        let ws = self.workspace("ws")?;
        let methods = ["tree.get", "check.run", "build.run", "run.run"];
        let requests: Vec<String> = (2..)
            .zip(methods)
            .map(|(id, m)| request(id, m, ANSWER))
            .collect();
        let bodies = self.rap(&ws, &requests)?;
        let build = format!(
            r#"{{"module":"skeleton.answer","target":"{}","mode":"dev","path":"{BINARY}"}}"#,
            build_target()
        );
        let expected = vec![
            INITIALIZED.to_string(),
            result_line(2, &self.golden_line("tree-answer.json")?),
            result_line(3, &self.golden_line("check-answer.json")?),
            result_line(4, &build),
            result_line(5, &self.golden_line("run-answer.json")?),
        ];
        expect_lines(
            &mut self.obs.findings,
            "protocol session",
            &expected,
            &bodies,
        );
        Ok(())
    }

    fn broken_refused(&mut self) -> Result<(), RunError> {
        let ws = self.workspace("ws")?;
        let (code, out) = self.rail(&ws, &["check", "--json", BROKEN], Vec::new())?;
        let golden = self.golden("check-broken.json")?;
        expect_code(&mut self.obs.findings, "rail check --json broken", 1, code);
        expect_eq(&mut self.obs.findings, "check-broken.json", &golden, &out);
        let (code, out) = self.rail(&ws, &["run", "--json", BROKEN], Vec::new())?;
        let golden = self.golden("run-broken.json")?;
        expect_code(&mut self.obs.findings, "rail run --json broken", 1, code);
        expect_eq(&mut self.obs.findings, "run-broken.json", &golden, &out);
        let run_broken = self.golden_line("run-broken.json")?;
        let tool_error = run_broken
            .strip_prefix(r#"{"error":"#)
            .and_then(|r| r.strip_suffix('}'))
            .unwrap_or(&run_broken)
            .to_string();
        let requests = [
            request(2, "check.run", BROKEN),
            request(3, "run.run", BROKEN),
        ];
        let bodies = self.rap(&ws, &requests)?;
        let expected = vec![
            result_line(2, &self.golden_line("check-broken.json")?),
            format!(
                r#"{{"jsonrpc":"2.0","id":3,"error":{{"code":-32000,"message":"skeleton.broken has blocking diagnostics","data":{tool_error}}}}}"#
            ),
        ];
        expect_lines(
            &mut self.obs.findings,
            "protocol refusal",
            &expected,
            bodies.get(1..).unwrap_or(&[]),
        );
        if ws.join(".rail/build/dev/skeleton/broken").exists() {
            self.obs
                .findings
                .push("a binary was built for skeleton.broken".into());
        }
        Ok(())
    }

    fn malformed_message(&mut self) -> Result<(), RunError> {
        let ws = self.workspace("ws")?;
        let requests = [
            r#"{"jsonrpc":"2.0","id":2,"#.to_string(),
            request(3, "check.run", ANSWER),
        ];
        let bodies = self.rap(&ws, &requests)?;
        let parse_error = bodies.get(1).is_some_and(|b| {
            b.starts_with(r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"#)
        });
        if !parse_error {
            self.obs.findings.push(format!(
                "expected a -32700 parse error, got {:?}",
                bodies.get(1)
            ));
        }
        let expected = vec![result_line(3, &self.golden_line("check-answer.json")?)];
        expect_lines(
            &mut self.obs.findings,
            "after the malformed message",
            &expected,
            bodies.get(2..).unwrap_or(&[]),
        );
        Ok(())
    }

    fn identical_builds(&mut self) -> Result<(), RunError> {
        let mut binaries = Vec::new();
        for name in ["one", "two"] {
            let ws = self.workspace(name)?;
            let (code, _) = self.rail(&ws, &["build", ANSWER], Vec::new())?;
            expect_code(&mut self.obs.findings, "rail build", 0, code);
            binaries.push(std::fs::read(ws.join(BINARY)).unwrap_or_default());
        }
        if binaries[0].is_empty() || binaries[0] != binaries[1] {
            self.obs
                .findings
                .push("the two builds differ (or were not produced)".into());
        }
        Ok(())
    }
}

impl Suite for SkeletonSmoke {
    fn id(&self) -> SuiteId {
        SuiteId::S0
    }

    fn generate_inputs(&self, _seed: u64, volume: &[Volume]) -> Result<Vec<Input>, String> {
        let rounds: u64 = volume
            .iter()
            .map(|v| match v {
                Volume::Count { count } => *count,
                Volume::Exhaustive { .. } => 1,
                Volume::NotRun { .. } => 0,
            })
            .sum();
        let mut inputs = Vec::new();
        for round in 1..=rounds {
            for (n, title) in (1..).zip(STEPS) {
                let mut label = format!("{n} {title}");
                if rounds > 1 {
                    label.push_str(&format!(" (round {round})"));
                }
                inputs.push(Input::new(label.clone(), label));
            }
        }
        Ok(inputs)
    }

    fn run(&self, ctx: &RunContext, input: &Input) -> Result<Observation, RunError> {
        let step = step_of(input)?;
        // Each step gets its own subdirectory of the run's work directory.
        let dir: String = input
            .label
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let mut s = Step {
            ctx,
            dir: ctx.work_dir().join(dir),
            golden: ctx.repo_root().join(GOLDEN),
            obs: Observation::new(Driver::Cli),
        };
        match step {
            1 => s.check_answer()?,
            2 => s.run_answer()?,
            3 => s.protocol_session()?,
            4 => s.broken_refused()?,
            5 => s.malformed_message()?,
            6 => s.identical_builds()?,
            _ => s.obs.driver = None,
        }
        Ok(s.obs)
    }

    fn judge(&self, inputs: &[Input], observations: &[Observation]) -> Vec<Option<String>> {
        let mut verdicts: Vec<Option<String>> = Vec::with_capacity(inputs.len());
        for (i, obs) in observations.iter().enumerate() {
            let step = i % STEPS.len() + 1;
            let verdict = if obs.timed_out {
                Some("timed out".to_string())
            } else if !obs.findings.is_empty() {
                Some(obs.findings.join("; "))
            } else if step == 7 {
                let round = i - 6;
                verdicts[round..round + 4]
                    .iter()
                    .any(Option::is_some)
                    .then(|| "outputs of steps 1-4 differ from the shared golden files".to_string())
            } else {
                None
            };
            verdicts.push(verdict);
        }
        verdicts
    }

    fn report_each_input(&self) -> bool {
        true
    }
}
