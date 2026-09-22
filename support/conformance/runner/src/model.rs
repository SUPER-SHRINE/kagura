use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCase {
    pub format: String,
    pub version: u32,
    pub id: String,
    pub level: u8,
    pub binary: String,
    pub spec: Vec<String>,
    pub image: Image,
    #[serde(default)]
    pub initial: Initial,
    pub execution: Execution,
    pub expected: CpuExpected,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusCase {
    pub format: String,
    pub version: u32,
    pub id: String,
    pub spec: Vec<String>,
    #[serde(default)]
    pub initial: Initial,
    pub transactions: Vec<Transaction>,
    pub expected: BusExpected,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Image {
    #[serde(default)]
    pub load_address: u32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Initial {
    pub pc: Option<u32>,
    #[serde(default)]
    pub registers: BTreeMap<String, u32>,
    #[serde(default)]
    pub memory: Vec<MemoryRange>,
    #[serde(default)]
    pub devices: Vec<DeviceState>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryRange {
    pub address: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceState {
    pub id: String,
    pub state: BTreeMap<String, u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub until: String,
    pub steps: Option<u64>,
    pub max_steps: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuExpected {
    pub termination: ExpectedTermination,
    pub state: Option<ExpectedState>,
    #[serde(default)]
    pub bus: Vec<BusEvent>,
    #[serde(default)]
    pub devices: Vec<DeviceState>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedTermination {
    pub kind: String,
    pub retired: Option<u64>,
    pub fault: Option<String>,
    pub faulting_pc: Option<u32>,
    pub address: Option<u32>,
    pub width: Option<u8>,
    pub operation: Option<String>,
    pub status: Option<String>,
    pub case_id: Option<u32>,
    pub detail: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedState {
    pub pc: Option<u32>,
    #[serde(default)]
    pub registers: BTreeMap<String, u32>,
    #[serde(default)]
    pub memory: Vec<MemoryRange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusEvent {
    pub operation: String,
    pub address: u32,
    pub width: u8,
    pub result: String,
    pub value: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    pub operation: String,
    pub address: u32,
    pub width: u8,
    pub value: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedTransaction {
    pub result: String,
    pub value: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusExpected {
    pub transactions: Vec<ExpectedTransaction>,
    #[serde(default)]
    pub memory: Vec<MemoryRange>,
    #[serde(default)]
    pub devices: Vec<DeviceState>,
}
