//! Molehill cave editor: a dev-only window for painting found-place templates (`assets/sites/*.json`).
//! `cargo run --release -p molehill-cave-editor [family]` from `engine/`. Never shipped: see
//! `docs/wiki/Cave-Editor.md`. The model and the checks live in `lib.rs`; this is only the window.
use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use molehill_cave_editor::*;
use molehill_sim::mat::Mat;
use molehill_sim::sites::Kind;
use molehill_sim::templates::{Template, ThingDef};
use molehill_sim::walls::Wall;

/// World units per cell on screen at zoom 1.
const PX: f32 = 3.;
const PANEL_W: f32 = 330.;
const LINE: f32 = 17.;
const ROWS: usize = 52;
const INK: Color = Color::srgb(0.86, 0.82, 0.74);
const ACCENT: Color = Color::srgb(0.98, 0.78, 0.36);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tool { Material, Lining, Thing, Chest }
enum Overlay { None, Reach(Reach), Stress(Vec<bool>), Sim(Sim) }
#[derive(Clone, Copy)]
enum Act { Family(usize), Tool(Tool), Pal(usize), Var(i64), Side, Brush(f32), Reach, Stress, Simulate, Clear, Undo, Redo, Save, Fit }

#[derive(Resource)]
struct Ed {
    fam: usize, doc: Doc, tpl: Option<Template>, var: u32, left: bool, tool: Tool,
    mat_i: usize, wall_i: usize, thing_i: usize, brush: f32, overlay: Overlay, msg: String,
    redraw: bool, stroke: bool, discard: Option<usize>, rows: Vec<(String, Option<Act>, bool)>,
    mats: Vec<Option<Mat>>, walls: Vec<Option<Wall>>, things: Vec<ThingDef>, img: Handle<Image>, fit: bool,
}
#[derive(Component)] struct Canvas;
#[derive(Component)] struct Row(usize);

fn main() {
    let fam = std::env::args().nth(1).and_then(|a| families().iter().position(|&k| kind_label(k) == a || Kind::from_name(&a) == Some(k))).unwrap_or(0);
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { title: "Molehill cave editor (dev only)".into(), resolution: (1600u32, 960u32).into(), ..default() }), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.045, 0.05)))
        .insert_resource(StartFam(fam))
        .add_systems(Startup, setup)
        .add_systems(Update, (input, panel, redraw).chain())
        .run();
}
#[derive(Resource)] struct StartFam(usize);

fn blank(w: usize, h: usize) -> Image {
    let mut im = Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, vec![0; w * h * 4], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
    im.sampler = ImageSampler::nearest();
    im
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, start: Res<StartFam>) {
    let fam = start.0;
    let doc = Doc::open(families()[fam]).unwrap_or_else(|e| panic!("{e}"));
    let img = images.add(blank(doc.w, doc.h));
    commands.spawn((Camera2d, Transform::from_xyz(-PANEL_W / 2., 0., 100.)));
    commands.spawn((Sprite { image: img.clone(), custom_size: Some(Vec2::new(doc.w as f32 * PX, doc.h as f32 * PX)), ..default() }, Transform::default(), Canvas));
    commands.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.), top: Val::Px(0.), width: Val::Px(PANEL_W), height: Val::Percent(100.), ..default() }, BackgroundColor(Color::srgba(0.02, 0.02, 0.025, 0.92)))).with_children(|p| {
        for i in 0..ROWS {
            p.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(10.), top: Val::Px(8. + i as f32 * LINE), ..default() },
                Text::new(""), TextFont { font_size: 13.0_f32.into(), ..default() }, TextColor(INK), Row(i)));
        }
    });
    let mut mats = vec![None]; mats.extend(paint_mats().into_iter().map(Some));
    let mut walls = vec![None]; walls.extend(paint_walls().into_iter().map(Some));
    let mat_i = mats.iter().position(|m| *m == Some(Mat::Rock)).unwrap_or(0);
    commands.insert_resource(Ed { fam, doc, tpl: None, var: 0, left: false, tool: Tool::Material, mat_i, wall_i: 1, thing_i: 0, brush: 2., overlay: Overlay::None,
        msg: "Opened. Paint with the left mouse; the right mouse picks or removes.".into(), redraw: true, stroke: false, discard: None, rows: vec![],
        mats, walls, things: thing_kinds(), img, fit: true });
}

