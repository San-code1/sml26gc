//! SM83 (Game Boy CPU) disassembler and raw ROM tables.
use crate::console::load;
use std::path::Path;

const R8: [&str; 8] = ["b", "c", "d", "e", "h", "l", "(hl)", "a"];
const RP: [&str; 4] = ["bc", "de", "hl", "sp"];
const RP2: [&str; 4] = ["bc", "de", "hl", "af"];
const CC: [&str; 4] = ["nz", "z", "nc", "c"];
const ALU: [&str; 8] = ["add a,", "adc a,", "sub ", "sbc a,", "and ", "xor ", "or ", "cp "];
const ROT: [&str; 8] = ["rlc", "rrc", "rl", "rr", "sla", "sra", "swap", "srl"];

/// Disassemble one instruction; returns (text, length).
pub fn dis1(b: &[u8; 3], pc: u16) -> (String, u16) {
    let op = b[0];
    let n = b[1];
    let nn = (b[2] as u16) << 8 | b[1] as u16;
    let rel = pc.wrapping_add(2).wrapping_add(n as i8 as i16 as u16);
    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;
    let s = |t: String, l| (t, l);
    match op {
        0x00 => s("nop".into(), 1),
        0x08 => s(format!("ld (${nn:04x}),sp"), 3),
        0x10 => s("stop".into(), 2),
        0x18 => s(format!("jr ${rel:04x}"), 2),
        0x20 | 0x28 | 0x30 | 0x38 => s(format!("jr {},${rel:04x}", CC[(y - 4) as usize]), 2),
        0x22 => s("ld (hl+),a".into(), 1),
        0x2A => s("ld a,(hl+)".into(), 1),
        0x32 => s("ld (hl-),a".into(), 1),
        0x3A => s("ld a,(hl-)".into(), 1),
        0x02 => s("ld (bc),a".into(), 1),
        0x12 => s("ld (de),a".into(), 1),
        0x0A => s("ld a,(bc)".into(), 1),
        0x1A => s("ld a,(de)".into(), 1),
        0x07 => s("rlca".into(), 1),
        0x0F => s("rrca".into(), 1),
        0x17 => s("rla".into(), 1),
        0x1F => s("rra".into(), 1),
        0x27 => s("daa".into(), 1),
        0x2F => s("cpl".into(), 1),
        0x37 => s("scf".into(), 1),
        0x3F => s("ccf".into(), 1),
        0x76 => s("halt".into(), 1),
        0xC3 => s(format!("jp ${nn:04x}"), 3),
        0xC9 => s("ret".into(), 1),
        0xD9 => s("reti".into(), 1),
        0xCD => s(format!("call ${nn:04x}"), 3),
        0xE0 => s(format!("ldh ($ff{n:02x}),a"), 2),
        0xF0 => s(format!("ldh a,($ff{n:02x})"), 2),
        0xE2 => s("ld ($ff00+c),a".into(), 1),
        0xF2 => s("ld a,($ff00+c)".into(), 1),
        0xE8 => s(format!("add sp,{}", n as i8), 2),
        0xF8 => s(format!("ld hl,sp{:+}", n as i8), 2),
        0xE9 => s("jp hl".into(), 1),
        0xF9 => s("ld sp,hl".into(), 1),
        0xEA => s(format!("ld (${nn:04x}),a"), 3),
        0xFA => s(format!("ld a,(${nn:04x})"), 3),
        0xF3 => s("di".into(), 1),
        0xFB => s("ei".into(), 1),
        0xCB => {
            let r = R8[(n & 7) as usize];
            let k = (n >> 3) & 7;
            match n >> 6 {
                0 => s(format!("{} {r}", ROT[k as usize]), 2),
                1 => s(format!("bit {k},{r}"), 2),
                2 => s(format!("res {k},{r}"), 2),
                _ => s(format!("set {k},{r}"), 2),
            }
        }
        _ => match x {
            0 => match z {
                1 => {
                    if y & 1 == 0 {
                        s(format!("ld {},${nn:04x}", RP[(y >> 1) as usize]), 3)
                    } else {
                        s(format!("add hl,{}", RP[(y >> 1) as usize]), 1)
                    }
                }
                3 => s(format!("{} {}", if y & 1 == 0 { "inc" } else { "dec" }, RP[(y >> 1) as usize]), 1),
                4 => s(format!("inc {}", R8[y as usize]), 1),
                5 => s(format!("dec {}", R8[y as usize]), 1),
                6 => s(format!("ld {},${n:02x}", R8[y as usize]), 2),
                _ => s(format!("db ${op:02x}"), 1),
            },
            1 => s(format!("ld {},{}", R8[y as usize], R8[z as usize]), 1),
            2 => s(format!("{}{}", ALU[y as usize], R8[z as usize]), 1),
            _ => match z {
                0 if y < 4 => s(format!("ret {}", CC[y as usize]), 1),
                1 if y & 1 == 0 => s(format!("pop {}", RP2[(y >> 1) as usize]), 1),
                2 if y < 4 => s(format!("jp {},${nn:04x}", CC[y as usize]), 3),
                4 if y < 4 => s(format!("call {},${nn:04x}", CC[y as usize]), 3),
                5 if y & 1 == 0 => s(format!("push {}", RP2[(y >> 1) as usize]), 1),
                6 => s(format!("{}${n:02x}", ALU[y as usize]), 2),
                7 => s(format!("rst ${:02x}", y * 8), 1),
                _ => s(format!("db ${op:02x}"), 1),
            },
        },
    }
}

