use register::Register;
use memory::Memory;
use rom::{HEADER_SIZE, Rom};
use ppu::Ppu;
use apu::Apu;
use button;
use joypad;
use joypad::Joypad;
use input::Input;
use display::Display;
use audio::Audio;
use save_state::{SaveState, CpuState};

fn to_joypad_button(button: button::Button) -> joypad::Button {
	match button {
		button::Button::Joypad1A |
		button::Button::Joypad2A => joypad::Button::A,
		button::Button::Joypad1B |
		button::Button::Joypad2B => joypad::Button::B,
		button::Button::Joypad1Up |
		button::Button::Joypad2Up => joypad::Button::Up,
		button::Button::Joypad1Down |
		button::Button::Joypad2Down => joypad::Button::Down,
		button::Button::Joypad1Left |
		button::Button::Joypad2Left => joypad::Button::Left,
		button::Button::Joypad1Right |
		button::Button::Joypad2Right => joypad::Button::Right,
		button::Button::Start => joypad::Button::Start,
		button::Button::Select => joypad::Button::Select,
		_ => joypad::Button::A // dummy @TODO: Throw an error?
	}
}

/**
 * Ricoh 6502
 * Refer to https://wiki.nesdev.com/w/index.php/CPU
 */
pub struct Cpu {
	power_on: bool,

	// registers
	pc: Register<u16>,
	sp: Register<u8>,
	a: Register<u8>,
	x: Register<u8>,
	y: Register<u8>,
	p: CpuStatusRegister,

	// CPU inside RAM
	ram: Memory,

	// manage additional stall cycles eg. DMA or branch success
	stall_cycles: u16,

	input: Box<dyn Input>,

	// other devices
	ppu: Ppu,
	apu: Apu,
	joypad1: Joypad,
	joypad2: Joypad,
	rom: Rom
}

// interrupts

pub enum Interrupts {
	NMI,
	RESET,
	IRQ,
	BRK  // not interrupt but instruction
}

fn interrupt_handler_address(interrupt_type: Interrupts) -> u16 {
	match interrupt_type {
		Interrupts::NMI => 0xFFFA,
		Interrupts::RESET => 0xFFFC,
		Interrupts::IRQ => 0xFFFE,
		Interrupts::BRK => 0xFFFE
	}
}

enum InstructionTypes {
	INV,
	ADC,
	AND,
	ASL,
	BCC,
	BCS,
	BEQ,
	BIT,
	BMI,
	BNE,
	BPL,
	BRK,
	BVC,
	BVS,
	CLC,
	CLD,
	CLI,
	CLV,
	CMP,
	CPX,
	CPY,
	DEC,
	DEX,
	DEY,
	EOR,
	INC,
	INX,
	INY,
	JMP,
	JSR,
	LDA,
	LDX,
	LDY,
	LSR,
	NOP,
	ORA,
	PHA,
	PHP,
	PLA,
	PLP,
	ROL,
	ROR,
	RTI,
	RTS,
	SBC,
	SEC,
	SED,
	SEI,
	STA,
	STX,
	STY,
	TAX,
	TAY,
	TSX,
	TXA,
	TXS,
	TYA
}

fn instruction_name(instruction_type: InstructionTypes) -> &'static str {
	match instruction_type {
		InstructionTypes::INV => "inv",
		InstructionTypes::ADC => "adc",
		InstructionTypes::AND => "and",
		InstructionTypes::ASL => "asl",
		InstructionTypes::BCC => "bcc",
		InstructionTypes::BCS => "bcs",
		InstructionTypes::BEQ => "beq",
		InstructionTypes::BIT => "bit",
		InstructionTypes::BMI => "bmi",
		InstructionTypes::BNE => "bne",
		InstructionTypes::BPL => "bpl",
		InstructionTypes::BRK => "brk",
		InstructionTypes::BVC => "bvc",
		InstructionTypes::BVS => "bvs",
		InstructionTypes::CLC => "clc",
		InstructionTypes::CLD => "cld",
		InstructionTypes::CLI => "cli",
		InstructionTypes::CLV => "clv",
		InstructionTypes::CMP => "cmp",
		InstructionTypes::CPX => "cpx",
		InstructionTypes::CPY => "cpy",
		InstructionTypes::DEC => "dec",
		InstructionTypes::DEX => "dex",
		InstructionTypes::DEY => "dey",
		InstructionTypes::EOR => "eor",
		InstructionTypes::INC => "inc",
		InstructionTypes::INX => "inx",
		InstructionTypes::INY => "iny",
		InstructionTypes::JMP => "jmp",
		InstructionTypes::JSR => "jsr",
		InstructionTypes::LDA => "lda",
		InstructionTypes::LDX => "ldx",
		InstructionTypes::LDY => "ldy",
		InstructionTypes::LSR => "lsr",
		InstructionTypes::NOP => "nop",
		InstructionTypes::ORA => "qra",
		InstructionTypes::PHA => "pha",
		InstructionTypes::PHP => "php",
		InstructionTypes::PLA => "pla",
		InstructionTypes::PLP => "plp",
		InstructionTypes::ROL => "rol",
		InstructionTypes::ROR => "ror",
		InstructionTypes::RTI => "rti",
		InstructionTypes::RTS => "rts",
		InstructionTypes::SBC => "sbc",
		InstructionTypes::SEC => "sec",
		InstructionTypes::SED => "sed",
		InstructionTypes::SEI => "sei",
		InstructionTypes::STA => "sta",
		InstructionTypes::STX => "stx",
		InstructionTypes::STY => "sty",
		InstructionTypes::TAX => "tax",
		InstructionTypes::TAY => "tay",
		InstructionTypes::TSX => "tsx",
		InstructionTypes::TXA => "txa",
		InstructionTypes::TXS => "txs",
		InstructionTypes::TYA => "tya"
	}
}

enum AddressingModes {
	Immediate,
	Absolute,
	IndexedAbsoluteX,
	IndexedAbsoluteY,
	ZeroPage,
	IndexedZeroPageX,
	IndexedZeroPageY,
	Implied,
	Accumulator,
	Indirect,
	IndexedIndirectX,
	IndexedIndirectY,
	Relative
}

// Compact operation encoding: [instruction_type: u8, cycle: u8, addressing_mode: u8]
// This avoids constructing Operation structs at runtime
struct Operation {
	instruction_type: InstructionTypes,
	cycle: u8,
	addressing_mode: AddressingModes
}

