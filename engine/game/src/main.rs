//! Molehill - Bevy front-end. The simulation lives in `molehill-sim`; this crate composites the
//! world into a GPU texture (albedo x per-cell light, fog of war, designations, cracks), draws
//! parallax background layers behind open cells, and syncs sprites for moles, items, chests,
//! stations and torches.
mod art;
mod audio;
use molehill_sim::logic::{Dev, DEVS, B as LB};
use molehill_sim::disasters::{Phase, Kind2};
use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::window::PrimaryWindow;
use molehill_sim::craft::{self, Item, Station, ITEMS, NITEM, QUESTS, RECIPES, STATIONS};
use molehill_sim::grid::{hash3, F_SEEN};
use molehill_sim::mat::{Kind, Mat, PROPS};
use molehill_sim::world::{self as sw, World};

const W: usize = 1024; const H: usize = 512; const CPX: f32 = 4.0; // world units per cell
const PAD: usize = 512;
const CH: usize = 64; // texture chunk size in cells
const FAR: usize = 4; // chunk rows drawn beyond each end of the simulated window (the camera roams there; nothing simulates)

#[derive(Resource)] struct Game { w: World, chunks: Vec<Handle<Image>>, buf: Vec<u8>, uploads: usize, acc: f32, paused: bool, speed: u32, tool: Tool, brush: usize, recipe: usize, station: usize, frame: u64, last_light: u64, msg: String, help: bool, dev: usize, ev: Vec<sw::Event>, shake_off: Vec2, autosave: f32, last_oy: i64, far: Vec<Handle<Image>>, far_done: Vec<bool>, inv_open: bool, aim: Option<(usize, usize, bool)> }
#[derive(Clone, Copy, PartialEq, Debug)] enum Tool { Wire, Device, Dig, Farm, Wood, Brick, Iron, Torch, Tnt, Station, Cancel, Stockpile }
#[derive(Resource)] struct Bg { layers: Vec<(Vec<u8>, f32)> }
#[derive(Component)] struct WorldSprite;
#[derive(Component)] struct Hud; #[derive(Component)] struct HudRight; #[derive(Component)] struct HudBottom;
#[derive(Component)] struct Pooled(u8, usize);
#[derive(Resource)] struct Sprites { mole: [Handle<Image>; 2], icons: Vec<Handle<Image>>, chest: [Handle<Image>; 2], station: Vec<[Handle<Image>; 2]>, torch: Handle<Image>, shroom: [Handle<Image>; 3], px: Handle<Image> }

fn seed_now() -> u32 {
    // ?seed=N (web) / --seed=N (native) pins the first world
    #[cfg(target_arch = "wasm32")] { if let Some(v) = web_sys::window().and_then(|w| w.location().search().ok()).and_then(|q| q.split(['?', '&']).find_map(|kv| kv.strip_prefix("seed=").and_then(|v| v.parse::<u32>().ok()))) { return v; } }
    #[cfg(not(target_arch = "wasm32"))] { if let Some(v) = std::env::args().find_map(|a| a.strip_prefix("--seed=").and_then(|v| v.parse::<u32>().ok())) { return v; } }
    #[cfg(target_arch = "wasm32")] { (js_sys::Date::now() as u64 % 1_000_000_007) as u32 }
    #[cfg(not(target_arch = "wasm32"))] { (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() % 1_000_000_007) as u32 }
}
fn fresh_requested() -> bool {
    #[cfg(target_arch = "wasm32")] { web_sys::window().and_then(|w| w.location().search().ok()).map(|q| q.contains("fresh")).unwrap_or(false) }
    #[cfg(not(target_arch = "wasm32"))] { std::env::args().any(|a| a == "--fresh") }
}
/// Save slots: localStorage (base64) on the web, files next to the binary natively.
fn storage_get(slot: &str) -> Option<Vec<u8>> {
    #[cfg(target_arch = "wasm32")] { let st = web_sys::window()?.local_storage().ok()??; let s = st.get_item(slot).ok()??; molehill_sim::save::from_b64(&s).ok() }
    #[cfg(not(target_arch = "wasm32"))] { std::fs::read(format!("{slot}.save")).ok() }
}
fn storage_put(slot: &str, b: &[u8]) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")] { let st = web_sys::window().and_then(|w| w.local_storage().ok().flatten()).ok_or("no localStorage")?; st.set_item(slot, &molehill_sim::save::to_b64(b)).map_err(|_| "storage full".to_string()) }
    #[cfg(not(target_arch = "wasm32"))] { std::fs::write(format!("{slot}.save"), b).map_err(|e| e.to_string()) }
}
fn cell_to_world(x: f32, y: f32) -> Vec2 { Vec2::new(x * CPX - W as f32 * CPX / 2., H as f32 * CPX / 2. - y * CPX) }
fn world_to_cell(p: Vec2) -> (i32, i32) { (((p.x + W as f32 * CPX / 2.) / CPX).floor() as i32, ((H as f32 * CPX / 2. - p.y) / CPX).floor() as i32) }

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { title: "Molehill".into(), canvas: Some("#bevy".into()), fit_canvas_to_parent: true, prevent_default_event_handling: true, ..default() }), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.04, 0.035, 0.05)))
        .add_systems(Startup, setup)
        .insert_non_send(audio::Audio::new())
        .add_systems(Update, (input, simulate, render_world, sync_sprites, hud, sound).chain())
        .run();
}