/// Print `count` instructions of ROM bank `bank` starting at CPU address `addr`.
pub fn disasm(rom: &Path, bank: usize, addr: u16, count: u32) {
    let Some(gb) = load(rom) else { return };
    let off = |a: u16| -> usize {
        if a < 0x4000 {
            a as usize
        } else {
            bank * 0x4000 + (a as usize - 0x4000)
        }
    };
    let mut pc = addr;
    for _ in 0..count {
        let b = [gb.cart().rom_byte(off(pc)), gb.cart().rom_byte(off(pc.wrapping_add(1))), gb.cart().rom_byte(off(pc.wrapping_add(2)))];
        let (t, l) = dis1(&b, pc);
        let bytes: Vec<String> = b[..l as usize].iter().map(|x| format!("{x:02x}")).collect();
        println!("{:02x}:{pc:04x}  {:<9} {t}", if pc < 0x4000 { 0 } else { bank }, bytes.join(" "));
        pc = pc.wrapping_add(l);
    }
}

/// Print ROM bytes as signed rows of `row` values with each row's sum.
pub fn table(rom: &Path, off: usize, len: usize, row: usize) {
    let Some(gb) = load(rom) else { return };
    let mut i = 0;
    while i < len {
        let vals: Vec<i32> = (0..row.min(len - i)).map(|k| gb.cart().rom_byte(off + i + k) as i8 as i32).collect();
        let s: i32 = vals.iter().sum();
        let t: Vec<String> = vals.iter().map(|v| format!("{v:3}")).collect();
        println!("{:05x} idx {:02x}: {}  sum {s}", off + i, i, t.join(""));
        i += row;
    }
}

#[cfg(test)]
mod tests {
    use super::dis1;

    #[test]
    fn known_instructions() {
        let d = |b: [u8; 3], pc| dis1(&b, pc);
        assert_eq!(d([0x00, 0, 0], 0), ("nop".into(), 1));
        assert_eq!(d([0xC3, 0x50, 0x01], 0), ("jp $0150".into(), 3));
        assert_eq!(d([0x18, 0xFE, 0], 0x200), ("jr $0200".into(), 2)); // jumps to itself
        assert_eq!(d([0x20, 0x05, 0], 0x100), ("jr nz,$0107".into(), 2));
        assert_eq!(d([0xE0, 0x9B, 0], 0), ("ldh ($ff9b),a".into(), 2));
        assert_eq!(d([0x3E, 0x08, 0], 0), ("ld a,$08".into(), 2));
        assert_eq!(d([0x78, 0, 0], 0), ("ld a,b".into(), 1));
        assert_eq!(d([0xD6, 0x04, 0], 0), ("sub $04".into(), 2));
        assert_eq!(d([0xCB, 0x7F, 0], 0), ("bit 7,a".into(), 2));
        assert_eq!(d([0xCB, 0x37, 0], 0), ("swap a".into(), 2));
        assert_eq!(d([0xD3, 0, 0], 0), ("db $d3".into(), 1));
    }
}
