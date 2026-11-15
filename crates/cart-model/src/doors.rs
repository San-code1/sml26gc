//! Doors between the world map and the zones, as the map code (bank $0F) has them.
//!
//! * Into a zone: on the world map, A or START on a point runs the chain at 0F:460B,
//!   `cp point / jp z,zone_code`; the zone's code (e.g. 0F:50D7 for the Tree Zone) loads the
//!   zone and puts Mario on its first point. Each zone has one or two `cp` there (its
//!   entrance and the point Mario stands on after walking out, e.g. $02 and $19).
//! * Out of a zone: on a zone map, the chain at 0F:4673 (`cp point / jp z,exit`) picks the
//!   exit code of a point; most check a pad direction, fade out and run
//!   `ld a,point / call $47CA`, which puts Mario on that world map point.
//!
//! Changing a door changes those bytes: the `cp` operands say which point is a door, the
//! `ld a,point` which world point an exit leads to. An exit can also lead straight into
//! another zone: its `ld a,point / call $47CA` (5 bytes) becomes `jp zone_code / nop / nop`,
//! the same code the world map's door runs (checked in the emulator: Tree Zone out to Macro
//! Zone, and Macro Zone is then walked and left as usual). A `cp` operand of $FF matches no
//! point (a door taken out).
//!
//! Turtle Zone is entered by walking a script from $0C to a hidden point ($0D), so it is not
//! one of the zones a door can lead into; its exit can be changed like the others.
use crate::patch::TWIN;
use crate::rom::{offset, set_word, word};

/// A zone a door can lead into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZoneDoor {
    /// Index into `zone_maps::ZONES`.
    pub zone: usize,
    /// Its code in bank $0F.
    pub code: u16,
    /// Where the chain keeps the points that open it (`cp` operands).
    pub slots: &'static [u16],
}

pub const INTO: [ZoneDoor; 5] = [
    ZoneDoor { zone: 0, code: 0x50D7, slots: &[0x460F, 0x4614] }, // Tree
    ZoneDoor { zone: 1, code: 0x51E0, slots: &[0x4619, 0x461E] }, // Pumpkin
    ZoneDoor { zone: 2, code: 0x533A, slots: &[0x4623, 0x4628] }, // Macro
    ZoneDoor { zone: 3, code: 0x5477, slots: &[0x462D] },         // Space
    ZoneDoor { zone: 5, code: 0x5760, slots: &[0x463C, 0x4641] }, // Mario
];

/// An exit out of a zone: which point it is on (`cp` operand in the chain) and where its
/// `ld a,point / call $47CA` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitSite {
    pub at: u16,
    pub to: u16,
    /// The exit point in the game.
    pub point: u8,
}

const fn x(at: u16, to: u16, point: u8) -> ExitSite {
    ExitSite { at, to, point }
}

pub const EXITS: [ExitSite; 11] = [
    x(0x4677, 0x46BD, 0x23),
    x(0x467C, 0x46D7, 0x28),
    x(0x4681, 0x46E9, 0x2D),
    x(0x4686, 0x46FB, 0x30),
    x(0x468B, 0x4724, 0x32),
    x(0x4690, 0x4736, 0x40),
    x(0x4695, 0x4750, 0x46),
    x(0x469A, 0x4762, 0x4A),
    x(0x469F, 0x477C, 0x50),
    x(0x46A4, 0x4796, 0x5C),
    x(0x46A9, 0x47B0, 0x74),
];

/// No point: a door taken out.
pub const NONE: u8 = 0xFF;