fn make_world(seed: u32) -> World { let mut w = World::new(W, H, seed); stage_scene(&mut w); w }
/// --scene=water|tnt (native, for recording): stage a set piece next to the burrow. Play is unchanged.
fn stage_scene(w: &mut World) {
    #[cfg(not(target_arch = "wasm32"))] {
        let Some(sc) = std::env::args().find_map(|a| a.strip_prefix("--scene=").map(|s| s.to_string())) else { return };
        let (mx, my) = (w.moles[0].x, w.moles[0].y); let set = |w: &mut World, x: usize, y: usize, m: Mat| { let i = w.g.idx(x, y); w.g.set(i, m); };
        w.disasters.enabled = false;
        match sc.as_str() {
            "water" => { // an open tunnel east out of the burrow ending in clay, with a water pocket behind it
                for x in mx + 4..mx + 30 { for y in my - 2..=my { set(w, x, y, Mat::Empty); } set(w, x, my + 1, Mat::Clay); set(w, x, my - 3, Mat::Limestone); }
                for x in mx + 30..mx + 34 { for y in my - 5..=my + 1 { set(w, x, y, Mat::Clay); } }
                for x in mx + 34..mx + 50 { for y in my - 12..=my + 1 { set(w, x, y, Mat::Water); } set(w, x, my + 2, Mat::Clay); }
                molehill_sim::light::reveal_box(&mut w.g, mx, my - 20, mx + 60, my + 10);
            }
            "tnt" => { // a long tunnel under a sand bed
                for x in mx + 4..mx + 60 { for y in my - 2..=my { set(w, x, y, Mat::Empty); } set(w, x, my + 1, Mat::Limestone); for y in my - 6..my - 2 { set(w, x, y, Mat::Dirt); } for y in my - 16..my - 6 { set(w, x, y, Mat::Sand); } }
                w.inv[Item::Tnt as usize] = 3; w.inv[Item::Torch as usize] = 12;
                molehill_sim::light::reveal_box(&mut w.g, mx, my - 24, mx + 70, my + 10);
                for k in 0..4 { w.torches.push((mx + 22 + k * 10, my - 2)); }
            }
            _ => {}
        }
        let f = w.st.solve(&w.g); w.st.crack(&f);
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let seed = seed_now() % 100000;
    let w = make_world(seed);
    // The world is split into CH×CH-cell chunk textures; only chunks whose composited pixels changed
    // are touched (Assets::get_mut marks an image for re-upload), so a quiet frame uploads nothing.
    let oy0 = w.oy;
    let mut chunks = vec![];
    for cy in 0..H / CH { for cx in 0..W / CH {
        let img = images.add(art::image(CH as u32, CH as u32, vec![0; CH * CH * 4], true));
        let p = cell_to_world((cx * CH) as f32 + CH as f32 / 2., (cy * CH) as f32 + CH as f32 / 2.);
        commands.spawn((Sprite { image: img.clone(), custom_size: Some(Vec2::splat(CH as f32 * CPX)), ..default() }, Transform::from_xyz(p.x, p.y, 0.), WorldSprite));
        chunks.push(img);
    } }
    // far bands: FAR chunk rows above (band rows 0..FAR) and below (FAR..2·FAR) the window
    let mut far = vec![];
    for b in 0..2 * FAR { for cx in 0..W / CH {
        let cy = if b < FAR { b as f32 - FAR as f32 } else { (H / CH + b - FAR) as f32 };
        let img = images.add(art::image(CH as u32, CH as u32, vec![0; CH * CH * 4], true));
        let p = cell_to_world((cx * CH) as f32 + CH as f32 / 2., cy * CH as f32 + CH as f32 / 2.);
        commands.spawn((Sprite { image: img.clone(), custom_size: Some(Vec2::splat(CH as f32 * CPX)), ..default() }, Transform::from_xyz(p.x, p.y, 0.), WorldSprite));
        far.push(img);
    } }
    let far_done = vec![false; far.len()];
    commands.insert_resource(make_bg(&w));
    let (sx, sy) = (w.moles[0].x as f32, w.moles[0].y as f32 - 1.5);
    let p = cell_to_world(sx, sy);
    commands.spawn((Camera2d, Transform::from_xyz(p.x, p.y, 100.).with_scale(Vec3::splat(0.32))));
    let font = |s: f32| TextFont { font_size: s.into(), ..default() };
    commands.spawn((Text::new(""), font(14.), TextColor(Color::srgb(0.95, 0.93, 0.88)), Node { position_type: PositionType::Absolute, left: Val::Px(10.), top: Val::Px(8.), ..default() }, Hud));
    commands.spawn((Text::new(""), font(13.), TextColor(Color::srgb(0.9, 0.9, 0.85)), Node { position_type: PositionType::Absolute, right: Val::Px(10.), top: Val::Px(8.), max_width: Val::Px(420.), ..default() }, HudRight));
    commands.spawn((Text::new(""), font(13.), TextColor(Color::srgb(0.85, 0.85, 0.8)), Node { position_type: PositionType::Absolute, left: Val::Px(10.), bottom: Val::Px(8.), ..default() }, HudBottom));
    let sp = Sprites {
        mole: [images.add(art::mole(0)), images.add(art::mole(1))],
        icons: ITEMS.iter().map(|it| { let k = match it { Item::Crystal | Item::Quartz => 2, Item::Brick | Item::Glass | Item::CopperIngot | Item::TinIngot | Item::BronzeIngot | Item::IronIngot | Item::WoodPillar | Item::IronBeam => 1, Item::Torch | Item::Lamp => 3, _ => 0 }; images.add(art::icon(it.color(), k)) }).collect(),
        chest: [images.add(art::chest(false)), images.add(art::chest(true))],
        station: (0..4u8).map(|k| [images.add(art::station(k, false)), images.add(art::station(k, true))]).collect(),
        torch: images.add(art::icon([255, 190, 90], 3)),
        shroom: [images.add(art::mushroom(0)), images.add(art::mushroom(1)), images.add(art::mushroom(2))],
        px: images.add(art::image(1, 1, vec![255; 4], true)),
    };
    commands.insert_resource(sp);
    commands.insert_resource(Game { w, chunks, buf: vec![1; W * H * 4], uploads: 0, acc: 0., paused: false, speed: 1, tool: Tool::Dig, brush: 4, recipe: 0, station: 0, frame: 0, last_light: 0, msg: String::new(), help: true, dev: 0, ev: vec![], shake_off: Vec2::ZERO, autosave: 0., last_oy: oy0, far, far_done, inv_open: false, aim: None });
}

