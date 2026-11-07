//! The Lid myth as a thread through the Deepholt: clues left by older Delvers (carvings, notes),
//! drafts breathing down from cracks above, and odd things that don't belong down here. They grow
//! more common the higher you go. Each one found adds a fragment to the journal.
//!
//! The rule from LORE.md: the game never shows or confirms the Lid. Fragments hint, doubt and argue;
//! none of them proves anything. No magic, no modern words.
use crate::gen::Gen;
use crate::grid::hash3;
use crate::mat::{CPM, S};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ClueKind { Carving, Note, Draft, Relic }
impl ClueKind {
    pub fn name(self) -> &'static str { ["Carving", "Old note", "Draft", "Odd find"][self as usize] }
    pub fn from_u8(v: u8) -> ClueKind { [ClueKind::Carving, ClueKind::Note, ClueKind::Draft, ClueKind::Relic][(v as usize).min(3)] }
}
/// A clue in the window: window cell (x, y) and its absolute row `wy` (what `found` remembers).
/// `look` picks a variant of the object's art (which relic, which carving).
#[derive(Clone, Debug, PartialEq)]
pub struct Clue { pub kind: ClueKind, pub x: usize, pub y: usize, pub wy: i64, pub found: bool, pub look: u8 }

pub struct Fragment { pub kind: ClueKind, pub title: &'static str, pub text: &'static str }
/// The journal's fragments, read in this order per kind. None of them confirms the Lid.
pub const FRAGMENTS: &[Fragment] = &[
    Fragment { kind: ClueKind::Carving, title: "Moles climbing", text: "Three moles climb a long shaft, one above the other. The last one points up. The rock above them is left uncut." },
    Fragment { kind: ClueKind::Carving, title: "The round hole", text: "A circle cut deep, with lines falling from it like water. Under it, in old marks: the Lid." },
    Fragment { kind: ClueKind::Carving, title: "Counting marks", text: "Rows of tally marks climbing the wall, a gap, then more. Someone counted their way upward and stopped." },
    Fragment { kind: ClueKind::Carving, title: "Cold and wet", text: "Up is colder. Up is wetter. Up is where the water comes from. Cut three times, as if to be sure." },
    Fragment { kind: ClueKind::Carving, title: "A closed eye", text: "A mole with its eyes shut, face turned up, whiskers spread. Feeling for something no one could see." },
    Fragment { kind: ClueKind::Carving, title: "Worn pattern", text: "A knocked pattern carved as dots: long, short, short, long. A listener's tune with no thrummer to play it." },
    Fragment { kind: ClueKind::Note, title: "A grandmother's claim", text: "My grandmother swore the drafts smell different up high. Wetter. Greener. She never said of what." },
    Fragment { kind: ClueKind::Note, title: "Forty days", text: "We dug forty days upward. The rock only got wetter and the roots thicker. We turned back. I still wonder." },
    Fragment { kind: ClueKind::Note, title: "Pip's doubt", text: "Pip says the Lid is a story for pups, to keep them digging. Pip has never stood in a draft." },
    Fragment { kind: ClueKind::Note, title: "Water from nowhere", text: "If there is nothing above, where does all the water come from? It never stops seeping down." },
    Fragment { kind: ClueKind::Note, title: "A rule", text: "Holt rule: no one digs straight up alone. Nobody remembers why." },
    Fragment { kind: ClueKind::Note, title: "Unfinished", text: "Found the old shaft my father spoke of. Going up tomorrow. If this note is still here," },
    Fragment { kind: ClueKind::Draft, title: "A draft from above", text: "A thin, cool draft pours down from a crack overhead. Every other breath in the Deepholt rises." },
    Fragment { kind: ClueKind::Draft, title: "A far sound", text: "Standing here you hear something faint and long, like water over stone, but from above, and never stopping." },
    Fragment { kind: ClueKind::Draft, title: "Fresh air", text: "The air here is fresh, fresher than any vent makes it. It comes from somewhere." },
    Fragment { kind: ClueKind::Draft, title: "Pressure", text: "Your ears pop. The draft pulses slowly, in and out, like something very large breathing." },
    Fragment { kind: ClueKind::Draft, title: "A smell", text: "The draft smells of nothing you know. Not rock, not root, not fungus. Damp, and sharp, and green." },
    Fragment { kind: ClueKind::Draft, title: "Silence", text: "The draft stops, all at once. Then it comes back, colder. You wait a long time for it to stop again." },
    Fragment { kind: ClueKind::Relic, title: "A flat green thing", text: "Thin as a beetle's wing, veined like a paw, dry as dust. No root down here grows anything like it." },
    Fragment { kind: ClueKind::Relic, title: "A feather", text: "Light as nothing, made for moving through open air. No Delver has seen the creature that dropped it." },
    Fragment { kind: ClueKind::Relic, title: "Blue glass", text: "A sliver of glass the blue of nothing in the Deepholt. Not quartz, not crystal. It doesn't glow." },
    Fragment { kind: ClueKind::Relic, title: "A hard seed", text: "A seed with a shell like stone. Planted in the damp, it does nothing at all. It is waiting for something." },
    Fragment { kind: ClueKind::Relic, title: "A round pebble", text: "Worn perfectly round and smooth, as if it had rolled for years in fast water. There is no water that fast down here." },
    Fragment { kind: ClueKind::Relic, title: "Bleached bone", text: "A small bone, pale as chalk, bleached by something. Nothing in the dark bleaches." },
];