impl Ed {
    fn kind(&self) -> Kind { families()[self.fam] }
    fn pal_len(&self) -> usize { match self.tool { Tool::Material => self.mats.len(), Tool::Lining => self.walls.len(), Tool::Thing => self.things.len(), Tool::Chest => 1 } }
    fn pal_i(&mut self) -> &mut usize { match self.tool { Tool::Material | Tool::Chest => &mut self.mat_i, Tool::Lining => &mut self.wall_i, Tool::Thing => &mut self.thing_i } }
    fn pal_label(&self, i: usize) -> String {
        match self.tool {
            Tool::Material | Tool::Chest => self.mats[i].map_or("(host rock)".into(), |m| format!("{m:?}")),
            Tool::Lining => self.walls[i].map_or("(no lining)".into(), |w| format!("{w:?}")),
            Tool::Thing => thing_label(&self.things[i]),
        }
    }
    fn open(&mut self, fam: usize) {
        if self.doc.dirty && self.discard != Some(fam) { self.discard = Some(fam); self.msg = "Unsaved changes: press again to discard them, or Ctrl+S first.".into(); return; }
        match Doc::open(families()[fam]) {
            Ok(d) => { self.doc = d; self.fam = fam; self.var = 0; self.left = false; self.overlay = Overlay::None; self.redraw = true; self.fit = true; self.discard = None;
                self.msg = format!("Opened {}.", kind_label(self.kind())); }
            Err(e) => self.msg = e,
        }
    }
    fn run(&mut self, a: Act) {
        match a {
            Act::Family(k) => self.open(k),
            Act::Tool(t) => { self.tool = t; }
            Act::Pal(i) => { if i < self.pal_len() { *self.pal_i() = i; } }
            Act::Var(d) => { self.var = (self.var as i64 + d).clamp(0, (1 << 20) - 1) as u32; self.overlay = Overlay::None; self.redraw = true; }
            Act::Side => { self.left = !self.left; self.overlay = Overlay::None; self.redraw = true; }
            Act::Brush(d) => self.brush = (self.brush + d).clamp(0.5, 24.),
            Act::Reach | Act::Stress | Act::Simulate => {
                let t = match self.doc.template() { Ok(t) => t, Err(e) => { self.msg = e; return; } };
                match a {
                    Act::Reach => match reach(&t, self.var, self.left) {
                        Ok(r) => { self.msg = if r.problems.is_empty() { format!("Walkable: {} way(s) in, each reaches the {}.", r.ways_in.len(), if t.chest(self.var, self.left).is_some() { "chest" } else { "floor" }) } else { r.problems.join(" / ") }; self.overlay = Overlay::Reach(r); }
                        Err(e) => self.msg = e },
                    Act::Stress => match stress(&t, self.var, self.left) {
                        Ok(o) => { let n = o.iter().filter(|&&b| b).count(); self.msg = if n == 0 { "Sound: nothing over capacity.".into() } else { format!("{n} cells over capacity (red): they crack and fall in play.") }; self.overlay = Overlay::Stress(o); }
                        Err(e) => self.msg = e },
                    _ => match simulate(&t, self.var, self.left, 600) {
                        Ok(s) => { self.msg = format!("After 10 s with a mole at the chest: {} cells changed, {} crumbled, hp {:.0}.", s.changed, s.crumbled, s.hp); self.overlay = Overlay::Sim(s); }
                        Err(e) => self.msg = e },
                }
                self.redraw = true;
            }
            Act::Clear => { self.overlay = Overlay::None; self.redraw = true; }
            Act::Undo => { if self.doc.undo() { self.overlay = Overlay::None; self.redraw = true; self.msg = "Undone.".into(); } }
            Act::Redo => { if self.doc.redo() { self.overlay = Overlay::None; self.redraw = true; self.msg = "Redone.".into(); } }
            Act::Save => { let p = path_of(self.kind()); match self.doc.save(&p) { Ok(()) => self.msg = format!("Saved {} (marked as the editor's: export_sites leaves it alone now).", p.display()), Err(e) => self.msg = format!("Not saved: {e}") } }
            Act::Fit => self.fit = true,
        }
    }
    fn build_rows(&mut self) {
        let mut r: Vec<(String, Option<Act>, bool)> = vec![];
        let mut row = |s: String, a: Option<Act>, on: bool| r.push((s, a, on));
        row(format!("CAVE EDITOR  (dev only, not shipped){}", if self.doc.dirty { "  *unsaved*" } else { "" }), None, false);
        row(String::new(), None, false);
        for (k, &f) in families().iter().enumerate() { row(format!("{}  {}", k + 1, kind_label(f)), Some(Act::Family(k)), k == self.fam); }
        row(String::new(), None, false);
        let t = self.tool;
        row(format!("M  paint material{}", if t == Tool::Material { "   <" } else { "" }), Some(Act::Tool(Tool::Material)), t == Tool::Material);
        row(format!("L  paint lining{}", if t == Tool::Lining { "   <" } else { "" }), Some(Act::Tool(Tool::Lining)), t == Tool::Lining);
        row(format!("T  set things (torch, furniture, rope...){}", if t == Tool::Thing { "   <" } else { "" }), Some(Act::Tool(Tool::Thing)), t == Tool::Thing);
        row(format!("C  move the chest{}", if t == Tool::Chest { "   <" } else { "" }), Some(Act::Tool(Tool::Chest)), t == Tool::Chest);
        row(format!("[ ]  brush radius {:.1} cells ({:.2} m)", self.brush, self.brush / molehill_sim::mat::CPM as f32), None, false);
        row(String::new(), None, false);
        if t != Tool::Chest {
            let n = self.pal_len(); let cur = match t { Tool::Material => self.mat_i, Tool::Lining => self.wall_i, _ => self.thing_i };
            let first = cur.saturating_sub(6).min(n.saturating_sub(13));
            row(format!(",  .  palette ({} of {n})", cur + 1), None, false);
            for i in first..(first + 13).min(n) { row(format!("{} {}", if i == cur { ">" } else { " " }, self.pal_label(i)), Some(Act::Pal(i)), i == cur); }
        } else { row("Left click sets the chest; right click removes it.".into(), None, false); }
        row(String::new(), None, false);
        row(format!("V / B  variant var = {}   (Shift: 16)", self.var), None, false);
        row("   -1".into(), Some(Act::Var(-1)), false); row("   +1".into(), Some(Act::Var(1)), false);
        row(format!("N  side: {}", if self.left { "west (left)" } else { "east" }), Some(Act::Side), self.left);
        if let Some(tp) = &self.tpl { for (i, s) in tp.file.slots.iter().enumerate() {
            let o = s.pick.choose(self.var, self.left).min(s.options.len() - 1); let _ = i;
            row(format!("   {}: {}", s.name, s.options[o].name), None, false); } }
        row(String::new(), None, false);
        row("R  can the mole walk it? (reachability)".into(), Some(Act::Reach), matches!(self.overlay, Overlay::Reach(_)));
        row("P  will it stand? (stress)".into(), Some(Act::Stress), matches!(self.overlay, Overlay::Stress(_)));
        row("G  run it 10 s with a mole in it".into(), Some(Act::Simulate), matches!(self.overlay, Overlay::Sim(_)));
        row("Esc  clear the check".into(), Some(Act::Clear), false);
        row(String::new(), None, false);
        row("Ctrl/Cmd+Z undo, +Shift (or Y) redo".into(), Some(Act::Undo), false);
        row("Ctrl/Cmd+S save to assets/sites/".into(), Some(Act::Save), false);
        row("Wheel or +/- zoom, arrows/WASD pan, F fit".into(), Some(Act::Fit), false);
        row(String::new(), None, false);
        let words: Vec<&str> = self.msg.split(' ').collect(); let mut line = String::new();
        for wd in words { if line.len() + wd.len() > 44 { row(line.clone(), None, false); line.clear(); } if !line.is_empty() { line.push(' '); } line.push_str(wd); }
        if !line.is_empty() { row(line, None, false); }
        self.rows = r;
    }
}