// Static opcode lookup table for O(1) decoding
// Format: (instruction_type, cycle, addressing_mode) encoded as bytes
static OPCODE_TABLE: [(u8, u8, u8); 256] = [
	// 0x00-0x0F
	(12, 7, 7),  // 0x00 BRK Implied
	(34, 6, 10), // 0x01 ORA IndexedIndirectX
	(0, 1, 0),   // 0x02 INV
	(0, 1, 0),   // 0x03 INV
	(0, 1, 0),   // 0x04 INV
	(34, 3, 4),  // 0x05 ORA ZeroPage
	(3, 5, 4),   // 0x06 ASL ZeroPage
	(0, 1, 0),   // 0x07 INV
	(35, 3, 7),  // 0x08 PHP Implied
	(34, 2, 0),  // 0x09 ORA Immediate
	(3, 2, 8),   // 0x0A ASL Accumulator
	(0, 1, 0),   // 0x0B INV
	(0, 1, 0),   // 0x0C INV
	(34, 4, 1),  // 0x0D ORA Absolute
	(3, 6, 1),   // 0x0E ASL Absolute
	(0, 1, 0),   // 0x0F INV
	// 0x10-0x1F
	(11, 2, 12), // 0x10 BPL Relative
	(34, 5, 11), // 0x11 ORA IndexedIndirectY
	(0, 1, 0),   // 0x12 INV
	(0, 1, 0),   // 0x13 INV
	(0, 1, 0),   // 0x14 INV
	(34, 4, 5),  // 0x15 ORA IndexedZeroPageX
	(3, 6, 5),   // 0x16 ASL IndexedZeroPageX
	(0, 1, 0),   // 0x17 INV
	(15, 2, 7),  // 0x18 CLC Implied
	(34, 4, 3),  // 0x19 ORA IndexedAbsoluteY
	(0, 1, 0),   // 0x1A INV
	(0, 1, 0),   // 0x1B INV
	(0, 1, 0),   // 0x1C INV
	(34, 4, 2),  // 0x1D ORA IndexedAbsoluteX
	(3, 7, 2),   // 0x1E ASL IndexedAbsoluteX
	(0, 1, 0),   // 0x1F INV
	// 0x20-0x2F
	(28, 6, 1),  // 0x20 JSR Absolute
	(2, 6, 10),  // 0x21 AND IndexedIndirectX
	(0, 1, 0),   // 0x22 INV
	(0, 1, 0),   // 0x23 INV
	(10, 3, 4),  // 0x24 BIT ZeroPage
	(2, 3, 4),   // 0x25 AND ZeroPage
	(37, 5, 4),  // 0x26 ROL ZeroPage
	(0, 1, 0),   // 0x27 INV
	(36, 4, 7),  // 0x28 PLP Implied
	(2, 2, 0),   // 0x29 AND Immediate
	(37, 2, 8),  // 0x2A ROL Accumulator
	(0, 1, 0),   // 0x2B INV
	(10, 4, 1),  // 0x2C BIT Absolute
	(2, 4, 1),   // 0x2D AND Absolute
	(37, 6, 1),  // 0x2E ROL Absolute
	(0, 1, 0),   // 0x2F INV
	// 0x30-0x3F
	(9, 2, 12),  // 0x30 BMI Relative
	(2, 5, 11),  // 0x31 AND IndexedIndirectY
	(0, 1, 0),   // 0x32 INV
	(0, 1, 0),   // 0x33 INV
	(0, 1, 0),   // 0x34 INV
	(2, 4, 5),   // 0x35 AND IndexedZeroPageX
	(37, 6, 5),  // 0x36 ROL IndexedZeroPageX
	(0, 1, 0),   // 0x37 INV
	(40, 2, 7),  // 0x38 SEC Implied
	(2, 4, 3),   // 0x39 AND IndexedAbsoluteY
	(0, 1, 0),   // 0x3A INV
	(0, 1, 0),   // 0x3B INV
	(0, 1, 0),   // 0x3C INV
	(2, 4, 2),   // 0x3D AND IndexedAbsoluteX
	(37, 7, 2),  // 0x3E ROL IndexedAbsoluteX
	(0, 1, 0),   // 0x3F INV
	// 0x40-0x4F
	(39, 6, 7),  // 0x40 RTI Implied
	(24, 6, 10), // 0x41 EOR IndexedIndirectX
	(0, 1, 0),   // 0x42 INV
	(0, 1, 0),   // 0x43 INV
	(0, 1, 0),   // 0x44 INV
	(24, 3, 4),  // 0x45 EOR ZeroPage
	(32, 5, 4),  // 0x46 LSR ZeroPage
	(0, 1, 0),   // 0x47 INV
	(34, 3, 7),  // 0x48 PHA Implied - note: reusing ORA code, but it's actually PHA
	(24, 2, 0),  // 0x49 EOR Immediate
	(32, 2, 8),  // 0x4A LSR Accumulator
	(0, 1, 0),   // 0x4B INV
	(27, 3, 1),  // 0x4C JMP Absolute
	(24, 4, 1),  // 0x4D EOR Absolute
	(32, 6, 1),  // 0x4E LSR Absolute
	(0, 1, 0),   // 0x4F INV
	// 0x50-0x5F
	(13, 2, 12), // 0x50 BVC Relative
	(24, 5, 11), // 0x51 EOR IndexedIndirectY
	(0, 1, 0),   // 0x52 INV
	(0, 1, 0),   // 0x53 INV
	(0, 1, 0),   // 0x54 INV
	(24, 4, 5),  // 0x55 EOR IndexedZeroPageX
	(32, 6, 5),  // 0x56 LSR IndexedZeroPageX
	(0, 1, 0),   // 0x57 INV
	(17, 2, 7),  // 0x58 CLI Implied
	(24, 4, 3),  // 0x59 EOR IndexedAbsoluteY
	(0, 1, 0),   // 0x5A INV
	(0, 1, 0),   // 0x5B INV
	(0, 1, 0),   // 0x5C INV
	(24, 4, 2),  // 0x5D EOR IndexedAbsoluteX
	(32, 7, 2),  // 0x5E LSR IndexedAbsoluteX
	(0, 1, 0),   // 0x5F INV
	// 0x60-0x6F
	(38, 6, 7),  // 0x60 RTS Implied
	(1, 6, 10),  // 0x61 ADC IndexedIndirectX
	(0, 1, 0),   // 0x62 INV
	(0, 1, 0),   // 0x63 INV
	(0, 1, 0),   // 0x64 INV
	(1, 3, 4),   // 0x65 ADC ZeroPage
	(38, 5, 4),  // 0x66 ROR ZeroPage - note: code 38 is RTS, this should be ROR
	(0, 1, 0),   // 0x67 INV
	(36, 4, 7),  // 0x68 PLA Implied - note: reusing PLP code
	(1, 2, 0),   // 0x69 ADC Immediate
	(38, 2, 8),  // 0x6A ROR Accumulator - note: code 38 is RTS
	(0, 1, 0),   // 0x6B INV
	(27, 5, 9),  // 0x6C JMP Indirect
	(1, 4, 1),   // 0x6D ADC Absolute
	(38, 6, 1),  // 0x6E ROR Absolute
	(0, 1, 0),   // 0x6F INV
	// 0x70-0x7F
	(14, 2, 12), // 0x70 BVS Relative
	(1, 5, 11),  // 0x71 ADC IndexedIndirectY
	(0, 1, 0),   // 0x72 INV
	(0, 1, 0),   // 0x73 INV
	(0, 1, 0),   // 0x74 INV
	(1, 4, 5),   // 0x75 ADC IndexedZeroPageX
	(38, 6, 5),  // 0x76 ROR IndexedZeroPageX
	(0, 1, 0),   // 0x77 INV
	(42, 2, 7),  // 0x78 SEI Implied
	(1, 4, 3),   // 0x79 ADC IndexedAbsoluteY
	(0, 1, 0),   // 0x7A INV
	(0, 1, 0),   // 0x7B INV
	(0, 1, 0),   // 0x7C INV
	(1, 4, 2),   // 0x7D ADC IndexedAbsoluteX
	(38, 7, 2),  // 0x7E ROR IndexedAbsoluteX
	(0, 1, 0),   // 0x7F INV
	// 0x80-0x8F
	(0, 1, 0),   // 0x80 INV
	(43, 6, 10), // 0x81 STA IndexedIndirectX
	(0, 1, 0),   // 0x82 INV
	(0, 1, 0),   // 0x83 INV
	(45, 3, 4),  // 0x84 STY ZeroPage
	(43, 3, 4),  // 0x85 STA ZeroPage
	(44, 3, 4),  // 0x86 STX ZeroPage
	(0, 1, 0),   // 0x87 INV
	(23, 2, 7),  // 0x88 DEY Implied
	(0, 1, 0),   // 0x89 INV
	(48, 2, 7),  // 0x8A TXA Implied
	(0, 1, 0),   // 0x8B INV
	(45, 4, 1),  // 0x8C STY Absolute
	(43, 4, 1),  // 0x8D STA Absolute
	(44, 4, 1),  // 0x8E STX Absolute
	(0, 1, 0),   // 0x8F INV
	// 0x90-0x9F
	(4, 2, 12),  // 0x90 BCC Relative
	(43, 6, 11), // 0x91 STA IndexedIndirectY
	(0, 1, 0),   // 0x92 INV
	(0, 1, 0),   // 0x93 INV
	(45, 4, 5),  // 0x94 STY IndexedZeroPageX
	(43, 4, 5),  // 0x95 STA IndexedZeroPageX
	(44, 4, 6),  // 0x96 STX IndexedZeroPageY
	(0, 1, 0),   // 0x97 INV
	(50, 2, 7),  // 0x98 TYA Implied
	(43, 5, 3),  // 0x99 STA IndexedAbsoluteY
	(49, 2, 7),  // 0x9A TXS Implied
	(0, 1, 0),   // 0x9B INV
	(0, 1, 0),   // 0x9C INV
	(43, 5, 2),  // 0x9D STA IndexedAbsoluteX
	(0, 1, 0),   // 0x9E INV
	(0, 1, 0),   // 0x9F INV
	// 0xA0-0xAF
	(31, 2, 0),  // 0xA0 LDY Immediate
	(29, 6, 10), // 0xA1 LDA IndexedIndirectX
	(30, 2, 0),  // 0xA2 LDX Immediate
	(0, 1, 0),   // 0xA3 INV
	(31, 3, 4),  // 0xA4 LDY ZeroPage
	(29, 3, 4),  // 0xA5 LDA ZeroPage
	(30, 3, 4),  // 0xA6 LDX ZeroPage
	(0, 1, 0),   // 0xA7 INV
	(47, 2, 7),  // 0xA8 TAY Implied
	(29, 2, 0),  // 0xA9 LDA Immediate
	(46, 2, 7),  // 0xAA TAX Implied
	(0, 1, 0),   // 0xAB INV
	(31, 4, 1),  // 0xAC LDY Absolute
	(29, 4, 1),  // 0xAD LDA Absolute
	(30, 4, 1),  // 0xAE LDX Absolute
	(0, 1, 0),   // 0xAF INV
	// 0xB0-0xBF
	(5, 2, 12),  // 0xB0 BCS Relative
	(29, 5, 11), // 0xB1 LDA IndexedIndirectY
	(0, 1, 0),   // 0xB2 INV
	(0, 1, 0),   // 0xB3 INV
	(31, 4, 5),  // 0xB4 LDY IndexedZeroPageX
	(29, 4, 5),  // 0xB5 LDA IndexedZeroPageX
	(30, 4, 6),  // 0xB6 LDX IndexedZeroPageY
	(0, 1, 0),   // 0xB7 INV
	(18, 2, 7),  // 0xB8 CLV Implied
	(29, 4, 3),  // 0xB9 LDA IndexedAbsoluteY
	(47, 2, 7),  // 0xBA TSX Implied - note: reusing TAY code
	(0, 1, 0),   // 0xBB INV
	(31, 4, 2),  // 0xBC LDY IndexedAbsoluteX
	(29, 4, 2),  // 0xBD LDA IndexedAbsoluteX
	(30, 4, 3),  // 0xBE LDX IndexedAbsoluteY
	(0, 1, 0),   // 0xBF INV
	// 0xC0-0xCF
	(21, 2, 0),  // 0xC0 CPY Immediate
	(19, 6, 10), // 0xC1 CMP IndexedIndirectX
	(0, 1, 0),   // 0xC2 INV
	(0, 1, 0),   // 0xC3 INV
	(21, 3, 4),  // 0xC4 CPY ZeroPage
	(19, 3, 4),  // 0xC5 CMP ZeroPage
	(22, 5, 4),  // 0xC6 DEC ZeroPage
	(0, 1, 0),   // 0xC7 INV
	(26, 2, 7),  // 0xC8 INY Implied
	(19, 2, 0),  // 0xC9 CMP Immediate
	(22, 2, 7),  // 0xCA DEX Implied - note: reusing DEC code
	(0, 1, 0),   // 0xCB INV
	(21, 4, 1),  // 0xCC CPY Absolute
	(19, 4, 1),  // 0xCD CMP Absolute
	(22, 6, 1),  // 0xCE DEC Absolute
	(0, 1, 0),   // 0xCF INV
	// 0xD0-0xDF
	(8, 2, 12),  // 0xD0 BNE Relative
	(19, 5, 11), // 0xD1 CMP IndexedIndirectY
	(0, 1, 0),   // 0xD2 INV
	(0, 1, 0),   // 0xD3 INV
	(0, 1, 0),   // 0xD4 INV
	(19, 4, 5),  // 0xD5 CMP IndexedZeroPageX
	(22, 6, 5),  // 0xD6 DEC IndexedZeroPageX
	(0, 1, 0),   // 0xD7 INV
	(16, 2, 7),  // 0xD8 CLD Implied
	(19, 4, 3),  // 0xD9 CMP IndexedAbsoluteY
	(0, 1, 0),   // 0xDA INV
	(0, 1, 0),   // 0xDB INV
	(0, 1, 0),   // 0xDC INV
	(19, 4, 2),  // 0xDD CMP IndexedAbsoluteX
	(22, 7, 2),  // 0xDE DEC IndexedAbsoluteX
	(0, 1, 0),   // 0xDF INV
	// 0xE0-0xEF
	(20, 2, 0),  // 0xE0 CPX Immediate
	(41, 6, 10), // 0xE1 SBC IndexedIndirectX
	(0, 1, 0),   // 0xE2 INV
	(0, 1, 0),   // 0xE3 INV
	(20, 3, 4),  // 0xE4 CPX ZeroPage
	(41, 3, 4),  // 0xE5 SBC ZeroPage
	(25, 5, 4),  // 0xE6 INC ZeroPage
	(0, 1, 0),   // 0xE7 INV
	(26, 2, 7),  // 0xE8 INX Implied - note: reusing INY code
	(41, 2, 0),  // 0xE9 SBC Immediate
	(33, 2, 7),  // 0xEA NOP Implied
	(0, 1, 0),   // 0xEB INV
	(20, 4, 1),  // 0xEC CPX Absolute
	(41, 4, 1),  // 0xED SBC Absolute
	(25, 6, 1),  // 0xEE INC Absolute
	(0, 1, 0),   // 0xEF INV
	// 0xF0-0xFF
	(6, 2, 12),  // 0xF0 BEQ Relative
	(41, 5, 11), // 0xF1 SBC IndexedIndirectY
	(0, 1, 0),   // 0xF2 INV
	(0, 1, 0),   // 0xF3 INV
	(0, 1, 0),   // 0xF4 INV
	(41, 4, 5),  // 0xF5 SBC IndexedZeroPageX
	(25, 6, 5),  // 0xF6 INC IndexedZeroPageX
	(0, 1, 0),   // 0xF7 INV
	(41, 2, 7),  // 0xF8 SED Implied - note: reusing SBC code
	(41, 4, 3),  // 0xF9 SBC IndexedAbsoluteY
	(0, 1, 0),   // 0xFA INV
	(0, 1, 0),   // 0xFB INV
	(0, 1, 0),   // 0xFC INV
	(41, 4, 2),  // 0xFD SBC IndexedAbsoluteX
	(25, 7, 2),  // 0xFE INC IndexedAbsoluteX
	(0, 1, 0),   // 0xFF INV
];

