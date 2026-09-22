use core::fmt;
use std::collections::BTreeMap;

const OP_ADD: u8 = 0x0;
const OP_NAND: u8 = 0x1;
const OP_SHIFT: u8 = 0x2;
const OP_MUL: u8 = 0x3;
const OP_LDB: u8 = 0x4;
const OP_LDH: u8 = 0x5;
const OP_LDW: u8 = 0x7;
const OP_STB: u8 = 0x8;
const OP_STH: u8 = 0x9;
const OP_STW: u8 = 0xB;
const OP_JZ: u8 = 0xC;
const OP_CMP: u8 = 0xD;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    words: Vec<u32>,
}

impl Program {
    pub fn new(words: Vec<u32>) -> Self {
        Self { words }
    }

    pub fn words(&self) -> &[u32] {
        &self.words
    }

    pub fn into_words(self) -> Vec<u32> {
        self.words
    }

    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.words.len() * 4);
        for word in &self.words {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblerError {
    DuplicateLabel {
        line: usize,
        label: String,
    },
    UnknownLabel {
        line: usize,
        label: String,
    },
    UnknownMnemonic {
        line: usize,
        mnemonic: String,
    },
    InvalidRegister {
        line: usize,
        token: String,
    },
    InvalidImmediate {
        line: usize,
        token: String,
    },
    ImmediateOutOfRange {
        line: usize,
        value: i64,
        min: i64,
        max: i64,
    },
    WrongOperandCount {
        line: usize,
        mnemonic: String,
        expected: &'static str,
        got: usize,
    },
    InvalidLabel {
        line: usize,
        label: String,
    },
    InvalidSyntax {
        line: usize,
        message: String,
    },
}

impl fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateLabel { line, label } => {
                write!(f, "line {line}: duplicate label `{label}`")
            }
            Self::UnknownLabel { line, label } => {
                write!(f, "line {line}: unknown label `{label}`")
            }
            Self::UnknownMnemonic { line, mnemonic } => {
                write!(f, "line {line}: unknown mnemonic `{mnemonic}`")
            }
            Self::InvalidRegister { line, token } => {
                write!(f, "line {line}: invalid register `{token}`")
            }
            Self::InvalidImmediate { line, token } => {
                write!(f, "line {line}: invalid immediate `{token}`")
            }
            Self::ImmediateOutOfRange {
                line,
                value,
                min,
                max,
            } => write!(
                f,
                "line {line}: immediate `{value}` out of range ({min}..={max})"
            ),
            Self::WrongOperandCount {
                line,
                mnemonic,
                expected,
                got,
            } => write!(
                f,
                "line {line}: `{mnemonic}` expects {expected}, got {got} operand(s)"
            ),
            Self::InvalidLabel { line, label } => {
                write!(f, "line {line}: invalid label `{label}`")
            }
            Self::InvalidSyntax { line, message } => {
                write!(f, "line {line}: {message}")
            }
        }
    }
}

impl std::error::Error for AssemblerError {}

pub fn assemble(source: &str) -> Result<Program, AssemblerError> {
    let parsed = parse_source(source)?;
    let symbols = first_pass(&parsed)?;
    let words = second_pass(&parsed, &symbols)?;
    Ok(Program::new(words))
}

#[derive(Debug, Clone)]
struct ParsedLine {
    line_no: usize,
    label: Option<String>,
    tokens: Vec<String>,
}

fn parse_source(source: &str) -> Result<Vec<ParsedLine>, AssemblerError> {
    let mut out = Vec::new();
    for (index, raw_line) in source.lines().enumerate() {
        let line_no = index + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let (label, rest) = if let Some((left, right)) = line.split_once(':') {
            let label = left.trim().to_string();
            validate_label(line_no, &label)?;
            (Some(label), right.trim())
        } else {
            (None, line)
        };

        let tokens = if rest.is_empty() {
            Vec::new()
        } else {
            rest.replace(',', " ")
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect()
        };

        out.push(ParsedLine {
            line_no,
            label,
            tokens,
        });
    }
    Ok(out)
}

fn strip_comment(line: &str) -> &str {
    line.split([';', '#']).next().unwrap_or("")
}

fn validate_label(line: usize, label: &str) -> Result<(), AssemblerError> {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return Err(AssemblerError::InvalidLabel {
            line,
            label: label.to_string(),
        });
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(AssemblerError::InvalidLabel {
            line,
            label: label.to_string(),
        });
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AssemblerError::InvalidLabel {
            line,
            label: label.to_string(),
        });
    }
    Ok(())
}

fn first_pass(lines: &[ParsedLine]) -> Result<BTreeMap<String, u32>, AssemblerError> {
    let mut symbols = BTreeMap::new();
    let mut pc = 0u32;

    for line in lines {
        if let Some(label) = &line.label
            && symbols.insert(label.clone(), pc).is_some()
        {
            return Err(AssemblerError::DuplicateLabel {
                line: line.line_no,
                label: label.clone(),
            });
        }

        if !line.tokens.is_empty() {
            pc = pc.wrapping_add(4);
        }
    }

    Ok(symbols)
}

