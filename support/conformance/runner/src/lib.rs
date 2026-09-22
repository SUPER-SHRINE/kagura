#![forbid(unsafe_code)]

mod model;
mod report;

pub use report::{CaseResult, Mismatch, Summary};

use conformance_machine::{
    BusEvent as ActualBusEvent, ConformanceMachine, HIGH_ROM_BASE, RAM_BASE, RAM_SIZE,
    ReporterState,
};
use kagura::{AccessOperation, AccessWidth, Fault};
use model::{BusCase, BusEvent, CpuCase, DeviceState, Initial, MemoryRange, Transaction};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_all(conformance_root: &Path) -> Vec<CaseResult> {
    let mut results = Vec::new();
    results.extend(run_cpu_suite(&conformance_root.join("cases/cpu")));
    results.extend(run_bus_suite(&conformance_root.join("cases/bus")));
    results
}

pub fn run_cpu_suite(root: &Path) -> Vec<CaseResult> {
    run_files(root, "cpu", run_cpu_file)
}

pub fn run_bus_suite(root: &Path) -> Vec<CaseResult> {
    run_files(root, "bus", run_bus_file)
}

fn run_files(
    root: &Path,
    suite: &'static str,
    run: fn(&Path) -> Result<CaseResult, String>,
) -> Vec<CaseResult> {
    let mut files = Vec::new();
    if let Err(error) = collect_case_files(root, &mut files) {
        return vec![CaseResult::error(suite, root.display().to_string(), error)];
    }
    files.sort();
    files
        .into_iter()
        .map(|path| {
            run(&path).unwrap_or_else(|message| {
                CaseResult::error(suite, path.display().to_string(), message)
            })
        })
        .collect()
}

fn collect_case_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(root).map_err(|error| format!("cannot read {}: {error}", root.display()))?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            collect_case_files(&path, files)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some("case.toml") {
            files.push(path);
        }
    }
    Ok(())
}

fn run_cpu_file(path: &Path) -> Result<CaseResult, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("cannot read case TOML: {error}"))?;
    let case: CpuCase = toml::from_str(&text).map_err(|error| error.to_string())?;
    validate_cpu_case(&case)?;
    let binary_path = safe_case_file(path, &case.binary)?;
    let binary = fs::read(&binary_path)
        .map_err(|error| format!("cannot read {}: {error}", binary_path.display()))?;

    let mut machine = ReferenceMachine::new()?;
    machine.setup_cpu(&case, &binary)?;
    let actual = machine.execute(&case)?;
    let mismatches = machine.compare_cpu(&case, &actual)?;
    Ok(CaseResult::finished(
        "cpu",
        case.id,
        Some(case.level),
        mismatches,
    ))
}

fn run_bus_file(path: &Path) -> Result<CaseResult, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("cannot read case TOML: {error}"))?;
    let case: BusCase = toml::from_str(&text).map_err(|error| error.to_string())?;
    validate_bus_case(&case)?;
    let mut machine = ReferenceMachine::new()?;
    machine.setup_common(&case.initial)?;
    let actual = machine.execute_transactions(&case.transactions)?;
    let mismatches = machine.compare_bus(&case, &actual)?;
    Ok(CaseResult::finished("bus", case.id, None, mismatches))
}

fn safe_case_file(case_toml: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err("binary path must stay inside the case directory".into());
    }
    Ok(case_toml
        .parent()
        .ok_or("case TOML has no parent directory")?
        .join(relative))
}

