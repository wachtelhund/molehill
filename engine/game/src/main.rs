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

#[derive(Resource)] struct Game { w: World, chunks: Vec<Handle<Image>>, buf: Vec<u8>, uploads: usize, acc: f32, paused: bool, speed: u32, tool: Tool, brush: usize, recipe: usize, station: usize, frame: u64, last_light: u64, msg: String, help: bool, dev: usize, ev: Vec<sw::Event>, shake_off: Vec2, autosave: f32 }
#[derive(Clone, Copy, PartialEq, Debug)] enum Tool { Wire, Device, Dig, Wood, Brick, Iron, Torch, Tnt, Station, Cancel, Stockpile }
#[derive(Resource)] struct Bg { layers: Vec<(Vec<u8>, f32)>, hills: Vec<(Vec<u8>, f32)> }
#[derive(Component)] struct WorldSprite;
#[derive(Component)] struct Hud; #[derive(Component)] struct HudRight; #[derive(Component)] struct HudBottom;
#[derive(Component)] struct Pooled(u8, usize);
#[derive(Resource)] struct Sprites { mole: [Handle<Image>; 2], icons: Vec<Handle<Image>>, chest: [Handle<Image>; 2], station: Vec<[Handle<Image>; 2]>, torch: Handle<Image> }

fn seed_now() -> u32 {
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

fn make_world(seed: u32) -> World { World::new(W, H, seed) }

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let seed = seed_now() % 100000;
    let w = make_world(seed);
    // The world is split into CH×CH-cell chunk textures; only chunks whose composited pixels changed
    // are touched (Assets::get_mut marks an image for re-upload), so a quiet frame uploads nothing.
    let mut chunks = vec![];
    for cy in 0..H / CH { for cx in 0..W / CH {
        let img = images.add(art::image(CH as u32, CH as u32, vec![0; CH * CH * 4], true));
        let p = cell_to_world((cx * CH) as f32 + CH as f32 / 2., (cy * CH) as f32 + CH as f32 / 2.);
        commands.spawn((Sprite { image: img.clone(), custom_size: Some(Vec2::splat(CH as f32 * CPX)), ..default() }, Transform::from_xyz(p.x, p.y, 0.), WorldSprite));
        chunks.push(img);
    } }
    commands.insert_resource(make_bg(&w));
    let (sx, sy) = (w.moles[0].x as f32, w.moles[0].y as f32 - 10.);
    let p = cell_to_world(sx, sy);
    commands.spawn((Camera2d, Transform::from_xyz(p.x, p.y, 100.).with_scale(Vec3::splat(0.45))));
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
    };
    commands.insert_resource(sp);
    commands.insert_resource(Game { w, chunks, buf: vec![1; W * H * 4], uploads: 0, acc: 0., paused: false, speed: 1, tool: Tool::Dig, brush: 4, recipe: 0, station: 0, frame: 0, last_light: 0, msg: String::new(), help: true, dev: 0, ev: vec![], shake_off: Vec2::ZERO, autosave: 0. });
}