fn make_bg(w: &World) -> Bg {
    let mk = |k, f| (art::layer(k, w.oy, &w.info.biome, W, H, PAD, 1, w.info.seed).0, f);
    Bg { layers: vec![mk(art::Layer::Strata, 0.8), mk(art::Layer::Caves, 0.6), mk(art::Layer::Veins, 0.35)] }
}
/// Hotbar slots (1-9, 0). LMB always mines; RMB uses the selected slot.
const HOTBAR: [Tool; 10] = [Tool::Dig, Tool::Wood, Tool::Brick, Tool::Iron, Tool::Torch, Tool::Wire, Tool::Device, Tool::Farm, Tool::Tnt, Tool::Station];
fn slot_name(g: &Game, t: Tool) -> String {
    match t { Tool::Dig => "Claws/pick".into(), Tool::Wood => "Wood pillar".into(), Tool::Brick => "Brick".into(), Tool::Iron => "Iron beam".into(), Tool::Torch => "Torch".into(), Tool::Wire => "Copperline".into(),
        Tool::Device => format!("{} (G)", DEVS[g.dev].name()), Tool::Farm => "Spore".into(), Tool::Tnt => "TNT".into(), Tool::Station => format!("{} (Tab)", STATIONS[g.station].name()), _ => format!("{:?}", t) }
}
fn slot_count(g: &Game, t: Tool) -> Option<u32> {
    let it = match t { Tool::Wood => Item::WoodPillar, Tool::Brick => Item::Brick, Tool::Iron => Item::IronBeam, Tool::Torch => Item::Torch, Tool::Wire => Item::CopperWire, Tool::Farm => Item::Spore, Tool::Tnt => Item::Tnt,
        Tool::Device => match dev_cost(DEVS[g.dev]) { Some(i) => i, None => return None }, _ => return None };
    Some(g.w.inv[it as usize])
}
fn dev_cost(d: Dev) -> Option<Item> { if d == Dev::Tnt { Some(Item::Tnt) } else if d.is_thrum() { Some(Item::Thrummer) } else if d.is_gate() { Some(Item::Gate) } else if d == Dev::Lamp { Some(Item::Lamp) } else if d == Dev::Vent { Some(Item::Vent) } else if d == Dev::Heat { Some(Item::IronIngot) } else if d == Dev::Wheel { Some(Item::Wood) } else { None } }
fn input(mut g: ResMut<Game>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, scroll: Res<AccumulatedMouseScroll>,
         win: Query<&Window, With<PrimaryWindow>>, mut cam: Query<&mut Transform, With<Camera2d>>, time: Res<Time>,
         mut commands: Commands) {
    let Ok(win) = win.single() else { return }; let Ok(mut ct) = cam.single_mut() else { return };
    let dt = time.delta_secs(); let sc = ct.scale.x;
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    // movement → the sim's input (applied every tick)
    let dead = !g.w.moles.first().map_or(false, |m| m.alive);
    g.w.input.left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    g.w.input.right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    g.w.input.jump = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::Space) || keys.pressed(KeyCode::ArrowUp);
    g.w.input.mine = None;
    // zoom: + / - or Ctrl+wheel; plain wheel cycles the hotbar
    if scroll.delta.y != 0. {
        if ctrl { let z = (sc * (1. - scroll.delta.y.signum() * 0.12)).clamp(0.12, 1.5); ct.scale = Vec3::splat(z); }
        else { let k = HOTBAR.iter().position(|&t| t == g.tool).unwrap_or(0) as i32; let n = (k - scroll.delta.y.signum() as i32).rem_euclid(10) as usize; g.tool = HOTBAR[n]; }
    }
    if keys.just_pressed(KeyCode::Equal) { ct.scale = Vec3::splat((sc * 0.8).max(0.12)); }
    if keys.just_pressed(KeyCode::Minus) { ct.scale = Vec3::splat((sc * 1.25).min(1.5)); }
    // camera follows the mole smoothly
    if let Some(m) = g.w.moles.first() {
        let p = cell_to_world(m.x as f32 + 0.5, m.y as f32 - 1.5); let k = 1. - (-7. * dt).exp();
        ct.translation.x += (p.x - ct.translation.x) * k; ct.translation.y += (p.y - ct.translation.y) * k;
    }
    let half = Vec2::new(W as f32 * CPX / 2., H as f32 * CPX / 2.);
    ct.translation.x = ct.translation.x.clamp(-half.x, half.x); { let fy = half.y + (FAR * CH) as f32 * CPX; ct.translation.y = ct.translation.y.clamp(-fy, fy); }
    let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9, KeyCode::Digit0];
    for (k, d) in digits.iter().enumerate() { if keys.just_pressed(*d) { g.tool = HOTBAR[k]; } }
    if keys.just_pressed(KeyCode::Escape) { g.paused = !g.paused; }
    if keys.just_pressed(KeyCode::KeyF) { g.speed = if g.speed >= 4 { 1 } else { g.speed * 2 }; }
    if keys.just_pressed(KeyCode::KeyI) { g.inv_open = !g.inv_open; }
    if g.inv_open {
        if keys.just_pressed(KeyCode::KeyQ) { g.recipe = (g.recipe + RECIPES.len() - 1) % RECIPES.len(); }
        if keys.just_pressed(KeyCode::KeyE) { g.recipe = (g.recipe + 1) % RECIPES.len(); }
        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::KeyC) { let r = g.recipe; g.msg = match g.w.craft_now(r) { Ok(()) => format!("Crafted {}", RECIPES[r].name), Err(e) => format!("Can't craft {}: {e}", RECIPES[r].name) }; }
    }
    if keys.just_pressed(KeyCode::Tab) { g.station = (g.station + 1) % 4; g.tool = Tool::Station; }
    if keys.just_pressed(KeyCode::KeyH) { g.help = !g.help; }
    if keys.just_pressed(KeyCode::KeyG) { if g.tool == Tool::Device { g.dev = (g.dev + 1) % DEVS.len(); } g.tool = Tool::Device; }
    if keys.just_pressed(KeyCode::KeyK) { let b = molehill_sim::save::save(&g.w); g.msg = match storage_put("molehill-manual", &b) { Ok(()) => format!("Saved ({} KB)", b.len() / 1024), Err(e) => format!("Save failed: {e}") }; }
    if keys.just_pressed(KeyCode::KeyL) {
        match storage_get("molehill-manual").or_else(|| storage_get("molehill-auto")).map(|b| molehill_sim::save::load(&b)) {
            Some(Ok(w)) => { g.w = w; g.last_oy = g.w.oy; commands.insert_resource(make_bg(&g.w)); g.buf.iter_mut().for_each(|v| *v = 1); g.msg = "Loaded".into(); g.far_done.iter_mut().for_each(|v| *v = false); }
            Some(Err(e)) => g.msg = format!("Load failed: {e}"), None => g.msg = "No save yet (K to save)".into() }
    }
    if keys.just_pressed(KeyCode::KeyB) { let kind = [Kind2::Flood, Kind2::Lava, Kind2::Quake][(g.frame % 3) as usize]; let mut d = std::mem::replace(&mut g.w.disasters, molehill_sim::disasters::Disasters::new(0)); let ok = d.schedule(&mut g.w, kind); g.w.disasters = d; if !ok { g.msg = "No site for that disaster".into(); } }
    if keys.just_pressed(KeyCode::KeyN) {
        let seed = seed_now() % 100000; g.w = make_world(seed); g.last_oy = g.w.oy; g.far_done.iter_mut().for_each(|v| *v = false);
        commands.insert_resource(make_bg(&g.w)); g.buf.iter_mut().for_each(|v| *v = 1);
        let p = cell_to_world(g.w.moles[0].x as f32, g.w.moles[0].y as f32 - 1.5); ct.translation.x = p.x; ct.translation.y = p.y;
        g.msg = format!("New world, seed {}", seed);
    }
    if dead { return; }
    // S: dig straight down
    let (mx, my) = (g.w.moles[0].x, g.w.moles[0].y);
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) { g.w.input.mine = Some((mx, my + 1)); }
    let Some(cur) = win.cursor_position() else { g.aim = None; return };
    let wp = Vec2::new(ct.translation.x + (cur.x - win.width() / 2.) * sc, ct.translation.y - (cur.y - win.height() / 2.) * sc);
    let (cx, cy) = world_to_cell(wp); if cx < 0 || cy < 0 || cx as usize >= W || cy as usize >= H { g.aim = None; return; }
    let (cx, cy) = (cx as usize, cy as usize);
    let reach = g.w.in_reach(cx, cy); g.aim = Some((cx, cy, reach));
    let blk = g.w.logic.block_of(cx, cy);
    if mouse.pressed(MouseButton::Left) { g.w.input.mine = Some((cx, cy)); }
    // E / R: use what's under the cursor (lever, thrum channel, ticker) or open a chest
    if (keys.just_pressed(KeyCode::KeyR) || (!g.inv_open && keys.just_pressed(KeyCode::KeyE))) && reach {
        if g.w.logic.dev[blk] != 0 { g.w.logic.interact(blk); g.w.events.push(sw::Event::Click); }
    }
    if !reach { if mouse.just_pressed(MouseButton::Right) { g.msg = "Out of reach".into(); } return; }
    let t = g.tool;
    // RMB places the selected item; Ctrl+RMB on copperline/devices picks them back up
    if mouse.pressed(MouseButton::Right) && ctrl && matches!(t, Tool::Wire | Tool::Device) { let gw = &mut g.w; if let Some(d) = gw.logic.remove(&mut gw.g, blk) { if let Some(it) = dev_cost(d) { gw.inv[it as usize] += 1; } } return; }
    if mouse.pressed(MouseButton::Right) {
        match t {
            Tool::Wood | Tool::Brick | Tool::Iron => { let it = match t { Tool::Wood => Item::WoodPillar, Tool::Brick => Item::Brick, _ => Item::IronBeam }; if let Err(e) = g.w.place_block(cx, cy, it) { if mouse.just_pressed(MouseButton::Right) { g.msg = format!("Can't place: {e}"); } } }
            Tool::Wire => { if g.w.logic.wire[blk] == 0 { if g.w.inv[Item::CopperWire as usize] > 0 { g.w.inv[Item::CopperWire as usize] -= 1; g.w.logic.place_wire(blk); } else { g.msg = "No copperline - draw it at an anvil".into(); } } }
            _ => {}
        }
    }
    if mouse.just_pressed(MouseButton::Right) {
        match t {
            Tool::Device => {
                let d = DEVS[g.dev]; let cost = dev_cost(d);
                if g.w.logic.dev[blk] != 0 { g.w.logic.interact(blk); g.w.events.push(sw::Event::Click); }
                else if cost.map_or(true, |it| g.w.inv[it as usize] > 0) { let gw = &mut g.w; if gw.logic.place(&mut gw.g, d, blk) { if let Some(it) = cost { gw.inv[it as usize] -= 1; } gw.events.push(sw::Event::Place); } else { g.msg = "Needs open space".into(); } }
                else { g.msg = format!("No {} in your pack", cost.unwrap().name()); }
            }
            Tool::Farm => { g.msg = match g.w.plant(cx, cy) { Ok(()) => "Spore planted - keep it dark and damp".into(), Err(e) => format!("Can't plant: {e}") }; }
            Tool::Torch => { if !g.w.place_torch(cx, cy) { g.msg = "No torches (craft at a workbench) or not open space".into(); } }
            Tool::Tnt => { if g.w.inv[Item::Tnt as usize] > 0 { g.w.inv[Item::Tnt as usize] -= 1; g.w.explode(cx, cy, 9., 6); } else { g.msg = "No TNT".into(); } }
            Tool::Station => { let s = STATIONS[g.station]; g.msg = match g.w.place_building(s, cx.saturating_sub(Station::W / 2), cy) { Ok(()) => format!("Built a {}", s.name()), Err(e) => format!("Can't build {}: {}", s.name(), e) }; }
            _ => {}
        }
    }
}

