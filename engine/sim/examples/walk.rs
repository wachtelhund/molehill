use molehill_sim::world::World;
fn main() {
    let mut w = World::new(1024, 512, 3636); w.disasters.enabled = false;
    for _ in 0..60 { w.step(); }
    let (x, y) = (w.moles[0].x, w.moles[0].y);
    for d in 1..6 { let i = w.g.idx(x, y + d); println!("below+{d} {:?} mt {:?}", w.g.m(i), w.mine_time(i)); }
    for _ in 0..150 { let (x, y) = (w.moles[0].x, w.moles[0].y); w.input.mine = Some((x, y + 1)); w.step(); }
    println!("after S: {:?} -> {:?} pl {:?}", (x, y), (w.moles[0].x, w.moles[0].y), w.pl);
}