/// Where an exit leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leads {
    /// To this point of the world map.
    World(u8),
    /// Into the zone `INTO[i]`.
    Zone(usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Doors {
    /// Per `INTO` zone, the points that open it (`NONE` = slot not used).
    pub into: Vec<Vec<u8>>,
    /// Per `EXITS` site, the zone point it is on and where it leads.
    pub exits: Vec<(u8, Leads)>,
}

fn b(rom: &[u8], a: u16) -> u8 {
    rom[offset(0x0F, a)]
}

/// Are the bytes where the doors are what this code expects?
pub fn known(rom: &[u8]) -> bool {
    INTO.iter().all(|z| z.slots.iter().all(|&s| b(rom, s - 1) == 0xFE && b(rom, s + 1) == 0xCA && word(rom, offset(0x0F, s + 2)) == z.code))
        && EXITS.iter().all(|e| {
            let t = offset(0x0F, e.to);
            b(rom, e.at - 1) == 0xFE
                && b(rom, e.at + 1) == 0xCA
                && ((rom[t] == 0x3E && rom[t + 2..t + 5] == [0xCD, 0xCA, 0x47]) || (rom[t] == 0xC3 && rom[t + 3..t + 5] == [0, 0]))
        })
}

pub fn read(rom: &[u8]) -> Doors {
    if !known(rom) {
        return Doors::default();
    }
    let into = INTO.iter().map(|z| z.slots.iter().map(|&s| b(rom, s)).collect()).collect();
    let exits = EXITS
        .iter()
        .map(|e| {
            let t = offset(0x0F, e.to);
            let leads = if rom[t] == 0xC3 {
                let code = word(rom, t + 1);
                INTO.iter().position(|z| z.code == code).map(Leads::Zone).unwrap_or(Leads::World(NONE))
            } else {
                Leads::World(rom[t + 1])
            };
            (b(rom, e.at), leads)
        })
        .collect();
    Doors { into, exits }
}

/// Write the doors (both halves of an expanded copy). False if this cartridge's map code
/// is not the one the doors were found in.
pub fn write(rom: &mut [u8], d: &Doors) -> bool {
    if !known(rom) || d.into.len() != INTO.len() || d.exits.len() != EXITS.len() {
        return false;
    }
    let halves: &[usize] = if rom.len() >= 2 * TWIN { &[0, TWIN] } else { &[0] };
    for &base in halves {
        for (z, pts) in INTO.iter().zip(&d.into) {
            for (&s, &p) in z.slots.iter().zip(pts) {
                rom[base + offset(0x0F, s)] = p;
            }
        }
        for (e, &(at, leads)) in EXITS.iter().zip(&d.exits) {
            rom[base + offset(0x0F, e.at)] = at;
            let t = base + offset(0x0F, e.to);
            match leads {
                Leads::World(p) => {
                    rom[t..t + 5].copy_from_slice(&[0x3E, p, 0xCD, 0xCA, 0x47]);
                }
                Leads::Zone(i) => {
                    rom[t] = 0xC3;
                    set_word(rom, t + 1, INTO[i].code);
                    rom[t + 3] = 0;
                    rom[t + 4] = 0;
                }
            }
        }
    }
    true
}

impl Doors {
    /// The zone (`INTO` index) world point `p` opens, if any.
    pub fn door_of(&self, p: u8) -> Option<usize> {
        self.into.iter().position(|s| s.contains(&p))
    }

    /// Make world point `p` open zone `zone` (`INTO` index), or nothing. It takes the zone's
    /// first slot (the point there stops being a door); its other slots stay.
    pub fn set_door(&mut self, p: u8, zone: Option<usize>) {
        for s in self.into.iter_mut().flatten() {
            if *s == p {
                *s = NONE;
            }
        }
        if let Some(z) = zone {
            self.into[z][0] = p;
        }
    }

    /// The exit (`EXITS` index) on zone point `p`, if any.
    pub fn exit_on(&self, p: u8) -> Option<usize> {
        self.exits.iter().position(|e| e.0 == p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake bank $0F with the chains as the game has them.
    fn fake() -> Vec<u8> {
        let mut rom = vec![0u8; 0x10 * 0x4000];
        let o = |a: u16| offset(0x0F, a);
        for z in INTO {
            for &s in z.slots {
                rom[o(s - 1)] = 0xFE;
                rom[o(s)] = 0x02;
                rom[o(s + 1)] = 0xCA;
                set_word(&mut rom, o(s + 2), z.code);
            }
        }
        for e in EXITS {
            rom[o(e.at - 1)] = 0xFE;
            rom[o(e.at)] = e.point;
            rom[o(e.at + 1)] = 0xCA;
            rom[o(e.to)..o(e.to) + 5].copy_from_slice(&[0x3E, 0x19, 0xCD, 0xCA, 0x47]);
        }
        rom
    }

    #[test]
    fn doors_round_trip_and_an_exit_into_a_zone() {
        let mut rom = fake();
        assert!(known(&rom));
        let mut d = read(&rom);
        assert_eq!(d.exits[0], (0x23, Leads::World(0x19)));
        d.exits[0].1 = Leads::Zone(2);
        d.exits[1] = (0x29, Leads::World(0x07));
        d.set_door(0x7B, Some(1));
        assert!(write(&mut rom, &d));
        assert_eq!(rom[offset(0x0F, 0x46BD)..offset(0x0F, 0x46BD) + 5], [0xC3, 0x3A, 0x53, 0, 0]);
        assert_eq!(read(&rom), d);
        assert_eq!(d.door_of(0x7B), Some(1));
        assert_eq!(d.exit_on(0x29), Some(1));
        d.set_door(0x7B, None);
        assert_eq!(d.door_of(0x7B), None);
        assert_eq!(d.into[1][0], NONE);
    }
}