fn make_bg(w: &World) -> Bg {
    let mk = |k, f| (art::layer(k, &w.info.surface, &w.info.biome, W, H, PAD, 1, w.info.seed).0, f);
    Bg { hills: vec![mk(art::Layer::HillsFar, 0.85), mk(art::Layer::HillsNear, 0.65)],
         layers: vec![mk(art::Layer::Strata, 0.8), mk(art::Layer::Caves, 0.6), mk(art::Layer::Roots, 0.35)] }
}
fn input(mut g: ResMut<Game>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, scroll: Res<AccumulatedMouseScroll>,
         win: Query<&Window, With<PrimaryWindow>>, mut cam: Query<&mut Transform, With<Camera2d>>, time: Res<Time>,
         mut commands: Commands) {
    let Ok(win) = win.single() else { return }; let Ok(mut ct) = cam.single_mut() else { return };
    let dt = time.delta_secs(); let sc = ct.scale.x;
    let pan = 900. * sc * dt * if keys.pressed(KeyCode::ShiftLeft) { 2.5 } else { 1. };
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) { ct.translation.x -= pan; }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) { ct.translation.x += pan; }
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) { ct.translation.y += pan; }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) { ct.translation.y -= pan; }
    if scroll.delta.y != 0. { let z = (sc * (1. - scroll.delta.y.signum() * 0.12)).clamp(0.12, 2.5); ct.scale = Vec3::splat(z); }
    if keys.just_pressed(KeyCode::Equal) { ct.scale = Vec3::splat((sc * 0.8).max(0.12)); }
    if keys.just_pressed(KeyCode::Minus) { ct.scale = Vec3::splat((sc * 1.25).min(2.5)); }
    let half = Vec2::new(W as f32 * CPX / 2., H as f32 * CPX / 2.);
    ct.translation.x = ct.translation.x.clamp(-half.x, half.x); ct.translation.y = ct.translation.y.clamp(-half.y, half.y + 200.);
    let tools = [(KeyCode::Digit0, Tool::Wire), (KeyCode::Digit1, Tool::Dig), (KeyCode::Digit2, Tool::Wood), (KeyCode::Digit3, Tool::Brick), (KeyCode::Digit4, Tool::Iron), (KeyCode::Digit5, Tool::Torch), (KeyCode::Digit6, Tool::Tnt), (KeyCode::Digit7, Tool::Station), (KeyCode::Digit8, Tool::Stockpile), (KeyCode::Digit9, Tool::Cancel)];
    for (k, t) in tools { if keys.just_pressed(k) { g.tool = t; } }
    if keys.just_pressed(KeyCode::Space) { g.paused = !g.paused; }
    if keys.just_pressed(KeyCode::KeyF) { g.speed = if g.speed >= 4 { 1 } else { g.speed * 2 }; }
    if keys.just_pressed(KeyCode::BracketLeft) { g.brush = g.brush.saturating_sub(1).max(1); }
    if keys.just_pressed(KeyCode::BracketRight) { g.brush = (g.brush + 1).min(12); }
    if keys.just_pressed(KeyCode::KeyQ) { g.recipe = (g.recipe + RECIPES.len() - 1) % RECIPES.len(); }
    if keys.just_pressed(KeyCode::KeyE) { g.recipe = (g.recipe + 1) % RECIPES.len(); }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::KeyC) { let r = g.recipe; g.w.queue_craft(r); g.msg = format!("Queued: {}", RECIPES[r].name); }
    if keys.just_pressed(KeyCode::Tab) { g.station = (g.station + 1) % 4; g.tool = Tool::Station; }
    if keys.just_pressed(KeyCode::KeyH) { g.help = !g.help; }
    if keys.just_pressed(KeyCode::KeyG) { if g.tool == Tool::Device { g.dev = (g.dev + 1) % DEVS.len(); } g.tool = Tool::Device; }
    if keys.just_pressed(KeyCode::KeyK) { let b = molehill_sim::save::save(&g.w); g.msg = match storage_put("molehill-manual", &b) { Ok(()) => format!("Saved ({} KB)", b.len() / 1024), Err(e) => format!("Save failed: {e}") }; }
    if keys.just_pressed(KeyCode::KeyL) {
        match storage_get("molehill-manual").or_else(|| storage_get("molehill-auto")).map(|b| molehill_sim::save::load(&b)) {
            Some(Ok(w)) => { g.w = w; commands.insert_resource(make_bg(&g.w)); g.buf.iter_mut().for_each(|v| *v = 1); g.msg = "Loaded".into(); }
            Some(Err(e)) => g.msg = format!("Load failed: {e}"), None => g.msg = "No save yet (K to save)".into() }
    }
    if keys.just_pressed(KeyCode::KeyB) { let kind = [Kind2::Flood, Kind2::Lava, Kind2::Quake][(g.frame % 3) as usize]; let mut d = std::mem::replace(&mut g.w.disasters, molehill_sim::disasters::Disasters::new(0)); let ok = d.schedule(&mut g.w, kind); g.w.disasters = d; if !ok { g.msg = "No site for that disaster".into(); } }
    if keys.just_pressed(KeyCode::KeyN) {
        let seed = seed_now() % 100000; g.w = make_world(seed);
        commands.insert_resource(make_bg(&g.w));
        let p = cell_to_world(g.w.moles[0].x as f32, g.w.moles[0].y as f32 - 10.); ct.translation.x = p.x; ct.translation.y = p.y;
        g.msg = format!("New world, seed {}", seed);
    }
    let Some(cur) = win.cursor_position() else { return };
    let wp = Vec2::new(ct.translation.x + (cur.x - win.width() / 2.) * sc, ct.translation.y - (cur.y - win.height() / 2.) * sc);
    let (cx, cy) = world_to_cell(wp); if cx < 0 || cy < 0 || cx as usize >= W || cy as usize >= H { return; }
    let (cx, cy) = (cx as usize, cy as usize);
    let (b, t) = (g.brush, g.tool);
    let blk = g.w.logic.block_of(cx, cy);
    if mouse.pressed(MouseButton::Right) { if matches!(t, Tool::Wire | Tool::Device) { let gw = &mut g.w; if let Some(d) = gw.logic.remove(&mut gw.g, blk) { if d == Dev::Tnt { gw.inv[Item::Tnt as usize] += 1; } } } else { g.w.designate(cx, cy, b, 0); } }
    if keys.just_pressed(KeyCode::KeyR) { g.w.logic.interact(blk); g.w.events.push(sw::Event::Click); }
    if mouse.pressed(MouseButton::Left) && t == Tool::Wire && g.w.logic.wire[blk] == 0 { if g.w.inv[Item::CopperWire as usize] > 0 { g.w.inv[Item::CopperWire as usize] -= 1; g.w.logic.place_wire(blk); } else { g.msg = "No copper wire - craft at a furnace".into(); } }
    if mouse.just_pressed(MouseButton::Left) && t == Tool::Device {
        let d = DEVS[g.dev];
        let cost = if d == Dev::Tnt { Some(Item::Tnt) } else if d.is_lora() { Some(Item::LoraNode) } else if d.is_gate() { Some(Item::Gate) } else if d == Dev::Lamp { Some(Item::Lamp) } else { None };
        if g.w.logic.dev[blk] != 0 { g.w.logic.interact(blk); g.w.events.push(sw::Event::Click); }
        else if cost.map_or(true, |it| g.w.inv[it as usize] > 0) { let gw = &mut g.w; if gw.logic.place(&mut gw.g, d, blk) { if let Some(it) = cost { gw.inv[it as usize] -= 1; } gw.events.push(sw::Event::Place); } else { g.msg = "Needs open space".into(); } }
        else { g.msg = format!("No {} in stockpile", cost.unwrap().name()); }
    }
    if mouse.pressed(MouseButton::Left) {
        match t {
            Tool::Dig => g.w.designate(cx, cy, b, sw::D_DIG), Tool::Wood => g.w.designate(cx, cy, b.min(2), sw::D_WOOD), Tool::Brick => g.w.designate(cx, cy, b.min(2), sw::D_BRICK),
            Tool::Iron => g.w.designate(cx, cy, b.min(2), sw::D_IRON), Tool::Cancel => g.w.designate(cx, cy, b, 0), _ => {}
        }
    }
    if mouse.just_pressed(MouseButton::Left) {
        match t {
            Tool::Torch => { if !g.w.place_torch(cx, cy) { g.msg = "No torches (craft at a workbench) or not open space".into(); } }
            Tool::Tnt => { if g.w.inv[Item::Tnt as usize] > 0 { g.w.inv[Item::Tnt as usize] -= 1; g.w.explode(cx, cy, 9., 6); } else { g.msg = "No TNT - craft at workbench (2 coal + 2 sand)".into(); } }
            Tool::Station => { let s = STATIONS[g.station]; g.msg = match g.w.place_building(s, cx.saturating_sub(Station::W / 2), cy) { Ok(()) => format!("{} blueprint placed", s.name()), Err(e) => format!("Can't place {}: {}", s.name(), e) }; }
            Tool::Stockpile => { g.w.stockpile = (cx.saturating_sub(6), cy.saturating_sub(3), cx + 6, cy); g.msg = "Stockpile moved".into(); }
            _ => {}
        }
    }
}

