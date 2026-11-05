use molehill_sim::{grid::Grid, gen, stress::Stress, mat::Mat};
use std::time::Instant;
fn main() {
    let mut g = Grid::new(1024, 512);
    let t = Instant::now(); let (_, info) = gen::generate(&mut g, 42, -256); println!("gen {:?} chests {}", t.elapsed(), info.chests.len());
    let mut st = Stress::new(1024 * 512);
    let t = Instant::now(); let f = st.solve(&g); println!("solve {:?} fails {}", t.elapsed(), f.len());
    let mut by = std::collections::BTreeMap::new(); let mut disc = 0; let mut deep=0;
    for &i in &f { let i = i as usize; if st.util[i]==255 {disc+=1;} *by.entry(format!("{:?}", g.m(i))).or_insert(0)+=1; if i/1024 > 300 {deep+=1;} }
    for &i in f.iter().step_by(f.len()/8+1) { let i=i as usize; let (x,y)=(i%1024,i/1024); println!("x{} y{} {:?} reach{} below {:?} P{:.0}", x,y,g.m(i),st.reach[i], g.m(i+1024), st.overburden[i]); let mut s=String::new(); for yy in y-3..y+4 { for xx in x-12..x+12 { s.push(if xx==x&&yy==y {'@'} else if g.is_static(yy*1024+xx) {'#'} else if g.mat[yy*1024+xx]==0 {' '} else {'~'}); } s.push('\n'); } println!("{}", s); }
    println!("disc {} deep {} {:?}", disc, deep, by);
    let t = Instant::now(); for _ in 0..100 { g.step_ca(); } println!("ca x100 {:?} active {}", t.elapsed(), g.active_chunks());
    for m in [Mat::Water, Mat::Lava, Mat::Coal, Mat::CopperOre, Mat::TinOre, Mat::IronOre, Mat::Crystal, Mat::Quartz, Mat::Fungus, Mat::Roots, Mat::Empty] { print!("{:?}={} ", m, g.count(m)); }
    println!();
    // ascii preview downsampled
    for y in (0..512).step_by(16) { let mut s = String::new(); for x in (0..1024).step_by(8) { let m = g.get(x, y); s.push(match m { Mat::Empty => ' ', Mat::Water => '~', Mat::Lava => '^', Mat::Dirt | Mat::Grass => '.', Mat::Clay => ',', Mat::Limestone => '=', Mat::Granite => '#', Mat::Rock => '%', Mat::Basalt => 'B', Mat::Bedrock => 'X', Mat::Sand => ':', Mat::Gravel => ';', _ => '*' }); } println!("{}", s); }
}
#[allow(dead_code)]
fn _x() {}