#[inline(always)]
fn decode_instruction_type(code: u8) -> InstructionTypes {
	match code {
		0 => InstructionTypes::INV,
		1 => InstructionTypes::ADC,
		2 => InstructionTypes::AND,
		3 => InstructionTypes::ASL,
		4 => InstructionTypes::BCC,
		5 => InstructionTypes::BCS,
		6 => InstructionTypes::BEQ,
		7 => InstructionTypes::BIT, // Not used directly - special cases below
		8 => InstructionTypes::BNE,
		9 => InstructionTypes::BMI,
		10 => InstructionTypes::BIT,
		11 => InstructionTypes::BPL,
		12 => InstructionTypes::BRK,
		13 => InstructionTypes::BVC,
		14 => InstructionTypes::BVS,
		15 => InstructionTypes::CLC,
		16 => InstructionTypes::CLD,
		17 => InstructionTypes::CLI,
		18 => InstructionTypes::CLV,
		19 => InstructionTypes::CMP,
		20 => InstructionTypes::CPX,
		21 => InstructionTypes::CPY,
		22 => InstructionTypes::DEC,
		23 => InstructionTypes::DEY,
		24 => InstructionTypes::EOR,
		25 => InstructionTypes::INC,
		26 => InstructionTypes::INY,
		27 => InstructionTypes::JMP,
		28 => InstructionTypes::JSR,
		29 => InstructionTypes::LDA,
		30 => InstructionTypes::LDX,
		31 => InstructionTypes::LDY,
		32 => InstructionTypes::LSR,
		33 => InstructionTypes::NOP,
		34 => InstructionTypes::ORA,
		35 => InstructionTypes::PHP,
		36 => InstructionTypes::PLP,
		37 => InstructionTypes::ROL,
		38 => InstructionTypes::ROR,
		39 => InstructionTypes::RTI,
		40 => InstructionTypes::SEC,
		41 => InstructionTypes::SBC,
		42 => InstructionTypes::SEI,
		43 => InstructionTypes::STA,
		44 => InstructionTypes::STX,
		45 => InstructionTypes::STY,
		46 => InstructionTypes::TAX,
		47 => InstructionTypes::TAY,
		48 => InstructionTypes::TXA,
		49 => InstructionTypes::TXS,
		50 => InstructionTypes::TYA,
		51 => InstructionTypes::TSX,
		52 => InstructionTypes::PHA,
		53 => InstructionTypes::PLA,
		54 => InstructionTypes::RTS,
		55 => InstructionTypes::SED,
		56 => InstructionTypes::DEX,
		57 => InstructionTypes::INX,
		_ => InstructionTypes::INV,
	}
}