fn cell_at(ed: &Ed, win: &Window, cam: &Transform) -> Option<(usize, usize)> {
    let c = win.cursor_position()?; if c.x < PANEL_W { return None; }
    let sc = cam.scale.x;
    let wp = Vec2::new(cam.translation.x + (c.x - win.width() / 2.) * sc, cam.translation.y - (c.y - win.height() / 2.) * sc);
    let (w, h) = (ed.doc.w as f32 * PX, ed.doc.h as f32 * PX);
    let (x, y) = ((wp.x + w / 2.) / PX, (h / 2. - wp.y) / PX);
    if x < 0. || y < 0. || x >= ed.doc.w as f32 || y >= ed.doc.h as f32 { return None; }
    Some((x as usize, y as usize))
}

fn input(mut ed: ResMut<Ed>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, scroll: Res<AccumulatedMouseScroll>,
         win: Query<&Window, With<PrimaryWindow>>, mut cam: Query<&mut Transform, (With<Camera2d>, Without<Canvas>)>, mut canvas: Query<&mut Sprite, With<Canvas>>, time: Res<Time>) {
    let Ok(win) = win.single() else { return }; let Ok(mut ct) = cam.single_mut() else { return };
    let cmd = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight, KeyCode::SuperLeft, KeyCode::SuperRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let k = |c: KeyCode| keys.just_pressed(c);
    // keys
    if cmd {
        if k(KeyCode::KeyS) { ed.run(Act::Save); }
        if k(KeyCode::KeyZ) { ed.run(if shift { Act::Redo } else { Act::Undo }); }
        if k(KeyCode::KeyY) { ed.run(Act::Redo); }
    } else {
        for (i, c) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7].into_iter().enumerate() { if k(c) { ed.run(Act::Family(i)); } }
        if k(KeyCode::KeyM) { ed.run(Act::Tool(Tool::Material)); }
        if k(KeyCode::KeyL) { ed.run(Act::Tool(Tool::Lining)); }
        if k(KeyCode::KeyT) { ed.run(Act::Tool(Tool::Thing)); }
        if k(KeyCode::KeyC) { ed.run(Act::Tool(Tool::Chest)); }
        if k(KeyCode::BracketLeft) { ed.run(Act::Brush(-0.5)); }
        if k(KeyCode::BracketRight) { ed.run(Act::Brush(0.5)); }
        let n = ed.pal_len().max(1);
        if k(KeyCode::Comma) { let i = (*ed.pal_i() + n - 1) % n; ed.run(Act::Pal(i)); }
        if k(KeyCode::Period) { let i = (*ed.pal_i() + 1) % n; ed.run(Act::Pal(i)); }
        let step = if shift { 16 } else { 1 };
        if k(KeyCode::KeyV) { ed.run(Act::Var(-step)); }
        if k(KeyCode::KeyB) { ed.run(Act::Var(step)); }
        if k(KeyCode::KeyN) { ed.run(Act::Side); }
        if k(KeyCode::KeyR) { ed.run(Act::Reach); }
        if k(KeyCode::KeyP) { ed.run(Act::Stress); }
        if k(KeyCode::KeyG) { ed.run(Act::Simulate); }
        if k(KeyCode::Escape) { ed.run(Act::Clear); }
        if k(KeyCode::KeyF) { ed.run(Act::Fit); }
    }
    // view: fit, zoom, pan
    if ed.fit {
        ed.fit = false;
        let z = ((ed.doc.w as f32 * PX) / (win.width() - PANEL_W - 40.)).max((ed.doc.h as f32 * PX) / (win.height() - 40.)).max(0.05);
        ct.scale = Vec3::splat(z); ct.translation.x = -PANEL_W / 2. * z; ct.translation.y = 0.;
        if let Ok(mut s) = canvas.single_mut() { s.custom_size = Some(Vec2::new(ed.doc.w as f32 * PX, ed.doc.h as f32 * PX)); }
    }
    let mut zoom = 0.;
    if scroll.delta.y != 0. && win.cursor_position().is_some_and(|c| c.x > PANEL_W) { zoom -= scroll.delta.y.signum(); }
    if k(KeyCode::Equal) { zoom -= 1.; } if k(KeyCode::Minus) { zoom += 1.; }
    if zoom != 0. { let z = (ct.scale.x * 1.15f32.powf(zoom)).clamp(0.05, 4.); ct.scale = Vec3::splat(z); }
    let pan = 600. * time.delta_secs() * ct.scale.x;
    if !cmd {
        if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) { ct.translation.x -= pan; }
        if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) { ct.translation.x += pan; }
        if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) { ct.translation.y += pan; }
        if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) { ct.translation.y -= pan; }
    }
    // panel clicks
    if mouse.just_pressed(MouseButton::Left) { if let Some(c) = win.cursor_position() { if c.x < PANEL_W {
        let i = ((c.y - 8.) / LINE).floor(); if i >= 0. { if let Some((_, Some(a), _)) = ed.rows.get(i as usize).cloned() { ed.run(a); } }
        return;
    } } }
    // canvas
    let at = cell_at(&ed, win, &ct);
    if !mouse.pressed(MouseButton::Left) { ed.stroke = false; }
    let Some((x, y)) = at else { return };
    let (var, left) = (ed.var, ed.left);
    match ed.tool {
        Tool::Material | Tool::Lining => {
            if mouse.pressed(MouseButton::Left) {
                if !ed.stroke { ed.doc.begin(); ed.stroke = true; }
                let b = if ed.tool == Tool::Material { Brush::Mat(ed.mats[ed.mat_i]) } else { Brush::Lining(ed.walls[ed.wall_i]) };
                let r = ed.brush;
                if ed.doc.paint(x as i64, y as i64, r, b) > 0 { ed.overlay = Overlay::None; ed.redraw = true; }
            }
            if mouse.just_pressed(MouseButton::Right) && ed.tool == Tool::Material {
                if let Some(Brush::Mat(m)) = ed.doc.pick(x, y) { if let Some(i) = ed.mats.iter().position(|&q| q == m) { ed.mat_i = i; ed.msg = format!("Picked {}.", ed.pal_label(i)); } }
            }
        }
        Tool::Thing => {
            if mouse.just_pressed(MouseButton::Left) { ed.doc.begin(); let t = ed.things[ed.thing_i].clone(); ed.doc.place_thing(&t, x, y, var, left); ed.msg = format!("Set down a {} at ({x}, {y}).", thing_label(&t)); ed.redraw = true; }
            if mouse.just_pressed(MouseButton::Right) { ed.doc.begin(); if ed.doc.remove_thing_near(x, y, 6.) { ed.msg = "Took the nearest thing away.".into(); ed.redraw = true; } }
        }
        Tool::Chest => {
            if mouse.just_pressed(MouseButton::Left) { ed.doc.begin(); ed.doc.set_chest(Some([x, y])); ed.msg = format!("Chest at ({x}, {y}).", ); ed.redraw = true; }
            if mouse.just_pressed(MouseButton::Right) { ed.doc.begin(); ed.doc.set_chest(None); ed.msg = "No chest.".into(); ed.redraw = true; }
        }
    }
}