fn simulate(mut commands: Commands, mut g: ResMut<Game>, time: Res<Time>, mut cam: Query<(&mut Transform, &Projection), With<Camera2d>>, win: Query<&Window, With<PrimaryWindow>>) {
    g.frame += 1;
    if !g.paused {
        g.acc += time.delta_secs().min(0.1) * 60. * g.speed as f32;
        let mut n = 0; while g.acc >= 1. && n < 3 * g.speed { let pre = std::mem::take(&mut g.w.events); g.w.step(); let mut e = std::mem::take(&mut g.w.events); let gg = &mut *g; gg.ev.extend(pre); gg.ev.append(&mut e); g.acc -= 1.; n += 1; }
        g.autosave += time.delta_secs();
        if g.autosave > 30. { g.autosave = 0.; let b = molehill_sim::save::save(&g.w); let _ = storage_put("molehill-auto", &b); }
        if g.acc > 4. { g.acc = 0.; }
    }
    let (Ok((mut ct, _)), Ok(win)) = (cam.single_mut(), win.single()) else { return };
    if g.w.oy != g.last_oy { // window slid: keep the camera on the same rock, redraw everything
        let d = (g.w.oy - g.last_oy) as f32; ct.translation.y += d * CPX; g.last_oy = g.w.oy;
        commands.insert_resource(make_bg(&g.w)); g.buf.iter_mut().for_each(|v| *v = 1); g.far_done.iter_mut().for_each(|v| *v = false);
    }
    // camera shake (quakes, explosions): remove last frame's offset, add a new one
    let off = g.shake_off; ct.translation.x -= off.x; ct.translation.y -= off.y;
    let sh = g.w.shake; let f = g.frame as f32;
    g.shake_off = if sh > 0.05 { Vec2::new((f * 1.7).sin(), (f * 2.3).cos()) * sh * 3. * ct.scale.x.max(0.3) } else { Vec2::ZERO };
    ct.translation.x += g.shake_off.x; ct.translation.y += g.shake_off.y;
    let ct = &*ct;
    if g.frame - g.last_light >= 2 {
        g.last_light = g.frame;
        let r = view_rect(ct, win, 24);
        g.w.compute_light(r);
    }
}
fn view_rect(ct: &Transform, win: &Window, margin: i32) -> (usize, usize, usize, usize) {
    let sc = ct.scale.x; let hw = win.width() / 2. * sc; let hh = win.height() / 2. * sc;
    let (x0, y0) = world_to_cell(Vec2::new(ct.translation.x - hw, ct.translation.y + hh));
    let (x1, y1) = world_to_cell(Vec2::new(ct.translation.x + hw, ct.translation.y - hh));
    ((x0 - margin).clamp(0, W as i32 - 1) as usize, (y0 - margin).clamp(0, H as i32 - 1) as usize, (x1 + margin).clamp(0, W as i32 - 1) as usize, (y1 + margin).clamp(0, H as i32 - 1) as usize)
}