fn second_pass(
    lines: &[ParsedLine],
    symbols: &BTreeMap<String, u32>,
) -> Result<Vec<u32>, AssemblerError> {
    let mut words = Vec::new();
    let mut pc = 0u32;

    for line in lines {
        if line.tokens.is_empty() {
            continue;
        }

        let word = assemble_line(line, symbols, pc)?;
        words.push(word);
        pc = pc.wrapping_add(4);
    }

    Ok(words)
}

fn assemble_line(
    line: &ParsedLine,
    symbols: &BTreeMap<String, u32>,
    pc: u32,
) -> Result<u32, AssemblerError> {
    let mnemonic = line.tokens[0].to_ascii_uppercase();
    let ops = &line.tokens[1..];

    match mnemonic.as_str() {
        "NOP" => expect_operands(line, &mnemonic, ops, 0, "no operands")
            .map(|_| encode(OP_ADD, 0, 0, 0, 0)),
        "LI" => {
            expect_operands(line, &mnemonic, ops, 2, "rd, imm")?;
            let rd = parse_reg(line.line_no, &ops[0])?;
            let imm = parse_signed_imm16(line.line_no, &ops[1])?;
            Ok(encode(OP_ADD, rd, 0, 0, imm as u16))
        }
        "BZ" => {
            expect_operands(line, &mnemonic, ops, 2, "rs, label")?;
            let rs = parse_reg(line.line_no, &ops[0])?;
            let imm = resolve_label_imm16(line.line_no, &ops[1], symbols, pc)?;
            Ok(encode(OP_JZ, 0, rs, 0, imm))
        }
        "JMP" => {
            expect_operands(line, &mnemonic, ops, 1, "label")?;
            let imm = resolve_label_imm16(line.line_no, &ops[0], symbols, pc)?;
            Ok(encode(OP_JZ, 0, 0, 0, imm))
        }
        "CALL" => {
            expect_operands(line, &mnemonic, ops, 1, "label")?;
            let imm = resolve_label_imm16(line.line_no, &ops[0], symbols, pc)?;
            Ok(encode(OP_JZ, 15, 0, 0, imm))
        }
        "JR" => {
            if !(ops.len() == 1 || ops.len() == 2) {
                return Err(AssemblerError::WrongOperandCount {
                    line: line.line_no,
                    mnemonic,
                    expected: "rs or rs, imm",
                    got: ops.len(),
                });
            }
            let rs = parse_reg(line.line_no, &ops[0])?;
            let imm = if ops.len() == 2 {
                parse_signed_imm16(line.line_no, &ops[1])? as u16
            } else {
                0
            };
            Ok(encode(OP_JZ, 0, 0, rs, imm))
        }
        "CALLR" => {
            if !(ops.len() == 1 || ops.len() == 2) {
                return Err(AssemblerError::WrongOperandCount {
                    line: line.line_no,
                    mnemonic,
                    expected: "rs or rs, imm",
                    got: ops.len(),
                });
            }
            let rs = parse_reg(line.line_no, &ops[0])?;
            let imm = if ops.len() == 2 {
                parse_signed_imm16(line.line_no, &ops[1])? as u16
            } else {
                0
            };
            Ok(encode(OP_JZ, 15, 0, rs, imm))
        }
        "RET" => expect_operands(line, &mnemonic, ops, 0, "no operands")
            .map(|_| encode(OP_JZ, 0, 0, 15, 0)),
        "ADD" => {
            expect_operands(line, &mnemonic, ops, 4, "rd, rs1, rs2, imm")?;
            Ok(encode(
                OP_ADD,
                parse_reg(line.line_no, &ops[0])?,
                parse_reg(line.line_no, &ops[1])?,
                parse_reg(line.line_no, &ops[2])?,
                parse_signed_imm16(line.line_no, &ops[3])? as u16,
            ))
        }
        "NAND" => {
            expect_operands(line, &mnemonic, ops, 3, "rd, rs1, rs2")?;
            Ok(encode(
                OP_NAND,
                parse_reg(line.line_no, &ops[0])?,
                parse_reg(line.line_no, &ops[1])?,
                parse_reg(line.line_no, &ops[2])?,
                0,
            ))
        }
        "SHIFT" => {
            expect_operands(line, &mnemonic, ops, 4, "rd, rs1, rs2, imm")?;
            Ok(encode(
                OP_SHIFT,
                parse_reg(line.line_no, &ops[0])?,
                parse_reg(line.line_no, &ops[1])?,
                parse_reg(line.line_no, &ops[2])?,
                parse_u16(line.line_no, &ops[3])?,
            ))
        }
        "MUL" => {
            expect_operands(line, &mnemonic, ops, 3, "rd, rs1, rs2")?;
            Ok(encode(
                OP_MUL,
                parse_reg(line.line_no, &ops[0])?,
                parse_reg(line.line_no, &ops[1])?,
                parse_reg(line.line_no, &ops[2])?,
                0,
            ))
        }
        "CMP" => {
            expect_operands(line, &mnemonic, ops, 4, "rd, rs1, rs2, imm")?;
            Ok(encode(
                OP_CMP,
                parse_reg(line.line_no, &ops[0])?,
                parse_reg(line.line_no, &ops[1])?,
                parse_reg(line.line_no, &ops[2])?,
                parse_u16(line.line_no, &ops[3])?,
            ))
        }
        "LDB" | "LDH" | "LDW" | "STB" | "STH" | "STW" => {
            expect_operands(line, &mnemonic, ops, 4, "rd, rs1, rs2, imm")?;
            let opcode = match mnemonic.as_str() {
                "LDB" => OP_LDB,
                "LDH" => OP_LDH,
                "LDW" => OP_LDW,
                "STB" => OP_STB,
                "STH" => OP_STH,
                "STW" => OP_STW,
                _ => unreachable!(),
            };
            Ok(encode(
                opcode,
                parse_reg(line.line_no, &ops[0])?,
                parse_reg(line.line_no, &ops[1])?,
                parse_reg(line.line_no, &ops[2])?,
                parse_signed_imm16(line.line_no, &ops[3])? as u16,
            ))
        }
        "JZ" => {
            expect_operands(line, &mnemonic, ops, 4, "rd, rs1, rs2, imm")?;
            Ok(encode(
                OP_JZ,
                parse_reg(line.line_no, &ops[0])?,
                parse_reg(line.line_no, &ops[1])?,
                parse_reg(line.line_no, &ops[2])?,
                parse_signed_imm16(line.line_no, &ops[3])? as u16,
            ))
        }
        _ => Err(AssemblerError::UnknownMnemonic {
            line: line.line_no,
            mnemonic,
        }),
    }
}