fn panel(mut ed: ResMut<Ed>, mut rows: Query<(&Row, &mut Text, &mut TextColor)>) {
    ed.build_rows();
    for (r, mut t, mut c) in rows.iter_mut() {
        let (s, a, on) = ed.rows.get(r.0).cloned().unwrap_or_default();
        if t.0 != s { t.0 = s; }
        c.0 = if on { ACCENT } else if a.is_some() { INK } else { INK.with_alpha(0.62) };
    }
}

fn redraw(mut ed: ResMut<Ed>, mut images: ResMut<Assets<Image>>) {
    if !ed.redraw { return; }
    ed.redraw = false;
    let t = match ed.doc.template() { Ok(t) => t, Err(e) => { ed.msg = e; return; } };
    let sim = if let Overlay::Sim(s) = &ed.overlay { Some(s.cells.as_slice()) } else { None };
    let mut px = draw(&t, ed.var, ed.left, sim);
    match &ed.overlay {
        Overlay::Reach(r) => {
            overlay(&mut px, r.region.iter().map(|&k| k != 0 && r.ways_in.contains(&k)), [80, 220, 120], 0.35);
            overlay(&mut px, r.region.iter().map(|&k| k != 0 && !r.ways_in.contains(&k)), [240, 140, 40], 0.5);
        }
        Overlay::Stress(o) => overlay(&mut px, o.iter().copied(), [255, 40, 40], 0.75),
        _ => {}
    }
    let (w, h) = (t.w, t.h);
    let handle = ed.img.clone();
    if let Some(mut im) = images.get_mut(&handle) {
        if im.width() as usize != w || im.height() as usize != h { *im = blank(w, h); }
        if let Some(d) = im.data.as_mut() { d.copy_from_slice(&px); }
    }
    ed.tpl = Some(t);
}

impl Default for Act { fn default() -> Self { Act::Clear } }