fn render_world(mut gm: ResMut<Game>, bg: Res<Bg>, mut images: ResMut<Assets<Image>>, cam: Query<&Transform, With<Camera2d>>, win: Query<&Window, With<PrimaryWindow>>) {
    let (Ok(ct), Ok(win)) = (cam.single(), win.single()) else { return };
    let (x0, y0, x1, y1) = view_rect(ct, win, 2);
    // snap the composited rect to whole chunks
    let (x0, y0) = (x0 / CH * CH, y0 / CH * CH); let (x1, y1) = (((x1 / CH) + 1) * CH - 1, ((y1 / CH) + 1) * CH - 1);
    let (x1, y1) = (x1.min(W - 1), y1.min(H - 1));
    let mut buf = std::mem::take(&mut gm.buf);
    let mut changed = vec![false; (W / CH) * (H / CH)];
    let g = &*gm;
    let lr = g.w.light_rect;
    let wd = &g.w; let gr = &wd.g;
    let camx = (ct.translation.x / CPX) as f32; // cells from world centre
    let bw = W + 2 * PAD;
    let (pmx, pmy) = wd.moles.first().filter(|m| m.alive).map(|m| (m.x as f32, m.y as f32 - 1.5)).unwrap_or((-999., -999.));
    let sample = |layer: &Vec<u8>, f: f32, x: usize, y: usize| -> [f32; 4] {
        let u = (x as f32 - camx * f) as i32 + PAD as i32; let u = u.clamp(0, bw as i32 - 1) as usize; let o = (y * bw + u) * 4;
        [layer[o] as f32, layer[o + 1] as f32, layer[o + 2] as f32, layer[o + 3] as f32 / 255.]
    };
    let over = |c: [f32; 3], s: [f32; 4]| [c[0] + (s[0] - c[0]) * s[3], c[1] + (s[1] - c[1]) * s[3], c[2] + (s[2] - c[2]) * s[3]];
    for y in y0..=y1 { for x in x0..=x1 {
        let i = y * W + x; let o = i * 4;
        let m = gr.m(i); let p = &PROPS[m as usize];
        let seen = gr.flags[i] & F_SEEN != 0;
        let px: [u8; 4];
        if !seen {
            let n = (hash3(x as u32 / 2, y as u32 / 2, 5) & 7) as u8;
            px = [14 + n, 12 + n, 17 + n, 255];
        } else {
            let lit = x >= lr.0 && x <= lr.2 && y >= lr.1 && y <= lr.3;
            let l = if lit { wd.light.at(i) } else { [0.05; 3] };
            let l = [l[0].max(0.22), l[1].max(0.21), l[2].max(0.24)]; // remembered rock stays faintly visible
            // the mole's own warm glow (its whiskers' sense of the rock nearby)
            let dd = ((x as f32 - pmx).powi(2) + (y as f32 - pmy).powi(2)).sqrt(); let gl = (1. - dd / 22.).max(0.) * 0.75;
            let l = [l[0].max(gl), l[1].max(gl * 0.92), l[2].max(gl * 0.8)];
            match p.kind {
                Kind::Empty | Kind::Gas => {
                    let lum = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).min(1.3);
                    let mut c: [f32; 3] = [44., 34., 30.];
                    for (ly, f) in &bg.layers { c = over(c, sample(ly, *f, x, y)); }
                    // darkened by light, never pure black; coloured light tints the rock behind
                    let k = 0.16 + 0.84 * (lum * 0.8).min(1.);
                    let tint = [l[0] / lum.max(0.05), l[1] / lum.max(0.05), l[2] / lum.max(0.05)];
                    let tw = (lum * 0.6).min(0.5);
                    for j in 0..3 { c[j] = c[j] * k * (1. - tw + tw * tint[j].min(1.6)) * 0.72; }
                    // stale air: a sickly olive haze where CO2 has built up
                    let co2 = wd.air.at(x, y); if co2 > 0.2 { c = over(c, [96., 104., 52., ((co2 - 0.2) * 0.9).min(0.55)]); }
                    if m == Mat::Steam { c = over(c, [200., 200., 210., 0.4]); }
                    if m == Mat::Fire { c = [255., 150., 50.]; }
                    px = [c[0].min(255.) as u8, c[1].min(255.) as u8, c[2].min(255.) as u8, 255];
                }
                _ => {
                    let v = (hash3(x as u32, y as u32, m as u32) & 255) as f32 / 255.;
                    let v2 = (hash3(x as u32 / 3, y as u32 / 2, m as u32 + 77) & 255) as f32 / 255.;
                    let mut k = 0.86 + v * 0.14 + (v2 - 0.5) * 0.12;
                    if m == Mat::Grass { k *= 0.9 + 0.2 * v; }
                    // soft shading: lit top faces, shadowed undersides
                    if y > 0 && !gr.is_solidish(i - W) { k *= 1.18; } else if y + 1 < H && !gr.is_solidish(i + W) { k *= 0.78; }
                    if p.kind == Kind::Powder || gr.loose(i) { k *= 0.93 + 0.1 * v; }
                    if wd.st.pending[i] > 0 && (hash3(x as u32, y as u32, 9) % 3 == 0) { k *= 0.45; }
                    let emit = p.emit; let em = (emit[0] as u16 + emit[1] as u16 + emit[2] as u16) > 0;
                    let lm = |c: f32, li: f32| if em { c * (0.8 + 0.4 * v) } else { c * (li.min(1.25) * 0.94 + 0.06) };
                    let mut c = [lm(p.color[0] as f32 * k, l[0]), lm(p.color[1] as f32 * k, l[1]), lm(p.color[2] as f32 * k, l[2])];
                    let mut alpha = 255.;
                    if m == Mat::Water { c = [lm(40. + 30. * v2, l[0]), lm(90. + 20. * v2, l[1]), lm(170., l[2])]; alpha = 255.; }
                    if m == Mat::Lava { let f = 0.85 + 0.3 * ((x as f32 * 0.3 + y as f32 * 0.2 + wd.tick as f32 * 0.05).sin() * 0.5 + 0.5); c = [255. * f, 110. * f, 30. * f]; }
                    if m == Mat::Glass { c = [c[0] * 0.6 + 20., c[1] * 0.6 + 26., c[2] * 0.6 + 30.]; }
                    px = [c[0].min(255.) as u8, c[1].min(255.) as u8, c[2].min(255.) as u8, alpha as u8];
                }
            }
        }
        let mut px = px;
        if seen { // wiring / device overlay on the 1 m block layer
            let lg = &wd.logic; let bw = lg.bw; let bk = (y / LB) * bw + x / LB; let (lx, ly) = (x % LB, y % LB);
            let con = |c: usize| c < lg.wire.len() && (lg.wire[c] != 0 || lg.dev[c] != 0);
            if lg.dev[bk] != 0 {
                let d = Dev::from_u8(lg.dev[bk]); let on = lg.dev_s[bk] != 0 || lg.powered[bk] != 0 || lg.burn[bk] > 0;
                let base: [u8; 3] = match d { Dev::Lever => [200, 170, 90], Dev::Plate => [150, 150, 150], Dev::Sensor => [80, 150, 230], Dev::Door | Dev::Floodgate => [150, 110, 70], Dev::Pump => [90, 130, 200], Dev::Lamp => [255, 240, 170], Dev::Tnt => [220, 50, 40], Dev::Fuse => [180, 140, 100], Dev::Tx | Dev::Rx | Dev::Rep => [160, 120, 255], Dev::Vent => [170, 200, 190], Dev::Heat => [230, 90, 40], Dev::Wheel => [90, 160, 210], _ => [110, 220, 160] };
                let edge = lx == 0 || ly == 0 || lx == LB - 1 || ly == LB - 1;
                if !(matches!(d, Dev::Door | Dev::Floodgate) && !edge) {
                    let k = if edge { 0.55 } else if on { 1.15 } else { 0.7 };
                    px = [(base[0] as f32 * k).min(255.) as u8, (base[1] as f32 * k).min(255.) as u8, (base[2] as f32 * k).min(255.) as u8, 255];
                    if d.is_thrum() && !edge && lx == 1 && ly == 1 { let ch = lg.dev_p[bk] as u8; px = [40 + ch * 25, 220 - ch * 20, 120, 255]; }
                }
            } else if lg.wire[bk] != 0 {
                let mid = |v: usize| v == 1 || v == 2;
                let draw = (mid(lx) && mid(ly)) || (mid(ly) && ((lx == 3 && x / LB + 1 < bw && con(bk + 1)) || (lx == 0 && x / LB > 0 && con(bk - 1))))
                    || (mid(lx) && ((ly == 3 && con(bk + bw)) || (ly == 0 && bk >= bw && con(bk - bw))));
                if draw { px = if lg.net_lit(bk) { [255, 205, 95, 255] } else { [150, 82, 45, 255] }; }
            }
        }
        if buf[o..o + 4] != px { buf[o..o + 4].copy_from_slice(&px); changed[(y / CH) * (W / CH) + x / CH] = true; }
    } }
    let mut n = 0;
    for (k, &c) in changed.iter().enumerate() {
        if !c { continue; } n += 1;
        let (cx, cy) = (k % (W / CH), k / (W / CH));
        let Some(mut im) = images.get_mut(&gm.chunks[k]) else { continue };
        let Some(data) = im.data.as_mut() else { continue };
        for r in 0..CH { let src = ((cy * CH + r) * W + cx * CH) * 4; data[r * CH * 4..(r + 1) * CH * 4].copy_from_slice(&buf[src..src + CH * 4]); }
    }
    gm.uploads = n; gm.buf = buf;
    render_far(&mut gm, ct, win, &mut images);
}