fn simulate(mut g: ResMut<Game>, time: Res<Time>, mut cam: Query<(&mut Transform, &Projection), With<Camera2d>>, win: Query<&Window, With<PrimaryWindow>>) {
    g.frame += 1;
    if !g.paused {
        g.acc += time.delta_secs().min(0.1) * 60. * g.speed as f32;
        let mut n = 0; while g.acc >= 1. && n < 3 * g.speed { let pre = std::mem::take(&mut g.w.events); g.w.step(); let mut e = std::mem::take(&mut g.w.events); let gg = &mut *g; gg.ev.extend(pre); gg.ev.append(&mut e); g.acc -= 1.; n += 1; }
        g.autosave += time.delta_secs();
        if g.autosave > 30. { g.autosave = 0.; let b = molehill_sim::save::save(&g.w); let _ = storage_put("molehill-auto", &b); }
        if g.acc > 4. { g.acc = 0.; }
    }
    let (Ok((mut ct, _)), Ok(win)) = (cam.single_mut(), win.single()) else { return };
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
    let night = 1. - wd.day;
    let camx = (ct.translation.x / CPX) as f32; // cells from world centre
    let bw = W + 2 * PAD;
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
            match p.kind {
                Kind::Empty | Kind::Gas => {
                    let lum = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).min(1.3);
                    let surf = wd.surface(x);
                    let mut c: [f32; 3];
                    if y < surf {
                        let t = y as f32 / surf.max(1) as f32; let d = wd.day;
                        c = [(92. + 110. * t) * (0.25 + 0.75 * d), (150. + 76. * t) * (0.25 + 0.75 * d), (214. + 24. * t) * (0.35 + 0.65 * d)];
                        for (ly, f) in &bg.hills { let s = sample(ly, *f, x, y); c = over(c, [s[0] * (0.3 + 0.7 * d), s[1] * (0.3 + 0.7 * d), s[2] * (0.4 + 0.6 * d), s[3]]); }
                    } else {
                        c = [44., 34., 30.];
                        for (ly, f) in &bg.layers { c = over(c, sample(ly, *f, x, y)); }
                        // darkened by light, never pure black; coloured light tints the rock behind
                        let k = 0.16 + 0.84 * (lum * 0.8).min(1.);
                        let tint = [l[0] / lum.max(0.05), l[1] / lum.max(0.05), l[2] / lum.max(0.05)];
                        let tw = (lum * 0.6).min(0.5);
                        for j in 0..3 { c[j] = c[j] * k * (1. - tw + tw * tint[j].min(1.6)) * 0.72; }
                    }
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
        let d = wd.desig[i];
        if d != 0 && seen {
            let stripe = (x + y) % 4 < 2;
            let col: [u8; 3] = match d { sw::D_DIG => [255, 210, 80], sw::D_WOOD => [190, 140, 80], sw::D_BRICK => [220, 110, 80], _ => [150, 170, 220] };
            let a = if stripe { 0.45 } else { 0.22 };
            for k in 0..3 { px[k] = (px[k] as f32 * (1. - a) + col[k] as f32 * a) as u8; } 
        }
        let sp = wd.stockpile; if x >= sp.0 && x <= sp.2 && y >= sp.1 && y <= sp.3 && (x == sp.0 || x == sp.2 || y == sp.3) && !gr.is_solidish(i) { px = [(px[0] as u16 / 2 + 60) as u8, (px[1] as u16 / 2 + 100) as u8, (px[2] as u16 / 2 + 70) as u8, 255]; }
        if seen { // wiring / device overlay on the 1 m block layer
            let lg = &wd.logic; let bw = lg.bw; let bk = (y / LB) * bw + x / LB; let (lx, ly) = (x % LB, y % LB);
            let con = |c: usize| c < lg.wire.len() && (lg.wire[c] != 0 || lg.dev[c] != 0);
            if lg.dev[bk] != 0 {
                let d = Dev::from_u8(lg.dev[bk]); let on = lg.dev_s[bk] != 0 || lg.powered[bk] != 0 || lg.burn[bk] > 0;
                let base: [u8; 3] = match d { Dev::Lever => [200, 170, 90], Dev::Plate => [150, 150, 150], Dev::Sensor => [80, 150, 230], Dev::Door | Dev::Floodgate => [150, 110, 70], Dev::Pump => [90, 130, 200], Dev::Lamp => [255, 240, 170], Dev::Tnt => [220, 50, 40], Dev::Fuse => [180, 140, 100], Dev::Tx | Dev::Rx | Dev::Rep => [160, 120, 255], _ => [110, 220, 160] };
                let edge = lx == 0 || ly == 0 || lx == LB - 1 || ly == LB - 1;
                if !(matches!(d, Dev::Door | Dev::Floodgate) && !edge) {
                    let k = if edge { 0.55 } else if on { 1.15 } else { 0.7 };
                    px = [(base[0] as f32 * k).min(255.) as u8, (base[1] as f32 * k).min(255.) as u8, (base[2] as f32 * k).min(255.) as u8, 255];
                    if d.is_lora() && !edge && lx == 1 && ly == 1 { let ch = lg.dev_p[bk] as u8; px = [40 + ch * 25, 220 - ch * 20, 120, 255]; }
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
}

fn sync_sprites(mut commands: Commands, g: Res<Game>, sp: Res<Sprites>, mut q: Query<(Entity, &Pooled, &mut Transform, &mut Sprite, &mut Visibility)>) {
    // kind 0 moles, 1 drops, 2 chests, 3 stations, 4 torches
    let w = &g.w;
    let mut want: Vec<(u8, usize, Vec2, f32, Handle<Image>, Vec2, bool, Color)> = vec![];
    let lightf = |x: usize, y: usize| { let l = w.light.at(y.min(H - 1) * W + x.min(W - 1)); let v = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).clamp(0.12, 1.2); Color::srgb(v.min(1.) , v.min(1.), v.min(1.)) };
    for (k, m) in w.moles.iter().enumerate() { if !m.alive { continue; }
        let walking = !m.path.is_empty() || m.work > 0;
        let c = if m.hp < 40. { Color::srgb(1., 0.6, 0.6) } else { lightf(m.x, m.y - 1) };
        want.push((0, k, cell_to_world(m.x as f32 + 0.5, m.y as f32 + 1.0 - 1.6), 5., sp.mole[if walking { ((m.anim / 6) % 2) as usize } else { 0 }].clone(), Vec2::new(26., 19.5), m.facing < 0, c)); }
    for (k, d) in w.drops.iter().enumerate() { if w.g.flags[d.y * W + d.x] & F_SEEN == 0 { continue; } want.push((1, k, cell_to_world(d.x as f32 + 0.5, d.y as f32 + 0.2), 4., sp.icons[d.item as usize].clone(), Vec2::splat(12.), false, lightf(d.x, d.y))); }
    for (k, c) in w.chests.iter().enumerate() { if w.g.flags[c.y * W + c.x] & F_SEEN == 0 || w.g.is_solidish(c.y * W + c.x) && !c.opened { continue; } want.push((2, k, cell_to_world(c.x as f32 + 0.5, c.y as f32 - 0.6), 3., sp.chest[c.opened as usize].clone(), Vec2::new(16., 13.), false, lightf(c.x, c.y))); }
    for (k, b) in w.buildings.iter().enumerate() { want.push((3, k, cell_to_world(b.x as f32 + Station::W as f32 / 2., b.y as f32 + 1. - Station::H as f32 / 2.), 2., sp.station[b.kind as usize][b.built as usize].clone(), Vec2::new(Station::W as f32 * CPX, Station::H as f32 * CPX), false, lightf(b.x + 5, b.y - 2))); }
    for (k, &(x, y)) in w.torches.iter().enumerate() { want.push((4, k, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 4.5, sp.torch.clone(), Vec2::splat(10.), false, Color::WHITE)); }
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
    let alive = w.moles.iter().filter(|m| m.alive).count();
    let tool = match g.tool { Tool::Station => format!("Station: {}", STATIONS[g.station].name()), Tool::Device => format!("Device: {}", DEVS[g.dev].name()), t => format!("{:?}", t) };
    let mut s = format!("MOLEHILL  seed {}   fps {:.0}  tex {}/128   {}{}\nMoles {}   deepest {:.1} m   tool [{}] brush {}   tier {}\n",
        w.info.seed, 1. / time.delta_secs().max(1e-3), g.uploads, if g.paused { "PAUSED " } else { "" }, if g.speed > 1 { format!("x{}", g.speed) } else { String::new() },
        alive, w.max_depth, tool, g.brush, ["claws", "stone", "bronze", "iron"][craft::tool_tier(&w.inv) as usize]);
    if w.quest < QUESTS.len() { let q = &QUESTS[w.quest]; s += &format!("Quest {}/{}: {}{}\n", w.quest + 1, QUESTS.len(), q.text, if w.quest_done(w.quest) { " [done]" } else { "" }); } else { s += "All quests complete!\n"; }
    s += &format!("Chests {}/{}   cracking {}   collapsed {}\n", w.chests_opened, w.chests.len(), w.st.cracking.len(), w.crumbled);
    let d = &w.disasters;
    match d.phase { Phase::Warning => s += &format!("!! {} in {:.0}s !!\n", d.kind.name(), d.seconds_left()), Phase::Active => s += &format!("!! {}! ({:.0}s) !!\n", d.kind.name().to_uppercase(), d.seconds_left()), Phase::Calm => {} }
    for m in w.moles.iter().filter(|m| m.alive) { s += &format!("{:<8} hp {:>3.0} food {:>3.0} sleep {:>3.0} {}\n", m.name, m.hp, m.hunger, m.sleep, match &m.job { Some(sw::Job::Sleep) => "zzz", Some(sw::Job::Eat) => "eating", Some(sw::Job::Dig) => "digging", Some(sw::Job::Build) => "building", Some(sw::Job::Haul(_)) | Some(sw::Job::Deliver) => "hauling", Some(sw::Job::Craft(_)) => "crafting", Some(sw::Job::Station(_)) => "constructing", _ => "idle" }); }
    s += "\nStockpile:\n";
    let mut line = String::new(); let mut n = 0;
    for k in 0..NITEM { if w.inv[k] > 0 { line += &format!("{} {}   ", ITEMS[k].name(), w.inv[k]); n += 1; if n % 4 == 0 { s += &line; s += "\n"; line.clear(); } } }
    s += &line;
    if let Ok(mut t) = q.single_mut() { t.0 = s; }
    let r = &RECIPES[g.recipe];
    let inputs: Vec<String> = r.inputs.iter().map(|&(it, n)| format!("{} {}/{}", it.name(), w.inv[it as usize].min(n), n)).collect();
    let has = w.buildings.iter().any(|b| b.built && b.kind == r.station);
    let mut rs = format!("Recipe (Q/E, Enter to queue):\n  {} @ {}{}\n  needs {}\nQueue: {}\n\n", r.name, r.station.name(), if has { "" } else { " (not built)" }, inputs.join(", "),
        w.craft_queue.iter().map(|&k| RECIPES[k].name).collect::<Vec<_>>().join(", "));
    let st = STATIONS[g.station]; rs += &format!("Station (Tab, tool 7): {} - {}\n\n", st.name(), st.cost().iter().map(|&(it, n)| format!("{} {}", it.name(), n)).collect::<Vec<_>>().join(", "));
    for (t, l) in w.log.iter().rev().take(8) { let _ = t; rs += &format!("* {}\n", l); }
    if !g.msg.is_empty() { rs += &format!("\n> {}", g.msg); }
    if let Ok(mut t) = qr.single_mut() { t.0 = rs; }
    let help = if g.help { "LMB designate/place   RMB clear   wheel/+- zoom   WASD pan (Shift fast)   [ ] brush   Space pause   F speed   N new world   H hide help   K save  L load  B disaster\n0 Wire  G device (cycle)  R interact (lever / LoRa channel / timer)  1 Dig  2 Wood pillar  3 Brick  4 Iron beam  5 Torch  6 TNT  7 Station (Tab cycles)  8 Move stockpile  9 Cancel" } else { "H for help" };
    if let Ok(mut t) = qb.single_mut() { t.0 = help.into(); }
}