fn validate_cpu_case(case: &CpuCase) -> Result<(), String> {
    if case.format != "kagura-conformance-case" || case.version != 1 {
        return Err("unsupported CPU case format or version".into());
    }
    if !(1..=3).contains(&case.level) || case.id.is_empty() || case.spec.is_empty() {
        return Err("invalid CPU case header".into());
    }
    if !matches!(case.image.load_address, RAM_BASE | HIGH_ROM_BASE) {
        return Err("image.load_address must be 0 or 0xFFFFFFFC".into());
    }
    validate_initial(&case.initial, case.level == 3)?;
    if case.execution.max_steps == 0 {
        return Err("execution.max_steps must be positive".into());
    }
    match case.execution.until.as_str() {
        "steps" => {
            let steps = case
                .execution
                .steps
                .ok_or("steps termination requires steps")?;
            if steps == 0 || steps > case.execution.max_steps {
                return Err("execution.steps must be in 1..=max_steps".into());
            }
        }
        "fault" | "reporter" if case.execution.steps.is_none() => {}
        "fault" | "reporter" => return Err("steps is only valid for steps termination".into()),
        _ => return Err("unknown execution.until".into()),
    }
    if let Some(state) = &case.expected.state {
        validate_registers(&state.registers)?;
    }
    validate_devices(&case.expected.devices, case.level == 3)?;
    Ok(())
}

fn validate_bus_case(case: &BusCase) -> Result<(), String> {
    if case.format != "kagura-bus-conformance-case" || case.version != 1 {
        return Err("unsupported Bus case format or version".into());
    }
    if case.id.is_empty() || case.spec.is_empty() || case.transactions.is_empty() {
        return Err("invalid Bus case header or empty transaction list".into());
    }
    if case.transactions.len() != case.expected.transactions.len() {
        return Err("transaction and expected transaction counts differ".into());
    }
    validate_initial(&case.initial, true)?;
    validate_devices(&case.expected.devices, true)?;
    for transaction in &case.transactions {
        validate_transaction(transaction)?;
    }
    Ok(())
}

fn validate_initial(initial: &Initial, allow_devices: bool) -> Result<(), String> {
    validate_registers(&initial.registers)?;
    validate_devices(&initial.devices, allow_devices)?;
    for range in &initial.memory {
        validate_memory_range(range)?;
    }
    Ok(())
}

fn validate_registers(registers: &BTreeMap<String, u32>) -> Result<(), String> {
    for (name, value) in registers {
        let Some(index) = name.strip_prefix('r').and_then(|n| n.parse::<usize>().ok()) else {
            return Err(format!("invalid register name `{name}`"));
        };
        if index >= 16 || (index == 0 && *value != 0) {
            return Err(format!("invalid register assignment `{name}`"));
        }
    }
    Ok(())
}