/// Beyond the simulated window: rows the colony has seen are drawn from the row cache (dimmed, frozen);
/// the rest is the Deepholt's rock as a mole senses it through the stone (dim silhouettes of caves).
fn render_far(gm: &mut Game, ct: &Transform, win: &Window, images: &mut Assets<Image>) {
    let sc = ct.scale.x; let (hw, hh) = (win.width() / 2. * sc, win.height() / 2. * sc);
    let (vx0, vy0) = world_to_cell(Vec2::new(ct.translation.x - hw, ct.translation.y + hh));
    let (vx1, vy1) = world_to_cell(Vec2::new(ct.translation.x + hw, ct.translation.y - hh));
    let mut budget = 3;
    for k in 0..gm.far.len() {
        if gm.far_done[k] { continue; }
        let (b, cx) = (k / (W / CH), k % (W / CH));
        let y0 = if b < FAR { (b as i32 - FAR as i32) * CH as i32 } else { (H + (b - FAR) * CH) as i32 };
        let x0 = (cx * CH) as i32;
        if x0 + CH as i32 <= vx0 - 8 || x0 > vx1 + 8 || y0 + CH as i32 <= vy0 - 8 || y0 > vy1 + 8 { continue; }
        if budget == 0 { break; } budget -= 1;
        let w = &gm.w; let mut data = vec![0u8; CH * CH * 4];
        for r in 0..CH { let wy = w.oy + (y0 + r as i32) as i64; let row = w.cache.get(&wy);
            for c in 0..CH { let x = cx * CH + c;
                let (m, seen) = match row { Some((m, f)) => (m[x], f[x] & F_SEEN != 0), None => (w.gen.cell(x, wy, W).0, false) };
                let mm = Mat::from_u8(m); let p = &PROPS[m as usize]; let v = (hash3(x as u32, wy as u32, m as u32) & 255) as f32 / 255.;
                let px: [f32; 3] = if seen {
                    match p.kind { Kind::Empty | Kind::Gas => [30., 24., 22.], _ => { let k = 0.5 + 0.08 * v; [p.color[0] as f32 * k, p.color[1] as f32 * k, p.color[2] as f32 * k] } }
                } else {
                    let n = (hash3(x as u32 / 2, wy as u32 / 2, 5) & 7) as f32;
                    match p.kind { Kind::Empty | Kind::Gas => [8. + n, 7. + n, 11. + n], _ if mm == Mat::Lava => [70. + n, 30., 16.], _ if mm == Mat::Water => [16., 24. + n, 44.],
                        _ => { let k = 0.16 + 0.03 * v; [p.color[0] as f32 * k + 12. + n, p.color[1] as f32 * k + 10. + n, p.color[2] as f32 * k + 15. + n] } }
                };
                let o = (r * CH + c) * 4; data[o] = px[0] as u8; data[o + 1] = px[1] as u8; data[o + 2] = px[2] as u8; data[o + 3] = 255;
            } }
        if let Some(mut im) = images.get_mut(&gm.far[k]) { if let Some(d) = im.data.as_mut() { d.copy_from_slice(&data); } }
        gm.far_done[k] = true;
    }
}

