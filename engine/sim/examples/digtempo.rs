//! Dig tempo, headless: per material and tool, the ticks one bite takes (`World::mine_time`) and the seconds
//! to clear a 4 × 3 m room of it with the tool's default round brush (bites of the disc's area, back to back;
//! walking and aiming not counted). Evidence for balancing `dig` in the material table.
//! The steam borer's column is the same room bored as 12 m of its 1 m tall tunnel (`borer.rs`: a column of the face
//! in `GRIND_S` × dig seconds, no faster than it drives), and how many metres a lump of coal's steam bores.
//! `cargo run --release -p molehill-sim --example digtempo`
use molehill_sim::{world::{World, BITES}, mat::{Mat, CPM, S, NMAT, Kind}, borer::{GRIND_S, GRIND_W, DRIVE_W, BORER_V}, steam::{burn_ticks, FIRE_W, BOILER_EFF}};
use molehill_sim::craft::Item;

fn main() {
    let mut w = World::new(64 * S, 64 * S, 1);
    let (x, y) = (w.g.w / 2, w.g.h / 2); let i = w.g.idx(x, y);
    let room = (4 * CPM * 3 * CPM) as f32;
    let tools = ["claws", "pickaxe", "drill", "bore"];
    println!("{:<16} {:>4}  {}{:>18}", "material", "dig", tools.iter().map(|t| format!("{t:>15}")).collect::<String>(), "steam borer");
    println!("{:<16} {:>4}  {}{:>18}", "", "", tools.iter().map(|_| format!("{:>15}", "ticks / room s")).collect::<String>(), "room s / m a coal");
    let coal_kj = FIRE_W * BOILER_EFF * burn_ticks(Item::Coal).unwrap() as f32 / 60. / 1000.;
    for k in 0..NMAT {
        let m = Mat::from_u8(k as u8);
        if !matches!(m.kind(), Kind::Solid | Kind::Powder) || m == Mat::Bedrock || m.p().dig == 0 { continue; }
        w.g.set(i, m);
        let mut row = format!("{:<16} {:>4}  ", m.p().name, m.p().dig);
        for t in 0..4u8 {
            w.input.tool = t;
            let area = std::f32::consts::PI * BITES[t as usize] * BITES[t as usize];
            row += &match w.mine_time(i) { Some(tk) => format!("{:>6} {:>7.1}s", tk, (room / area).ceil() * tk as f32 / 60.), None => format!("{:>15}", "-") };
        }
        let per_m = (CPM as f32 * GRIND_S * m.p().dig as f32).max(CPM as f32 / BORER_V);
        row += &format!("{:>9.0}s {:>7.1}", 12. * per_m, coal_kj / ((GRIND_W + DRIVE_W) * per_m / 1000.));
        println!("{row}");
    }
}