fn validate_devices(devices: &[DeviceState], allow: bool) -> Result<(), String> {
    if !allow && !devices.is_empty() {
        return Err("device state is only valid at Level 3 or in Bus cases".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for device in devices {
        if !ids.insert(&device.id) {
            return Err(format!("duplicate device `{}`", device.id));
        }
        let expected: &[&str] = match device.id.as_str() {
            "reporter" => &["status", "case_id", "detail", "reserved"],
            "high-rom" => &["word"],
            other => return Err(format!("unknown device `{other}`")),
        };
        if device.state.len() != expected.len()
            || expected
                .iter()
                .any(|field| !device.state.contains_key(*field))
        {
            return Err(format!("invalid state fields for device `{}`", device.id));
        }
        if device.id == "reporter" && device.state["reserved"] != 0 {
            return Err("reporter reserved state must be zero".into());
        }
    }
    Ok(())
}

fn validate_memory_range(range: &MemoryRange) -> Result<(), String> {
    if range.bytes.is_empty()
        || u64::from(range.address) + range.bytes.len() as u64 > u64::from(RAM_SIZE)
    {
        return Err("memory range is empty or outside Test RAM".into());
    }
    Ok(())
}

fn validate_transaction(transaction: &Transaction) -> Result<(), String> {
    if !matches!(transaction.width, 8 | 16 | 32) {
        return Err("transaction width must be 8, 16, or 32".into());
    }
    match transaction.operation.as_str() {
        "read" if transaction.value.is_none() => Ok(()),
        "write" if transaction.value.is_some() => Ok(()),
        _ => Err("read must omit value and write must include value".into()),
    }
}

struct ReferenceMachine {
    machine: ConformanceMachine,
}

impl ReferenceMachine {
    fn new() -> Result<Self, String> {
        Ok(Self {
            machine: ConformanceMachine::new()?,
        })
    }

    fn setup_cpu(&mut self, case: &CpuCase, binary: &[u8]) -> Result<(), String> {
        if case.image.load_address == RAM_BASE {
            if binary.len() > RAM_SIZE as usize {
                return Err("binary does not fit in Test RAM".into());
            }
            self.machine
                .load_ram(binary)
                .map_err(|_| "binary load failed")?;
        } else {
            if binary.len() != 4 {
                return Err("High Test ROM binary must be exactly four bytes".into());
            }
            self.machine
                .set_high_rom_word(u32::from_le_bytes(binary.try_into().unwrap()));
        }
        self.setup_common(&case.initial)?;
        if let Some(pc) = case.initial.pc {
            self.machine.set_pc(pc);
        }
        for (name, value) in &case.initial.registers {
            self.machine.set_reg(register_index(name)?, *value);
        }
        self.machine.clear_trace();
        Ok(())
    }

    fn setup_common(&mut self, initial: &Initial) -> Result<(), String> {
        for range in &initial.memory {
            self.machine
                .patch_memory(range.address, &range.bytes)
                .map_err(|_| "initial memory patch failed")?;
        }
        for device in &initial.devices {
            self.restore_device(device)?;
        }
        self.machine.clear_trace();
        Ok(())
    }

    fn execute(&mut self, case: &CpuCase) -> Result<ActualTermination, String> {
        let mut retired = 0;
        for _ in 0..case.execution.max_steps {
            match self.machine.step() {
                Ok(()) => retired += 1,
                Err(fault) => return Ok(ActualTermination::fault(retired, fault)),
            }
            let reporter = self.machine.reporter();
            if reporter.status != 0 {
                return Ok(ActualTermination::reporter(retired, reporter));
            }
            if case.execution.until == "steps" && case.execution.steps == Some(retired) {
                return Ok(ActualTermination::steps(retired));
            }
        }
        Ok(ActualTermination::step_limit(retired))
    }

    fn execute_transactions(
        &mut self,
        transactions: &[Transaction],
    ) -> Result<Vec<ActualBusEvent>, String> {
        let mut actual = Vec::with_capacity(transactions.len());
        for transaction in transactions {
            actual.push(self.machine.perform(
                &transaction.operation,
                transaction.address,
                transaction.width,
                transaction.value,
            )?);
        }
        Ok(actual)
    }

    fn compare_cpu(
        &mut self,
        case: &CpuCase,
        actual: &ActualTermination,
    ) -> Result<Vec<Mismatch>, String> {
        let mut out = Vec::new();
        actual.compare(&case.expected.termination, &mut out);
        if let Some(state) = &case.expected.state {
            compare_option("state.pc", state.pc, Some(self.machine.pc()), &mut out);
            for (name, expected) in &state.registers {
                compare_value(
                    format!("state.registers.{name}"),
                    json!(expected),
                    json!(self.machine.reg(register_index(name)?)),
                    &mut out,
                );
            }
            self.compare_memory(&state.memory, "state.memory", &mut out)?;
        }
        compare_bus_events(&case.expected.bus, self.machine.trace(), "bus", &mut out);
        self.compare_devices(&case.expected.devices, &mut out)?;
        Ok(out)
    }

    fn compare_bus(
        &mut self,
        case: &BusCase,
        actual: &[ActualBusEvent],
    ) -> Result<Vec<Mismatch>, String> {
        let mut out = Vec::new();
        if actual.len() != case.expected.transactions.len() {
            compare_value(
                "transactions.length",
                json!(case.expected.transactions.len()),
                json!(actual.len()),
                &mut out,
            );
        }
        for (index, (expected, actual)) in case.expected.transactions.iter().zip(actual).enumerate()
        {
            compare_value(
                format!("transactions[{index}].result"),
                json!(expected.result),
                json!(actual.result),
                &mut out,
            );
            if let Some(value) = expected.value {
                compare_value(
                    format!("transactions[{index}].value"),
                    json!(value),
                    json!(actual.value),
                    &mut out,
                );
            }
        }
        self.compare_memory(&case.expected.memory, "memory", &mut out)?;
        self.compare_devices(&case.expected.devices, &mut out)?;
        Ok(out)
    }

    fn compare_memory(
        &self,
        ranges: &[MemoryRange],
        prefix: &str,
        out: &mut Vec<Mismatch>,
    ) -> Result<(), String> {
        for (index, range) in ranges.iter().enumerate() {
            let actual = self
                .machine
                .read_memory(range.address, range.bytes.len())
                .map_err(|_| "memory snapshot failed")?;
            compare_value(
                format!("{prefix}[{index}]"),
                json!(range.bytes),
                json!(actual),
                out,
            );
        }
        Ok(())
    }

    fn compare_devices(
        &self,
        expected: &[DeviceState],
        out: &mut Vec<Mismatch>,
    ) -> Result<(), String> {
        for device in expected {
            let actual = self.snapshot_device(&device.id)?;
            for (field, value) in &device.state {
                compare_value(
                    format!("devices.{}.{}", device.id, field),
                    json!(value),
                    json!(actual[field]),
                    out,
                );
            }
        }
        Ok(())
    }

    fn restore_device(&mut self, device: &DeviceState) -> Result<(), String> {
        match device.id.as_str() {
            "reporter" => {
                self.machine.restore_reporter(ReporterState {
                    status: device.state["status"],
                    case_id: device.state["case_id"],
                    detail: device.state["detail"],
                    reserved: device.state["reserved"],
                });
            }
            "high-rom" => self.machine.set_high_rom_word(device.state["word"]),
            _ => return Err(format!("unknown device `{}`", device.id)),
        }
        Ok(())
    }

    fn snapshot_device(&self, id: &str) -> Result<BTreeMap<String, u32>, String> {
        let mut state = BTreeMap::new();
        match id {
            "reporter" => {
                let reporter = self.machine.reporter();
                state.insert("status".into(), reporter.status);
                state.insert("case_id".into(), reporter.case_id);
                state.insert("detail".into(), reporter.detail);
                state.insert("reserved".into(), reporter.reserved);
            }
            "high-rom" => {
                state.insert("word".into(), self.machine.high_rom_word());
            }
            _ => return Err(format!("unknown device `{id}`")),
        }
        Ok(state)
    }
}

fn register_index(name: &str) -> Result<usize, String> {
    name.strip_prefix('r')
        .and_then(|n| n.parse().ok())
        .filter(|n| *n < 16)
        .ok_or_else(|| format!("invalid register name `{name}`"))
}

struct ActualTermination {
    kind: &'static str,
    retired: u64,
    fault: Option<Fault>,
    reporter: Option<ReporterState>,
}

impl ActualTermination {
    fn steps(retired: u64) -> Self {
        Self {
            kind: "steps",
            retired,
            fault: None,
            reporter: None,
        }
    }
    fn fault(retired: u64, fault: Fault) -> Self {
        Self {
            kind: "fault",
            retired,
            fault: Some(fault),
            reporter: None,
        }
    }
    fn reporter(retired: u64, reporter: ReporterState) -> Self {
        Self {
            kind: "reporter",
            retired,
            fault: None,
            reporter: Some(reporter),
        }
    }
    fn step_limit(retired: u64) -> Self {
        Self {
            kind: "step_limit",
            retired,
            fault: None,
            reporter: None,
        }
    }

    fn compare(&self, expected: &model::ExpectedTermination, out: &mut Vec<Mismatch>) {
        compare_value(
            "termination.kind",
            json!(expected.kind),
            json!(self.kind),
            out,
        );
        if let Some(value) = expected.retired {
            compare_value(
                "termination.retired",
                json!(value),
                json!(self.retired),
                out,
            );
        }
        if let Some(fault) = self.fault {
            let (name, pc, address, width, operation) = fault_fields(fault);
            compare_option(
                "termination.fault",
                expected.fault.as_deref(),
                Some(name),
                out,
            );
            compare_option(
                "termination.faulting_pc",
                expected.faulting_pc,
                Some(pc),
                out,
            );
            compare_option("termination.address", expected.address, address, out);
            compare_option("termination.width", expected.width, width, out);
            compare_option(
                "termination.operation",
                expected.operation.as_deref(),
                operation,
                out,
            );
        } else if expected.fault.is_some() {
            compare_option(
                "termination.fault",
                expected.fault.as_deref(),
                None::<&str>,
                out,
            );
        }
        if let Some(reporter) = self.reporter {
            let status = match reporter.status {
                1 => "pass",
                2 => "fail",
                _ => "invalid",
            };
            compare_option(
                "reporter.status",
                expected.status.as_deref(),
                Some(status),
                out,
            );
            compare_option(
                "reporter.case_id",
                expected.case_id,
                Some(reporter.case_id),
                out,
            );
            compare_option(
                "reporter.detail",
                expected.detail,
                Some(reporter.detail),
                out,
            );
        }
    }
}

fn fault_fields(
    fault: Fault,
) -> (
    &'static str,
    u32,
    Option<u32>,
    Option<u8>,
    Option<&'static str>,
) {
    match fault {
        Fault::InvalidInstruction { faulting_pc } => {
            ("INVALID_INSTRUCTION", faulting_pc, None, None, None)
        }
        Fault::UnalignedAccess {
            faulting_pc,
            addr,
            width,
            operation,
        } => (
            "UNALIGNED_ACCESS",
            faulting_pc,
            Some(addr),
            Some(width_bits(width)),
            Some(operation_name(operation)),
        ),
        Fault::UnalignedPc { faulting_pc, addr } => {
            ("UNALIGNED_PC", faulting_pc, Some(addr), None, None)
        }
        Fault::BusFault {
            faulting_pc,
            addr,
            width,
            operation,
        } => (
            "BUS_FAULT",
            faulting_pc,
            Some(addr),
            Some(width_bits(width)),
            Some(operation_name(operation)),
        ),
    }
}

fn width_bits(width: AccessWidth) -> u8 {
    match width {
        AccessWidth::Byte => 8,
        AccessWidth::Half => 16,
        AccessWidth::Word => 32,
    }
}
fn operation_name(operation: AccessOperation) -> &'static str {
    match operation {
        AccessOperation::Fetch => "fetch",
        AccessOperation::Load => "load",
        AccessOperation::Store => "store",
    }
}

fn compare_bus_events(
    expected: &[BusEvent],
    actual: &[ActualBusEvent],
    prefix: &str,
    out: &mut Vec<Mismatch>,
) {
    if expected.is_empty() {
        return;
    }
    if expected.len() != actual.len() {
        compare_value(
            format!("{prefix}.length"),
            json!(expected.len()),
            json!(actual.len()),
            out,
        );
    }
    for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        compare_value(
            format!("{prefix}[{index}].operation"),
            json!(expected.operation),
            json!(actual.operation),
            out,
        );
        compare_value(
            format!("{prefix}[{index}].address"),
            json!(expected.address),
            json!(actual.address),
            out,
        );
        compare_value(
            format!("{prefix}[{index}].width"),
            json!(expected.width),
            json!(actual.width),
            out,
        );
        compare_value(
            format!("{prefix}[{index}].result"),
            json!(expected.result),
            json!(actual.result),
            out,
        );
        if expected.value.is_some() {
            compare_value(
                format!("{prefix}[{index}].value"),
                json!(expected.value),
                json!(actual.value),
                out,
            );
        }
    }
}

fn compare_option<T: serde::Serialize>(
    path: impl Into<String>,
    expected: Option<T>,
    actual: Option<T>,
    out: &mut Vec<Mismatch>,
) {
    compare_value(path, json!(expected), json!(actual), out);
}

fn compare_value(path: impl Into<String>, expected: Value, actual: Value, out: &mut Vec<Mismatch>) {
    if expected != actual {
        out.push(Mismatch {
            path: path.into(),
            expected,
            actual,
        });
    }
}