#[inline(always)]
fn decode_addressing_mode(code: u8) -> AddressingModes {
	match code {
		0 => AddressingModes::Immediate,
		1 => AddressingModes::Absolute,
		2 => AddressingModes::IndexedAbsoluteX,
		3 => AddressingModes::IndexedAbsoluteY,
		4 => AddressingModes::ZeroPage,
		5 => AddressingModes::IndexedZeroPageX,
		6 => AddressingModes::IndexedZeroPageY,
		7 => AddressingModes::Implied,
		8 => AddressingModes::Accumulator,
		9 => AddressingModes::Indirect,
		10 => AddressingModes::IndexedIndirectX,
		11 => AddressingModes::IndexedIndirectY,
		12 => AddressingModes::Relative,
		_ => AddressingModes::Immediate,
	}
}

// Fast O(1) opcode lookup using static table
#[inline(always)]
fn operation(opc: u8) -> Operation {
	let (inst, cyc, addr) = OPCODE_TABLE[opc as usize];

	// Handle special cases that the table encoding doesn't capture perfectly
	let instruction_type = match opc {
		0x48 => InstructionTypes::PHA,
		0x68 => InstructionTypes::PLA,
		0x60 => InstructionTypes::RTS,
		0x66 | 0x6A | 0x6E | 0x76 | 0x7E => InstructionTypes::ROR,
		0xBA => InstructionTypes::TSX,
		0xCA => InstructionTypes::DEX,
		0xE8 => InstructionTypes::INX,
		0xF8 => InstructionTypes::SED,
		_ => decode_instruction_type(inst),
	};

	Operation {
		instruction_type,
		cycle: cyc,
		addressing_mode: decode_addressing_mode(addr),
	}
}

impl Cpu {
	pub fn new(input: Box<dyn Input>, display: Box<dyn Display>, audio: Box<dyn Audio>) -> Self {
		Cpu {
			power_on: false,
			pc: Register::<u16>::new(),
			sp: Register::<u8>::new(),
			a: Register::<u8>::new(),
			x: Register::<u8>::new(),
			y: Register::<u8>::new(),
			p: CpuStatusRegister::new(),
			ram: Memory::new(vec![0; 64 * 1024]), // 64KB
			stall_cycles: 0,
			input: input,
			ppu: Ppu::new(display),
			apu: Apu::new(audio),
			joypad1: Joypad::new(),
			joypad2: Joypad::new(),
			rom: Rom::new(vec![0; HEADER_SIZE]).unwrap() // dummy
		}
	}

	pub fn set_rom(&mut self, rom: Rom) {
		self.rom = rom;
	}

	pub fn bootup(&mut self) {
		self.power_on = true;
		self.bootup_internal();
		self.ppu.bootup();
		self.apu.bootup();
	}

	fn bootup_internal(&mut self) {
		self.p.store(0x34);
		self.a.clear();
		self.x.clear();
		self.y.clear();
		self.sp.store(0xFD);

		for i in 0..0x10 {
			self.store(0x4000 + i, 0);
		}

		self.store(0x4015, 0);
		self.store(0x4017, 0);

		self.interrupt(Interrupts::RESET);
	}

	pub fn reset(&mut self) {
		self.reset_internal();
		self.ppu.reset();
		self.apu.reset();
		self.interrupt(Interrupts::RESET);
	}

	pub fn is_power_on(&self) -> bool {
		self.power_on
	}

	fn reset_internal(&mut self) {
		self.sp.sub(3);
		self.p.set_i();
	}

	pub fn get_ppu(&self) -> &Ppu {
		&self.ppu
	}

	pub fn get_mut_apu(&mut self) -> &mut Apu {
		&mut self.apu
	}

	pub fn get_mut_input(&mut self) -> &mut Box<dyn Input> {
		&mut self.input
	}

	//

	#[inline]
	pub fn step(&mut self) {
		let stall_cycles = self.step_internal();
		for _i in 0..stall_cycles * 3 {
			self.ppu.step(&mut self.rom);
		}
		for _i in 0..stall_cycles {
			// No reference to CPU from APU so detecting if APU DMC needs
			// CPU memory data, loading data, and sending to APU if needed
			// @TODO: Simplify
			let dmc_sample_data = match self.apu.dmc_needs_cpu_memory_data() {
				true => {
					// The CPU is stalled for up to 4 CPU cycles
					// @TODO: Fix me
					self.stall_cycles += 4;
					self.load(self.apu.dmc_sample_address())
				}
				false => 0
			};
			self.apu.step(dmc_sample_data);
		}
	}

	pub fn step_frame(&mut self) {
		// Input handling should be here? Or under nes.rs?
		self.handle_inputs();
		// @TODO: More precise frame update detection?
		let ppu_frame = self.ppu.frame;
		loop {
			self.step();
			if ppu_frame != self.ppu.frame {
				break;
			}
		}
	}

	fn handle_inputs(&mut self) {
		while let Some((button, event)) = self.input.get_input() {
			match button {
				button::Button::Poweroff => {
					self.power_on = false;
				},
				button::Button::Reset => {
					self.reset();
				},
				button::Button::Select |
				button::Button::Start |
				button::Button::Joypad1A |
				button::Button::Joypad1B |
				button::Button::Joypad1Up |
				button::Button::Joypad1Down |
				button::Button::Joypad1Left |
				button::Button::Joypad1Right => {
					self.joypad1.handle_input(to_joypad_button(button), event);
				},
				button::Button::Joypad2A |
				button::Button::Joypad2B |
				button::Button::Joypad2Up |
				button::Button::Joypad2Down |
				button::Button::Joypad2Left |
				button::Button::Joypad2Right => {
					self.joypad2.handle_input(to_joypad_button(button), event);
				},
                button::Button::X |
                button::Button::Y |
                button::Button::L |
                button::Button::R => {
                    // Do nothing for NES
                }
			}
		}
	}

	#[inline]
	fn step_internal(&mut self) -> u16 {
		// @TODO: What if both NMI and IRQ happen?
		if self.ppu.nmi_interrupted {
			self.ppu.nmi_interrupted = false;
			self.interrupt(Interrupts::NMI);
		}
		if self.ppu.irq_interrupted {
			self.ppu.irq_interrupted = false;
			self.interrupt(Interrupts::IRQ);
		}
		if self.apu.irq_interrupted {
			self.apu.irq_interrupted = false;
			self.interrupt(Interrupts::IRQ);
		}

		let opc = self.fetch();
		let op = self.decode(opc);
		self.operate(&op);
		let stall_cycles = self.stall_cycles;
		self.stall_cycles = 0;
		stall_cycles + op.cycle as u16
	}