fn sync_sprites(mut commands: Commands, g: Res<Game>, sp: Res<Sprites>, mut q: Query<(Entity, &Pooled, &mut Transform, &mut Sprite, &mut Visibility)>) {
    // kind 0 moles, 1 drops, 2 chests, 3 stations, 4 torches, 5 mushroom plots
    let w = &g.w;
    let mut want: Vec<(u8, usize, Vec2, f32, Handle<Image>, Vec2, bool, Color)> = vec![];
    let lightf = |x: usize, y: usize| { let l = w.light.at(y.min(H - 1) * W + x.min(W - 1)); let v = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).clamp(0.12, 1.2); Color::srgb(v.min(1.) , v.min(1.), v.min(1.)) };
    for (k, m) in w.moles.iter().enumerate() { if !m.alive { continue; }
        let walking = w.input.left || w.input.right || w.pl.mine.is_some();
        let c = if m.hp < 40. { Color::srgb(1., 0.6, 0.6) } else { lightf(m.x, m.y - 1) };
        want.push((0, k, cell_to_world(m.x as f32 + 0.5, m.y as f32 + 1.0 - 1.6), 5., sp.mole[if walking { ((m.anim / 6) % 2) as usize } else { 0 }].clone(), Vec2::new(26., 19.5), m.facing < 0, c)); }
    for (k, d) in w.drops.iter().enumerate() { if w.g.flags[d.y * W + d.x] & F_SEEN == 0 { continue; } want.push((1, k, cell_to_world(d.x as f32 + 0.5, d.y as f32 + 0.2), 4., sp.icons[d.item as usize].clone(), Vec2::splat(12.), false, lightf(d.x, d.y))); }
    for (k, c) in w.chests.iter().enumerate() { if w.g.flags[c.y * W + c.x] & F_SEEN == 0 || w.g.is_solidish(c.y * W + c.x) && !c.opened { continue; } want.push((2, k, cell_to_world(c.x as f32 + 0.5, c.y as f32 - 0.6), 3., sp.chest[c.opened as usize].clone(), Vec2::new(16., 13.), false, lightf(c.x, c.y))); }
    for (k, b) in w.buildings.iter().enumerate() { want.push((3, k, cell_to_world(b.x as f32 + Station::W as f32 / 2., b.y as f32 + 1. - Station::H as f32 / 2.), 2., sp.station[b.kind as usize][b.built as usize].clone(), Vec2::new(Station::W as f32 * CPX, Station::H as f32 * CPX), false, lightf(b.x + 5, b.y - 2))); }
    for (k, &(x, y)) in w.torches.iter().enumerate() { want.push((4, k, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 4.5, sp.torch.clone(), Vec2::splat(10.), false, Color::WHITE)); }
    for (k, f) in w.farms.iter().enumerate() { let st = if f.growth < 0.3 { 0 } else if f.growth < 0.85 { 1 } else { 2 }; want.push((5, k, cell_to_world(f.x as f32 + 0.5, f.y as f32 - 0.9), 3.5, sp.shroom[st].clone(), Vec2::splat(10.), false, lightf(f.x, f.y))); }
    // aim: a faint outline on the cell under the cursor; mining: the targeted cell brightens and cracks as it gives way
    if let Some((x, y, reach)) = g.aim { let c = if reach { Color::srgba(1., 1., 0.85, 0.22) } else { Color::srgba(1., 0.4, 0.3, 0.10) }; want.push((7, 0, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 6., sp.px.clone(), Vec2::splat(CPX), false, c)); }
    if let Some((i, wk, need)) = w.pl.mine { let (x, y) = (i % W, i / W); let f = wk as f32 / need.max(1) as f32;
        want.push((6, 0, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 6.1, sp.px.clone(), Vec2::splat(CPX * (1. - 0.35 * f)), false, Color::srgba(1., 0.9, 0.6, 0.35 - 0.2 * f))); }
    let mut used = std::collections::HashSet::new();
    for (e, p, mut t, mut s, mut vis) in &mut q {
        if let Some(wn) = want.iter().find(|wn| wn.0 == p.0 && wn.1 == p.1) {
            t.translation = wn.2.extend(wn.3); s.image = wn.4.clone(); s.custom_size = Some(wn.5); s.flip_x = wn.6; s.color = wn.7; *vis = Visibility::Visible; used.insert((p.0, p.1));
        } else { commands.entity(e).despawn(); }
    }
    for wn in want { if !used.contains(&(wn.0, wn.1)) { commands.spawn((Sprite { image: wn.4, custom_size: Some(wn.5), flip_x: wn.6, color: wn.7, ..default() }, Transform::from_translation(wn.2.extend(wn.3)), Pooled(wn.0, wn.1))); } }
}