fn expect_operands(
    line: &ParsedLine,
    mnemonic: &str,
    ops: &[String],
    expected_count: usize,
    expected_text: &'static str,
) -> Result<(), AssemblerError> {
    if ops.len() != expected_count {
        return Err(AssemblerError::WrongOperandCount {
            line: line.line_no,
            mnemonic: mnemonic.to_string(),
            expected: expected_text,
            got: ops.len(),
        });
    }
    Ok(())
}

fn parse_reg(line: usize, token: &str) -> Result<u8, AssemblerError> {
    let Some(number) = token.strip_prefix(['r', 'R']) else {
        return Err(AssemblerError::InvalidRegister {
            line,
            token: token.to_string(),
        });
    };
    let index: u8 = number
        .parse()
        .map_err(|_| AssemblerError::InvalidRegister {
            line,
            token: token.to_string(),
        })?;
    if index > 15 {
        return Err(AssemblerError::InvalidRegister {
            line,
            token: token.to_string(),
        });
    }
    Ok(index)
}

fn parse_u16(line: usize, token: &str) -> Result<u16, AssemblerError> {
    let value = parse_i64(line, token)?;
    if !(0..=u16::MAX as i64).contains(&value) {
        return Err(AssemblerError::ImmediateOutOfRange {
            line,
            value,
            min: 0,
            max: u16::MAX as i64,
        });
    }
    Ok(value as u16)
}

fn parse_signed_imm16(line: usize, token: &str) -> Result<i16, AssemblerError> {
    let value = parse_i64(line, token)?;
    if !(i16::MIN as i64..=i16::MAX as i64).contains(&value) {
        return Err(AssemblerError::ImmediateOutOfRange {
            line,
            value,
            min: i16::MIN as i64,
            max: i16::MAX as i64,
        });
    }
    Ok(value as i16)
}

fn parse_i64(line: usize, token: &str) -> Result<i64, AssemblerError> {
    let token = token.trim();
    let (negative, digits) = if let Some(rest) = token.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = token.strip_prefix('+') {
        (false, rest)
    } else {
        (false, token)
    };

    let value = if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16)
    } else if let Some(bin) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        i64::from_str_radix(bin, 2)
    } else {
        digits.parse()
    }
    .map_err(|_| AssemblerError::InvalidImmediate {
        line,
        token: token.to_string(),
    })?;

    Ok(if negative { -value } else { value })
}

fn resolve_label_imm16(
    line: usize,
    label: &str,
    symbols: &BTreeMap<String, u32>,
    pc: u32,
) -> Result<u16, AssemblerError> {
    let target = *symbols
        .get(label)
        .ok_or_else(|| AssemblerError::UnknownLabel {
            line,
            label: label.to_string(),
        })?;
    let diff = i64::from(target) - i64::from(pc.wrapping_add(4));
    if diff % 4 != 0 {
        return Err(AssemblerError::InvalidSyntax {
            line,
            message: format!("label `{label}` is not 4-byte aligned relative to branch"),
        });
    }
    let imm = diff / 4;
    if !(i16::MIN as i64..=i16::MAX as i64).contains(&imm) {
        return Err(AssemblerError::ImmediateOutOfRange {
            line,
            value: imm,
            min: i16::MIN as i64,
            max: i16::MAX as i64,
        });
    }
    Ok((imm as i16) as u16)
}

fn encode(op: u8, rd: u8, rs1: u8, rs2: u8, imm: u16) -> u32 {
    ((op as u32) << 28)
        | ((rd as u32) << 24)
        | ((rs1 as u32) << 20)
        | ((rs2 as u32) << 16)
        | imm as u32
}