/// Clue bands are this many rows tall (32 m, like chest bands).
pub const LORE_BAND: usize = 32 * CPM;
/// Clues expected in a 32 m band at depth `d` metres (+ deeper): about one at the start layer, doubling
/// every 30 m upward, fading out below.
pub fn clue_rate(depth_m: f32) -> f32 { (1.2 * 2f32.powf(-depth_m / 30.)).clamp(0.1, 8.) }

/// The clues a band holds: a pure function of the seed and the band, like chests. Each kind wants its
/// own spot: a carving on a wall, a note or an odd find on a floor, a draft where an open shaft rises.
/// Drafts and odd finds only show up above the start layer; deep down a clue is a carving or a note.
/// A band is `gen.w0` columns wide: span `bx` 0 is the home columns, the others run on either side.
/// Returns (kind, absolute column, absolute row, look).
pub fn clues_in_band(gen: &Gen, seed: u32, bx: i64, band: i64) -> Vec<(ClueKind, i64, i64, u8)> {
    let w = gen.w0; if w <= 16 * S { return vec![]; }
    let mix = |b: i64| if bx == 0 { b as u32 } else { (b as u32) ^ (bx as u32).wrapping_mul(0x85EB_CA6B) };
    let bh = LORE_BAND as i64; let mid = (band * bh + bh / 2) as f32 / CPM as f32; // band centre depth, metres
    let rate = clue_rate(mid); let h0 = hash3(mix(band), 17, seed ^ 0x11D);
    let n = rate.floor() as u32 + ((h0 & 1023) as f32 / 1024. < rate.fract()) as u32;
    let empty = |x: i64, wy: i64| gen.open(x, wy) && gen.cell(x, wy).0 == 0;
    let mut v = vec![];
    for k in 0..n * 6 { if v.len() as u32 >= n { break; }
        let h = hash3(k, mix(band), seed ^ 0x1D7); let x = bx * w as i64 + (8 * S + h as usize % (w - 16 * S)) as i64;
        let r = (h >> 20) % 100; let shallow = mid < 0.;
        let kind = if shallow && r < 25 { ClueKind::Draft } else if shallow && r < 50 { ClueKind::Relic } else if r < 75 { ClueKind::Carving } else { ClueKind::Note };
        for dy in 0..bh { let wy = band * bh + dy; if !empty(x, wy) { continue; }
            let ok = match kind {
                ClueKind::Carving => !gen.open(x + 1, wy) || !gen.open(x - 1, wy),     // beside a wall
                ClueKind::Note | ClueKind::Relic => !gen.open(x, wy + 1),             // on a floor
                ClueKind::Draft => (1..6 * S as i64).all(|u| gen.open(x, wy - u)),    // under a rising shaft
            };
            if ok { v.push((kind, x, wy, (h >> 8) as u8)); break; }
        }
    }
    v
}

/// Journal ids from here up are the Ancients' carvings found in ruins (`ruins::FRAGMENTS`), offset so
/// they never collide with the clue fragments above. The journal and saves hold both kinds of id.
pub const RUIN_BASE: u16 = 1000;
/// A title per ruin carving (same order as `ruins::FRAGMENTS`), for the journal's title line.
pub const RUIN_TITLES: [&str; 8] = ["Delvers in a line", "Knocked marks", "A scored plate", "A shaft with no top",
    "Signs for water and cool", "Twelve stations", "A newer scratch", "The holt layers"];
/// The journal id of a ruin's carving text, if it is one of `ruins::FRAGMENTS`.
pub fn ruin_entry(text: &str) -> Option<u16> { crate::ruins::FRAGMENTS.iter().position(|f| *f == text).map(|k| RUIN_BASE + k as u16) }
/// What the journal shows for an id: (label, title, text). Clue fragments and ruin carvings alike.
pub fn entry(id: u16) -> (&'static str, &'static str, &'static str) {
    if id >= RUIN_BASE { let k = (id - RUIN_BASE) as usize % crate::ruins::FRAGMENTS.len(); return ("Ancient carving", RUIN_TITLES[k % RUIN_TITLES.len()], crate::ruins::FRAGMENTS[k]); }
    let f = &FRAGMENTS[id as usize % FRAGMENTS.len()]; (f.kind.name(), f.title, f.text)
}

/// The next fragment of this kind not yet in the journal, if any are left.
pub fn next_fragment(kind: ClueKind, journal: &[u16]) -> Option<u16> {
    FRAGMENTS.iter().enumerate().find(|(k, f)| f.kind == kind && !journal.contains(&(*k as u16))).map(|(k, _)| k as u16)
}