fn sound(mut g: ResMut<Game>, mut au: NonSendMut<audio::Audio>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>) {
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() { au.unlock(); }
    let ev = std::mem::take(&mut g.ev);
    for e in ev { au.play(e, if e == sw::Event::Dig { 0.6 } else { 1. }); }
    if g.frame % 30 == 0 { let d = g.w.max_depth / 60.; au.ambient(d); }
}
fn hud(g: Res<Game>, mut q: Query<&mut Text, With<Hud>>, mut qr: Query<&mut Text, (With<HudRight>, Without<Hud>)>, mut qb: Query<&mut Text, (With<HudBottom>, Without<Hud>, Without<HudRight>)>, time: Res<Time>) {
    if g.frame % 6 != 0 { return; }
    let w = &g.w;
    let me = &w.moles[0];
    let view_d = w.depth_m(me.x, me.y);
    let mut s = format!("MOLEHILL - the Deepholt  seed {}   fps {:.0}   {}{}\n{}  at {:+.1} m   deepest {:+.1} m   tools: {}   deaths {}\n",
        w.info.seed, 1. / time.delta_secs().max(1e-3), if g.paused { "PAUSED " } else { "" }, if g.speed > 1 { format!("x{}", g.speed) } else { String::new() },
        me.name, view_d, w.max_depth, ["claws", "stone", "bronze", "iron"][craft::tool_tier(&w.inv) as usize], w.pl.deaths);
    if !me.alive { s += &format!("** You were crushed/burnt/starved. Back to the burrow in {:.0}s **\n", (150u32.saturating_sub(w.pl.dead_t)) as f32 / 60.); }
    let bar = |v: f32| { let n = (v / 10.).round().clamp(0., 10.) as usize; format!("{}{}", "#".repeat(n), ".".repeat(10 - n)) };
    let c = w.air.at(me.x, me.y.saturating_sub(1));
    s += &format!("HP [{}] {:>3.0}   Food [{}] {:>3.0}   Breath [{}] {:>3.0}  air {}\n", bar(me.hp), me.hp, bar(me.hunger), me.hunger, bar(me.breath), me.breath, if c > 0.45 { "STALE" } else if c > 0.25 { "close" } else { "fresh" });
    if w.quest < QUESTS.len() { let q = &QUESTS[w.quest]; s += &format!("Quest {}/{}: {}{}\n", w.quest + 1, QUESTS.len(), q.text, if w.quest_done(w.quest) { " [done]" } else { "" }); } else { s += "All quests complete!\n"; }
    let d = &w.disasters;
    match d.phase { Phase::Warning => s += &format!("!! {} in {:.0}s !!\n", d.kind.name(), d.seconds_left()), Phase::Active => s += &format!("!! {}! ({:.0}s) !!\n", d.kind.name().to_uppercase(), d.seconds_left()), Phase::Calm => {} }
    if let Some((i, wk, need)) = w.pl.mine { s += &format!("Mining {} {:.0}%\n", w.g.m(i).p().name, wk as f32 * 100. / need as f32); }
    // hotbar
    s += "\n"; for (k, &t) in HOTBAR.iter().enumerate() { let sel = t == g.tool; let n = slot_count(&g, t).map(|n| format!(" {n}")).unwrap_or_default(); s += &format!("{}{} {}{}{} ", if sel { "[" } else { " " }, (k + 1) % 10, slot_name(&g, t), n, if sel { "]" } else { " " }); if k == 4 { s += "\n"; } }
    s += "\n";
    if !g.inv_open { if let Ok(mut t) = q.single_mut() { t.0 = s.clone(); } }
    if g.inv_open { s += "\nPack (I to close):\n";
    let mut line = String::new(); let mut n = 0;
    for k in 0..NITEM { if w.inv[k] > 0 { line += &format!("{} {}   ", ITEMS[k].name(), w.inv[k]); n += 1; if n % 4 == 0 { s += &line; s += "\n"; line.clear(); } } }
    s += &line; if let Ok(mut t) = q.single_mut() { t.0 = s; } }
    let r = &RECIPES[g.recipe];
    let inputs: Vec<String> = r.inputs.iter().map(|&(it, n)| format!("{} {}/{}", it.name(), w.inv[it as usize].min(n), n)).collect();
    let has = w.buildings.iter().any(|b| b.built && b.kind == r.station);
    let mut rs = if g.inv_open { format!("Craft (Q/E pick, Enter craft - stand near the station):\n  {} @ {}{}\n  needs {}\n\n", r.name, r.station.name(), if has { "" } else { " (not built)" }, inputs.join(", ")) } else { "I: pack & crafting\n\n".to_string() };
    let st = STATIONS[g.station]; rs += &format!("Station (Tab, slot 0): {} - {}\n\n", st.name(), st.cost().iter().map(|&(it, n)| format!("{} {}", it.name(), n)).collect::<Vec<_>>().join(", "));
    for (t, l) in w.log.iter().rev().take(8) { let _ = t; rs += &format!("* {}\n", l); }
    if !g.msg.is_empty() { rs += &format!("\n> {}", g.msg); }
    if let Ok(mut t) = qr.single_mut() { t.0 = rs; }
    let help = if g.help { "A/D walk   W/Space jump (climbs 1-cell steps)   S dig down   LMB hold: mine under cursor (reach 4.5 m)   RMB: place selected   1-9,0 / wheel: hotbar   Ctrl+wheel or +/-: zoom\nI pack & crafting (Q/E pick, Enter craft near the station)   Tab station type   G device type   E/R use lever/thrum/ticker   Ctrl+RMB pick up line/device   Esc pause  F speed  K save  L load  N new world  B disaster  H hide help" } else { "H for help" };
    if let Ok(mut t) = qb.single_mut() { t.0 = help.into(); }
}