	#[inline(always)]
	fn fetch(&mut self) -> u8 {
		let opc = self.load(self.pc.load());
		self.pc.increment();
		opc
	}

	#[inline(always)]
	fn decode(&self, opc: u8) -> Operation {
		operation(opc)
	}

	fn jump_to_interrupt_handler(&mut self, interrupt_type: Interrupts) {
		let address = interrupt_handler_address(interrupt_type);
		let value = self.load_2bytes(address);
		self.pc.store(value);
	}

	fn do_branch(&mut self, op: &Operation, flag: bool) {
		let result = self.load_with_addressing_mode(&op);
		if flag {
			// stall_cycle + 1 if branch succeeds
			self.stall_cycles += 1;
			let current_page = self.pc.load() & 0xff00;
			self.pc.add(result);
			if current_page != (self.pc.load() & 0xff00) {
				// stall_cycle + 1 if across page
				self.stall_cycles += 1;
			}
		}
	}

	// @TODO: Clean up if needed
	fn operate(&mut self, op: &Operation) {
		match op.instruction_type {
			InstructionTypes::ADC => {
				let src1 = self.a.load();
				let src2 = self.load_with_addressing_mode(&op);
				let c = match self.p.is_c() {
					true => 1,
					false => 0
				} as u16;
				let result = (src1 as u16).wrapping_add(src2).wrapping_add(c);
				self.a.store(result as u8);
				self.update_n(result);
				self.update_z(result);
				self.update_c(result);
				if !(((src1 ^ src2 as u8) & 0x80) != 0) && ((src2 as u8 ^ result as u8) & 0x80) != 0 {
					self.p.set_v();
				} else {
					self.p.clear_v();
				}
			},
			InstructionTypes::AND => {
				let src1 = self.a.load();
				let src2 = self.load_with_addressing_mode(&op);
				let result = (src1 as u16) & src2;
				self.a.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::ASL => {
				let result = self.update_memory_with_addressing_mode(op, |src: u8| {
					(src as u16) << 1
				});
				self.update_n(result);
				self.update_z(result);
				self.update_c(result);
			},
			InstructionTypes::BCC => {
				let flag = !self.p.is_c();
				self.do_branch(&op, flag);
			},
			InstructionTypes::BCS => {
				let flag = self.p.is_c();
				self.do_branch(&op, flag);
			},
			InstructionTypes::BEQ => {
				let flag = self.p.is_z();
				self.do_branch(&op, flag);
			},
			// @TODO: check logic
			InstructionTypes::BIT => {
				let src1 = self.a.load();
				let src2 = self.load_with_addressing_mode(&op);
				let result = (src1 as u16) & src2;
				self.update_n(src2);
				self.update_z(result);
				if (src2 & 0x40) == 0 {
					self.p.clear_v();
				} else {
					self.p.set_v();
				}
			},
			InstructionTypes::BMI => {
				let flag = self.p.is_n();
				self.do_branch(&op, flag);
			},
			InstructionTypes::BNE => {
				let flag = !self.p.is_z();
				self.do_branch(&op, flag);
			},
			InstructionTypes::BPL => {
				let flag = !self.p.is_n();
				self.do_branch(&op, flag);
			},
			InstructionTypes::BRK => {
				self.pc.increment(); // seems like necessary
				self.p.set_a();
				self.p.set_b();
				self.interrupt(Interrupts::BRK);
			},
			InstructionTypes::BVC => {
				let flag = !self.p.is_v();
				self.do_branch(&op, flag);
			},
			InstructionTypes::BVS => {
				let flag = self.p.is_v();
				self.do_branch(&op, flag);
			},
			InstructionTypes::CLC => {
				self.p.clear_c();
			},
			InstructionTypes::CLD => {
				self.p.clear_d();
			},
			InstructionTypes::CLI => {
				self.p.clear_i();
			},
			InstructionTypes::CLV => {
				self.p.clear_v();
			},
			InstructionTypes::CMP | InstructionTypes::CPX | InstructionTypes::CPY => {
				let src1 = match op.instruction_type {
					InstructionTypes::CMP => {
						self.a.load()
					},
					InstructionTypes::CPX => {
						self.x.load()
					},
					_ => { //InstructionTypes::CPY
						self.y.load()
					}
				};
				let src2 = self.load_with_addressing_mode(&op);
				let result = (src1 as u16).wrapping_sub(src2);
				self.update_n(result);
				self.update_z(result);
				if src1 as u16 >= src2 {
					self.p.set_c();
				} else {
					self.p.clear_c();
				}
			},
			InstructionTypes::DEC => {
				let result = self.update_memory_with_addressing_mode(op, |src: u8| {
					(src as u16).wrapping_sub(1)
				});
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::DEX | InstructionTypes::DEY => {
				let result = match op.instruction_type {
					InstructionTypes::DEX => {
						let src = self.x.load();
						let result = (src as u16).wrapping_sub(1);
						self.x.store(result as u8);
						result
					},
					_ => { // InstructionTypes::DEY
						let src = self.y.load();
						let result = (src as u16).wrapping_sub(1);
						self.y.store(result as u8);
						result
					}
				};
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::EOR => {
				let src1 = self.a.load();
				let src2 = self.load_with_addressing_mode(&op);
				let result = (src1 as u16) ^ src2;
				self.a.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::INC => {
				let result = self.update_memory_with_addressing_mode(op, |src: u8| {
					(src as u16).wrapping_add(1)
				});
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::INV => {
				// @TODO: Throw?
				println!("INV operation");
			},
			InstructionTypes::INX | InstructionTypes::INY => {
				let result = match op.instruction_type {
					InstructionTypes::INX => {
						let src = self.x.load();
						let result = (src as u16).wrapping_add(1);
						self.x.store(result as u8);
						result
					},
					_ => { // InstructionTypes::INY
						let src = self.y.load();
						let result = (src as u16).wrapping_add(1);
						self.y.store(result as u8);
						result
					}
				};
				self.update_n(result);
				self.update_z(result);
			},
			// TODO: check the logic.
			InstructionTypes::JMP => {
				let address = self.get_address_with_addressing_mode(op);
				self.pc.store(address);
			},
			// TODO: check the logic.
			InstructionTypes::JSR => {
				let address = self.get_address_with_addressing_mode(op);
				self.pc.decrement();
				let value = self.pc.load();
				self.push_stack_2bytes(value);
				self.pc.store(address);
			},
			InstructionTypes::LDA | InstructionTypes::LDX | InstructionTypes::LDY => {
				let result = match op.instruction_type {
					InstructionTypes::LDA => {
						let result = self.load_with_addressing_mode(&op);
						self.a.store(result as u8);
						result
					},
					InstructionTypes::LDX => {
						let result = self.load_with_addressing_mode(&op);
						self.x.store(result as u8);
						result
					},
					_ /*InstructionTypes::LDY*/ => {
						let result = self.load_with_addressing_mode(&op);
						self.y.store(result as u8);
						result
					}
				};
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::LSR => {
				let result = match op.addressing_mode {
					AddressingModes::Accumulator => {
						let src = self.a.load();
						if (src & 1) == 0 {
							self.p.clear_c();
						} else {
							self.p.set_c();
						}
						let result = (src as u16) >> 1;
						self.a.store(result as u8);
						result
					},
					_ => {
						let address = self.get_address_with_addressing_mode(op);
						let src = self.load(address);
						if (src & 1) == 0 {
							self.p.clear_c();
						} else {
							self.p.set_c();
						}
						let result = (src as u16) >> 1;
						self.store(address, result as u8);
						result
					}
				};
				self.p.clear_n();
				self.update_z(result);
			},
			InstructionTypes::NOP => {},
			InstructionTypes::ORA => {
				let src1 = self.a.load();
				let src2 = self.load_with_addressing_mode(op);
				let result = (src1 as u16) | src2;
				self.a.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::PHA => {
				let value = self.a.load();
				self.push_stack(value);
			},
			InstructionTypes::PHP => {
				self.p.set_a();
				self.p.set_b();
				let value = self.p.load();
				self.push_stack(value);
			},
			InstructionTypes::PLA => {
				let result = self.pop_stack() as u16;
				self.a.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::PLP => {
				let value = self.pop_stack();
				self.p.store(value);
			},
			InstructionTypes::ROL => {
				let result = match op.addressing_mode {
					AddressingModes::Accumulator => {
						let src = self.a.load();
						let c = match self.p.is_c() {
							true => 1,
							false => 0
						} as u16;
						let result = ((src as u16) << 1) | c;
						self.a.store(result as u8);
						result
					},
					_ => {
						let address = self.get_address_with_addressing_mode(op);
						let src = self.load(address);
						let c = match self.p.is_c() {
							true => 1,
							false => 0
						} as u16;
						let result = ((src as u16) << 1) | c;
						self.store(address, result as u8);
						result
					}
				};
				self.update_n(result);
				self.update_z(result);
				self.update_c(result);
			},
			InstructionTypes::ROR => {
				let result = match op.addressing_mode {
					AddressingModes::Accumulator => {
						let src = self.a.load();
						let c = match self.p.is_c() {
							true => 0x80,
							false => 0
						} as u16;
						let result = ((src as u16) >> 1) | c;
						self.a.store(result as u8);
						if (src & 1) == 0 {
							self.p.clear_c();
						} else {
							self.p.set_c();
						}
						result
					},
					_ => {
						let address = self.get_address_with_addressing_mode(op);
						let src = self.load(address);
						let c = match self.p.is_c() {
							true => 0x80,
							false => 0
						} as u16;
						let result = ((src as u16) >> 1) | c;
						self.store(address, result as u8);
						if (src & 1) == 0 {
							self.p.clear_c();
						} else {
							self.p.set_c();
						}
						result
					}
				};
				self.update_n(result);
				self.update_z(result);
			},
			// TODO: check logic.
			InstructionTypes::RTI => {
				let value = self.pop_stack();
				self.p.store(value);
				let value2 = self.pop_stack_2bytes();
				self.pc.store(value2);
			},
			// TODO: check logic.
			InstructionTypes::RTS => {
				let value = self.pop_stack_2bytes().wrapping_add(1);
				self.pc.store(value);
			},
			InstructionTypes::SBC => {
				let src1 = self.a.load();
				let src2 = self.load_with_addressing_mode(&op);
				let c = match self.p.is_c() {
					true => 0,
					false => 1
				} as u16;
				let result = (src1 as u16).wrapping_sub(src2).wrapping_sub(c);
				self.a.store(result as u8);
				self.update_n(result);
				self.update_z(result);
				// TODO: check if this logic is right.
				if src1 as u16 >= src2.wrapping_add(c) {
					self.p.set_c();
				} else {
					self.p.clear_c();
				}
				// TODO: implement right overflow logic.
				//       this is just a temporal logic.
				if ((src1 ^ result as u8) & 0x80) != 0 && ((src1 ^ src2 as u8) & 0x80) != 0 {
					self.p.set_v();
				} else {
					self.p.clear_v();
				}
			},
			InstructionTypes::SEC => {
				self.p.set_c();
			},
			InstructionTypes::SED => {
				self.p.set_d();
			},
			InstructionTypes::SEI => {
				self.p.set_i();
			},
			InstructionTypes::STA => {
				let value = self.a.load();
				self.store_with_addressing_mode(&op, value);
			},
			InstructionTypes::STX => {
				let value = self.x.load();
				self.store_with_addressing_mode(&op, value);
			},
			InstructionTypes::STY => {
				let value = self.y.load();
				self.store_with_addressing_mode(&op, value);
			},
			InstructionTypes::TAX => {
				let result = self.a.load() as u16;
				self.x.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::TAY => {
				let result = self.a.load() as u16;
				self.y.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::TSX => {
				let result = self.sp.load() as u16;
				self.x.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::TXA => {
				let result = self.x.load() as u16;
				self.a.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			},
			InstructionTypes::TXS => {
				let result = self.x.load();
				self.sp.store(result);
			},
			InstructionTypes::TYA => {
				let result = self.y.load() as u16;
				self.a.store(result as u8);
				self.update_n(result);
				self.update_z(result);
			}
		}
	}

	#[inline]
	pub fn load(&mut self, address: u16) -> u8 {
		// 0x0000 - 0x07FF: 2KB internal RAM
		// 0x0800 - 0x1FFF: Mirrors of 0x0000 - 0x07FF (repeats every 0x800 bytes)

		if address < 0x2000 {
			return self.ram.load((address & 0x07FF) as u32);
		}

		// 0x2000 - 0x2007: PPU registers
		// 0x2008 - 0x3FFF: Mirrors of 0x2000 - 0x2007 (repeats every 8 bytes)

		if address >= 0x2000 && address < 0x4000 {
			return self.ppu.load_register(address & 0x2007, &self.rom);
		}

		if address >= 0x4000 && address < 0x4014 {
			return self.apu.load_register(address);
		}

		if address == 0x4014 {
			return self.ppu.load_register(address, &self.rom);
		}

		if address == 0x4015 {
			return self.apu.load_register(address);
		}

		if address == 0x4016 {
			return self.joypad1.load_register();
		}

		if address == 0x4017 {
			return self.joypad2.load_register();
		}

		if address >= 0x4017 && address < 0x4020 {
			return self.apu.load_register(address);
		}

		if address >= 0x4020 && address < 0x6000 {
			return self.ram.load(address as u32);
		}

		if address >= 0x6000 && address < 0x8000 {
			return self.ram.load(address as u32);
		}

		if address >= 0x8000 {
			return self.rom.load(address as u32);
		}

		0 // dummy
	}

	fn load_2bytes(&mut self, address: u16) -> u16 {
		let byte_low = self.load(address) as u16;
		let byte_high = self.load(address.wrapping_add(1)) as u16;
		(byte_high << 8) | byte_low
	}

	fn load_2bytes_from_zeropage(&mut self, address: u16) -> u16 {
		self.ram.load((address & 0xff) as u32) as u16 | ((self.ram.load((address.wrapping_add(1) & 0xff) as u32) as u16) << 8)
	}

	fn load_2bytes_in_page(&mut self, address: u16) -> u16 {
		let addr1 = address;
		let addr2 = (address & 0xff00) | ((address.wrapping_add(1)) & 0xff);
		let byte_low = self.load(addr1) as u16;
		let byte_high = self.load(addr2) as u16;
		(byte_high << 8) | byte_low
	}

	#[inline]
	fn store(&mut self, address: u16, value: u8) {
		// 0x0000 - 0x07FF: 2KB internal RAM
		// 0x0800 - 0x1FFF: Mirrors of 0x0000 - 0x07FF (repeats every 0x800 bytes)

		if address < 0x2000 {
			self.ram.store((address & 0x07FF) as u32, value);
		}

		// 0x2000 - 0x2007: PPU registers
		// 0x2008 - 0x3FFF: Mirrors of 0x2000 - 0x2007 (repeats every 8 bytes)

		if address >= 0x2000 && address < 0x4000 {
			self.ppu.store_register(address & 0x2007, value, &mut self.rom);
		}

		if address >= 0x4000 && address < 0x4014 {
			self.apu.store_register(address, value);
		}

		// @TODO: clean up

		if address == 0x4014 {
			self.ppu.store_register(address, value, &mut self.rom);

			// DMA.
			// Writing 0xXX will upload 256 bytes of data from CPU page
			// 0xXX00-0xXXFF to the internal PPU OAM.
			let offset = (value as u16) << 8;
			for i in 0..256 {
				let data = self.load(offset + i);
				self.ppu.store_register(0x2004, data, &mut self.rom);
			}

			// @TODO
			self.stall_cycles += 514;
		}

		if address == 0x4015 {
			self.apu.store_register(address, value);
		}

		if address == 0x4016 {
			self.joypad1.store_register(value);
			self.joypad2.store_register(value); // to clear the joypad2 state
		}

		if address >= 0x4017 && address < 0x4020 {
			self.apu.store_register(address, value);
		}

		// cartridge space

		if address >= 0x4020 && address < 0x6000 {
			self.ram.store(address as u32, value);
		}

		// 0x6000 - 0x7FFF: Battery Backed Save or Work RAM

		if address >= 0x6000 && address < 0x8000 {
			self.ram.store(address as u32, value);
		}

		// 0x8000 - 0xFFFF: ROM

		if address >= 0x8000 {
			self.rom.store(address as u32, value);
		}
	}

	pub fn interrupt(&mut self, interrupt_type: Interrupts) {
		// @TODO: Optimize

		match interrupt_type {
			Interrupts::IRQ => {
				if self.p.is_i() {
					return;
				}
			},
			_ => {}
		}

		match interrupt_type {
			Interrupts::RESET => {},
			_ => {
				match interrupt_type {
					Interrupts::BRK => {},
					_ => self.p.clear_b()
				};
				self.p.set_a();

				let value = self.pc.load();
				self.push_stack_2bytes(value);
				let value2 = self.p.load();
				self.push_stack(value2);
				self.p.set_i();
			}
		};

		self.jump_to_interrupt_handler(interrupt_type);
	}

	fn load_with_addressing_mode(&mut self, op: &Operation) -> u16 {
		match op.addressing_mode {
			AddressingModes::Accumulator => {
				self.a.load() as u16
			},
			_ => {
				let address = self.get_address_with_addressing_mode(&op);
				let value = self.load(address) as u16;
				match op.addressing_mode {
					// expects that relative addressing mode is used only for load.
					AddressingModes::Relative => {
						// TODO: confirm if this logic is right.
						if (value & 0x80) != 0 {
							value | 0xff00
						} else {
							value
						}
					},
					_ => value
				}
			}
		}
	}

	fn store_with_addressing_mode(&mut self, op: &Operation, value: u8) {
		match op.addressing_mode {
			AddressingModes::Accumulator => {
				self.a.store(value);
			},
			_ => {
				let address = self.get_address_with_addressing_mode(op);
				self.store(address, value);
			}
		};
	}

	fn update_memory_with_addressing_mode<F>(&mut self, op: &Operation, func: F) -> u16 where F: Fn(u8) -> u16 {
		match op.addressing_mode {
			AddressingModes::Accumulator => {
				let src = self.a.load();
				let result = func(src);
				self.a.store(result as u8);
				result
			},
			_ => {
				let address = self.get_address_with_addressing_mode(op);
				let src = self.load(address);
				let result = func(src);
				self.store(address, result as u8);
				result
			}
		}
	}

	fn get_address_with_addressing_mode(&mut self, op: &Operation) -> u16 {
		match op.addressing_mode {
			AddressingModes::Immediate | AddressingModes::Relative => {
				let address = self.pc.load();
				self.pc.increment();
				address
			},
			AddressingModes::Absolute | AddressingModes::IndexedAbsoluteX | AddressingModes::IndexedAbsoluteY => {
				let address = self.load_2bytes(self.pc.load());
				self.pc.increment_by_2();
				let effective_address = address.wrapping_add(match op.addressing_mode {
					AddressingModes::IndexedAbsoluteX => self.x.load(),
					AddressingModes::IndexedAbsoluteY => self.y.load(),
					_ => 0
				} as u16);
				match op.instruction_type {
					InstructionTypes::ADC |
					InstructionTypes::AND |
					InstructionTypes::CMP |
					InstructionTypes::EOR |
					InstructionTypes::LDA |
					InstructionTypes::LDY |
					InstructionTypes::LDX |
					InstructionTypes::ORA |
					InstructionTypes::SBC => {
						// stall_cycles + 1 if page is crossed
						if (address & 0xff00) != (effective_address & 0xff00) {
							self.stall_cycles += 1;
						}
					},
					_ => {}
				};
				effective_address
			},
			AddressingModes::ZeroPage | AddressingModes::IndexedZeroPageX | AddressingModes::IndexedZeroPageY => {
				let address = self.pc.load();
				let address2 = self.load(address) as u16;
				self.pc.increment();
				address2.wrapping_add(match op.addressing_mode {
					AddressingModes::IndexedZeroPageX => self.x.load(),
					AddressingModes::IndexedZeroPageY => self.y.load(),
					_ => 0
				} as u16) & 0xFF
			},
			AddressingModes::Indirect => {
				let address = self.pc.load();
				let tmp = self.load_2bytes(address);
				self.pc.increment_by_2();
				self.load_2bytes_in_page(tmp)
			},
			AddressingModes::IndexedIndirectX => {
				let address = self.pc.load();
				let tmp = self.load(address);
				self.pc.increment();
				self.load_2bytes_from_zeropage(((tmp.wrapping_add(self.x.load())) & 0xFF) as u16)
			},
			AddressingModes::IndexedIndirectY => {
				let address = self.pc.load();
				let tmp = self.load(address);
				self.pc.increment();
				let address2 = self.load_2bytes_from_zeropage(tmp as u16);
				let effective_address = address2.wrapping_add(self.y.load() as u16);
				match op.instruction_type {
					InstructionTypes::AND |
					InstructionTypes::CMP |
					InstructionTypes::EOR |
					InstructionTypes::LDA |
					InstructionTypes::ORA |
					InstructionTypes::SBC => {
						// stall_cycles + 1 if page is crossed
						if (address2 & 0xff00) != (effective_address & 0xff00) {
							self.stall_cycles += 1;
						}
					},
					_ => {}
				};
				effective_address
			},
			_ => {
				// @TODO: Throw?
				println!("Unknown addressing mode.");
				0
			}
		}
	}

	fn update_n(&mut self, value: u16) {
		if (value & 0x80) == 0 {
			self.p.clear_n();
		} else {
			self.p.set_n();
		}
	}

	fn update_z(&mut self, value: u16) {
		if (value & 0xff) == 0 {
			self.p.set_z();
		} else {
			self.p.clear_z();
		}
	}

	fn update_c(&mut self, value: u16) {
		if (value & 0x100) == 0 {
			self.p.clear_c();
		} else {
			self.p.set_c();
		}
	}

	fn get_stack_address(&self) -> u16 {
		self.sp.load() as u16 + 0x100
	}

	fn push_stack(&mut self, value: u8) {
		let address = self.get_stack_address();
		self.store(address, value);
		self.sp.decrement();
	}

	fn push_stack_2bytes(&mut self, value: u16) {
		let address = self.get_stack_address();
		self.store(address, ((value >> 8) & 0xff) as u8);
		self.sp.decrement();
		let address2 = self.get_stack_address();
		self.store(address2, (value & 0xff) as u8);
		self.sp.decrement();
	}

	fn pop_stack(&mut self) -> u8 {
		self.sp.increment();
		self.load(self.get_stack_address())
	}

	fn pop_stack_2bytes(&mut self) -> u16 {
		self.sp.increment();
		let byte_low = self.load(self.get_stack_address()) as u16;
		self.sp.increment();
		let byte_high = self.load(self.get_stack_address()) as u16;
		(byte_high << 8) | byte_low
	}

	pub fn dump(&mut self) -> String {
		let opc = self.load(self.pc.load());
		let op = self.decode(opc);
		"p:".to_owned() + &self.p.dump() + &" ".to_owned() +
		&"pc:".to_owned() + &self.pc.dump() + &format!("(0x{:02x})", opc) + &" ".to_owned() +
		&"sp:".to_owned() + &self.sp.dump() + &" ".to_owned() +
		&"a:".to_owned() + &self.a.dump() + &" ".to_owned() +
		&"x:".to_owned() + &self.x.dump() + &" ".to_owned() +
		&"y:".to_owned() + &self.y.dump() + &" ".to_owned() +
		instruction_name(op.instruction_type) + &" ".to_owned() +
		&self.dump_addressing_mode(op.addressing_mode, self.pc.load().wrapping_add(1))
	}

	fn dump_addressing_mode(&mut self, mode: AddressingModes, pc: u16) -> String {
		match mode {
			AddressingModes::Immediate => {
				"#".to_owned() + &format!("0x{:02x} ", self.load(pc)) +
				&"immediate".to_owned()
			},
			AddressingModes::Relative => {
				format!("0x{:02x} ", self.load(pc) as i8) +
				&"relative".to_owned()
			},
			AddressingModes::Absolute => {
				let address = self.load_2bytes(pc);
				format!("0x{:04x} ", address) +
				&format!("(0x{:02x}) ", self.load(address) as i8) +
				&"absolute".to_owned()
			},
			AddressingModes::IndexedAbsoluteX => {
				let address = self.load_2bytes(pc);
				format!("0x{:04x},X ", address) +
				&format!("(0x{:02x}) ", self.load((self.x.load() as u16).wrapping_add(address)) as i8) +
				&"indexed_absolute_x".to_owned()
			},
			AddressingModes::IndexedAbsoluteY => {
				let address = self.load_2bytes(pc);
				format!("0x{:04x},Y ", address) +
				&format!("(0x{:02x}) ", self.load((self.y.load() as u16).wrapping_add(address)) as i8) +
				&"indexed_absolute_y".to_owned()
			},
			AddressingModes::ZeroPage => {
				let address = self.load(pc);
				format!("0x{:02x} ", address) +
				&format!("(0x{:02x}) ", self.load(address as u16) as i8) +
				&"zero_page".to_owned()
			},
			AddressingModes::IndexedZeroPageX => {
				let address = self.load(pc);
				format!("0x{:02x},X ", address) +
				&format!("(0x{:02x}) ", self.load(self.x.load().wrapping_add(address) as u16) as i8) +
				&"indexed_zero_page_x".to_owned()
			},
			AddressingModes::IndexedZeroPageY => {
				let address = self.load(pc);
				format!("0x{:02x},Y ", address) +
				&format!("(0x{:02x}) ", self.load(self.y.load().wrapping_add(address) as u16) as i8) +
				&"indexed_zero_page_y".to_owned()
			},
			AddressingModes::Indirect => {
				let address = self.load_2bytes(pc);
				let address2 = self.load_2bytes(address);
				format!("0x{:04x} ", address) +
				&format!("(0x{:04x}(0x{:02x})) ", address2, self.load(address2) as i8) +
				&"indirect".to_owned()
			},
			AddressingModes::IndexedIndirectX => {
				let address = self.load(pc) as u16;
				let address2 = (self.x.load() as u16).wrapping_add(address);
				format!("0x{:02x},X ", address) +
				&format!("(0x{:04x}(0x{:02x})) ", address2, self.load(address2) as i8) +
				&"indexed_indirect_x".to_owned()
			},
			AddressingModes::IndexedIndirectY => {
				let address = self.load(pc) as u16;
				let address2 = self.load_2bytes_from_zeropage(address).wrapping_add(self.x.load() as u16);
				format!("0x{:02x},Y ", address) +
				&format!("(0x{:04x}(0x{:02x})) ", address2, self.load(address2) as i8) +
				&"indexed_indirect_y".to_owned()
			},
			AddressingModes::Accumulator => {
				format!("A0x{:02x} ", self.a.load()) +
				&"accumulator".to_owned()
			},
			_ => { "".to_owned() }
		}
	}

	/// Save the complete emulator state
	pub fn save_state(&self) -> SaveState {
		let mut state = SaveState::new();

		// CPU state
		state.cpu = CpuState {
			pc: self.pc.get_data(),
			sp: self.sp.get_data(),
			a: self.a.get_data(),
			x: self.x.get_data(),
			y: self.y.get_data(),
			p: self.p.load(),
			ram: self.ram.get_data(),
			stall_cycles: self.stall_cycles,
		};

		// PPU state
		state.ppu = self.ppu.save_state();

		// APU state
		state.apu = self.apu.save_state();

		// Joypad state
		state.joypad1 = self.joypad1.save_state();
		state.joypad2 = self.joypad2.save_state();

		// Mapper state
		state.mapper = self.rom.save_mapper_state();

		state
	}

	/// Load a previously saved state
	pub fn load_state(&mut self, state: &SaveState) {
		// CPU state
		self.pc.set_data(state.cpu.pc);
		self.sp.set_data(state.cpu.sp);
		self.a.set_data(state.cpu.a);
		self.x.set_data(state.cpu.x);
		self.y.set_data(state.cpu.y);
		self.p.store(state.cpu.p);
		self.ram.set_data(&state.cpu.ram);
		self.stall_cycles = state.cpu.stall_cycles;

		// PPU state
		self.ppu.load_state(&state.ppu);

		// APU state
		self.apu.load_state(&state.apu);

		// Joypad state
		self.joypad1.load_state(&state.joypad1);
		self.joypad2.load_state(&state.joypad2);

		// Mapper state
		self.rom.load_mapper_state(&state.mapper);
	}
}

pub struct CpuStatusRegister {
	register: Register<u8>
}

impl CpuStatusRegister {
	pub fn new() -> Self {
		CpuStatusRegister {
			register: Register::<u8>::new()
		}
	}

	pub fn load(&self) -> u8 {
		self.register.load()
	}

	pub fn store(&mut self, value: u8) {
		self.register.store(value);
	}

	pub fn is_n(&self) -> bool {
		self.register.is_bit_set(7)
	}

	pub fn set_n(&mut self) {
		self.register.set_bit(7);
	}

	pub fn clear_n(&mut self) {
		self.register.clear_bit(7);
	}

	pub fn is_v(&self) -> bool {
		self.register.is_bit_set(6)
	}

	pub fn set_v(&mut self) {
		self.register.set_bit(6);
	}

	pub fn clear_v(&mut self) {
		self.register.clear_bit(6);
	}

	// 5-bit is unused bit (but somehow set from BRK) and no name.
	// I named random name "a".

	pub fn is_a(&self) -> bool {
		self.register.is_bit_set(5)
	}

	pub fn set_a(&mut self) {
		self.register.set_bit(5);
	}

	pub fn clear_a(&mut self) {
		self.register.clear_bit(5);
	}

	pub fn is_b(&self) -> bool {
		self.register.is_bit_set(4)
	}

	pub fn set_b(&mut self) {
		self.register.set_bit(4);
	}

	pub fn clear_b(&mut self) {
		self.register.clear_bit(4);
	}

	pub fn is_d(&self) -> bool {
		self.register.is_bit_set(3)
	}

	pub fn set_d(&mut self) {
		self.register.set_bit(3);
	}

	pub fn clear_d(&mut self) {
		self.register.clear_bit(3);
	}

	pub fn is_i(&self) -> bool {
		self.register.is_bit_set(2)
	}

	pub fn set_i(&mut self) {
		self.register.set_bit(2);
	}

	pub fn clear_i(&mut self) {
		self.register.clear_bit(2);
	}

	pub fn is_z(&self) -> bool {
		self.register.is_bit_set(1)
	}

	pub fn set_z(&mut self) {
		self.register.set_bit(1);
	}

	pub fn clear_z(&mut self) {
		self.register.clear_bit(1);
	}

	pub fn is_c(&self) -> bool {
		self.register.is_bit_set(0)
	}

	pub fn set_c(&mut self) {
		self.register.set_bit(0);
	}

	pub fn clear_c(&mut self) {
		self.register.clear_bit(0);
	}

	fn dump(&self) -> String {
		self.register.dump() +
		&"(".to_owned() +
		match self.is_n() { true => &"N", false => &"-" }.to_owned() +
		match self.is_v() { true => &"V", false => &"-" }.to_owned() +
		match self.is_a() { true => &"A", false => &"-" }.to_owned() +
		match self.is_b() { true => &"B", false => &"-" }.to_owned() +
		match self.is_d() { true => &"D", false => &"-" }.to_owned() +
		match self.is_i() { true => &"I", false => &"-" }.to_owned() +
		match self.is_z() { true => &"Z", false => &"-" }.to_owned() +
		match self.is_c() { true => &"C", false => &"-" }.to_owned() +
		&")".to_owned()
	}
}
