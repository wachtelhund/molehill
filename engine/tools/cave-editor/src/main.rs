//! Molehill cave editor: a dev-only window for painting found-place templates (`assets/sites/*.json`).
//! `cargo run --release -p molehill-cave-editor [family]` from `engine/`. Never shipped: see
//! `docs/wiki/Cave-Editor.md`. Everything the panel and the mouse do is `Editor` in `lib.rs` (tested
//! headless); this file only reads input, lays out the panel and draws the canvas.
//!
//! Dev checks: `--canvas=FILE.bmp [--shot-keys=…]` writes the canvas (with the check the keys ran) and quits
//! without opening a window; `--shot=FILE.bmp [--shot-at=N] [--shot-keys=…]` saves one frame of the window
//! (on macOS only when the window is really on screen; from a background shell it reads back black).
use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler};
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use molehill_cave_editor::*;
use molehill_sim::sites::Kind;
use molehill_sim::templates::Template;

/// World units per cell on screen at zoom 1.
const PX: f32 = 3.;
const INK: Color = Color::srgb(0.86, 0.82, 0.74);
const ACCENT: Color = Color::srgb(0.98, 0.78, 0.36);

#[derive(Resource)]
struct Ed { e: Editor, img: Handle<Image>, tpl: Option<Template> }
#[derive(Component)] struct Canvas;
#[derive(Component)] struct Row(usize);

fn arg(k: &str) -> Option<String> { std::env::args().find_map(|a| a.strip_prefix(k).map(|v| v.to_string())) }
fn keys_from_args(e: &mut Editor) { for c in arg("--shot-keys=").unwrap_or_default().chars() { e.key(&c.to_string()); } }

fn main() {
    let fam = std::env::args().skip(1).find(|a| !a.starts_with("--"))
        .and_then(|a| families().iter().position(|&k| kind_label(k) == a || Kind::from_name(&a) == Some(k))).unwrap_or(0);
    let e = Editor::new(fam).unwrap_or_else(|err| panic!("{err}"));
    if let Some(path) = arg("--canvas=") {
        let mut e = e; keys_from_args(&mut e);
        let t = e.doc.template().unwrap();
        write_bmp(std::path::Path::new(&path), t.w, t.h, &e.pixels(&t), false).unwrap();
        println!("{}", e.msg);
        return;
    }
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { title: "Molehill cave editor (dev only)".into(), resolution: (1600u32, 960u32).into(),
            resize_constraints: bevy::window::WindowResizeConstraints { min_width: PANEL_W + 400., min_height: MIN_WINDOW_H, ..default() }, ..default() }), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.045, 0.05)))
        .insert_non_send(Start(Some(e)))
        .add_systems(Startup, setup)
        .add_systems(Update, (input, panel, redraw, shot).chain())
        .run();
}
struct Start(Option<Editor>);

fn blank(w: usize, h: usize) -> Image {
    let mut im = Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, vec![0; w * h * 4], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default());
    im.sampler = ImageSampler::nearest();
    im
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut start: NonSendMut<Start>) {
    let e = start.0.take().unwrap();
    let img = images.add(blank(e.doc.w, e.doc.h));
    commands.spawn((Camera2d, Transform::from_xyz(-PANEL_W / 2., 0., 100.)));
    commands.spawn((Sprite { image: img.clone(), custom_size: Some(Vec2::new(e.doc.w as f32 * PX, e.doc.h as f32 * PX)), ..default() }, Transform::default(), Canvas));
    commands.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.), top: Val::Px(0.), width: Val::Px(PANEL_W), height: Val::Percent(100.), ..default() },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.025, 0.92)))).with_children(|p| {
        for i in 0..ROWS {
            p.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(10.), top: Val::Px(ROW_TOP + i as f32 * LINE), ..default() },
                Text::new(""), TextFont { font_size: 13.0_f32.into(), ..default() }, TextColor(INK), Row(i)));
        }
    });
    commands.insert_resource(Ed { e, img, tpl: None });
}

fn cell_at(e: &Editor, win: &Window, cam: &Transform) -> Option<(usize, usize)> {
    let c = win.cursor_position()?; if c.x < PANEL_W { return None; }
    let sc = cam.scale.x;
    let wp = Vec2::new(cam.translation.x + (c.x - win.width() / 2.) * sc, cam.translation.y - (c.y - win.height() / 2.) * sc);
    let (w, h) = (e.doc.w as f32 * PX, e.doc.h as f32 * PX);
    let (x, y) = ((wp.x + w / 2.) / PX, (h / 2. - wp.y) / PX);
    if x < 0. || y < 0. || x >= e.doc.w as f32 || y >= e.doc.h as f32 { return None; }
    Some((x as usize, y as usize))
}

fn input(mut ed: ResMut<Ed>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, scroll: Res<AccumulatedMouseScroll>,
         win: Query<&Window, With<PrimaryWindow>>, mut cam: Query<&mut Transform, (With<Camera2d>, Without<Canvas>)>, mut canvas: Query<&mut Sprite, With<Canvas>>, time: Res<Time>) {
    let Ok(win) = win.single() else { return }; let Ok(mut ct) = cam.single_mut() else { return };
    let Ed { e, tpl, .. } = &mut *ed;
    let cmd = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight, KeyCode::SuperLeft, KeyCode::SuperRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    for (code, name) in [(KeyCode::Digit1, "1"), (KeyCode::Digit2, "2"), (KeyCode::Digit3, "3"), (KeyCode::Digit4, "4"), (KeyCode::Digit5, "5"), (KeyCode::Digit6, "6"), (KeyCode::Digit7, "7"),
        (KeyCode::KeyM, "m"), (KeyCode::KeyL, "l"), (KeyCode::KeyT, "t"), (KeyCode::KeyC, "c"), (KeyCode::BracketLeft, "["), (KeyCode::BracketRight, "]"),
        (KeyCode::Comma, ","), (KeyCode::Period, "."), (KeyCode::KeyV, "v"), (KeyCode::KeyB, "b"), (KeyCode::KeyN, "n"), (KeyCode::KeyR, "r"), (KeyCode::KeyP, "p"),
        (KeyCode::KeyG, "g"), (KeyCode::Escape, "esc"), (KeyCode::KeyF, "f"), (KeyCode::KeyS, "s"), (KeyCode::KeyZ, "z"), (KeyCode::KeyY, "y")] {
        if !keys.just_pressed(code) { continue; }
        let mut k = if shift && name.len() == 1 && name.as_bytes()[0].is_ascii_lowercase() { name.to_uppercase() } else { name.to_string() };
        if cmd { k = format!("^{k}"); } else if matches!(name, "s" | "z" | "y") { continue; } // plain S pans; Z, Y do nothing alone
        e.key(&k);
    }
    // view: fit, zoom, pan
    if e.fit {
        e.fit = false;
        let z = ((e.doc.w as f32 * PX) / (win.width() - PANEL_W - 40.)).max((e.doc.h as f32 * PX) / (win.height() - 40.)).max(0.05);
        ct.scale = Vec3::splat(z); ct.translation.x = -PANEL_W / 2. * z; ct.translation.y = 0.;
        if let Ok(mut s) = canvas.single_mut() { s.custom_size = Some(Vec2::new(e.doc.w as f32 * PX, e.doc.h as f32 * PX)); }
    }
    let mut zoom = 0.;
    if scroll.delta.y != 0. && win.cursor_position().is_some_and(|c| c.x > PANEL_W) { zoom -= scroll.delta.y.signum(); }
    if keys.just_pressed(KeyCode::Equal) { zoom -= 1.; }
    if keys.just_pressed(KeyCode::Minus) { zoom += 1.; }
    if zoom != 0. { ct.scale = Vec3::splat((ct.scale.x * 1.15f32.powf(zoom)).clamp(0.05, 4.)); }
    if !cmd {
        let pan = 600. * time.delta_secs() * ct.scale.x;
        if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) { ct.translation.x -= pan; }
        if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) { ct.translation.x += pan; }
        if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) { ct.translation.y += pan; }
        if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) { ct.translation.y -= pan; }
    }
    // mouse: the panel, then the canvas
    if !mouse.pressed(MouseButton::Left) { e.release(); }
    let Some(c) = win.cursor_position() else { return };
    if c.x < PANEL_W {
        if mouse.just_pressed(MouseButton::Left) { let slots = tpl.as_ref().map(|t| e.slot_choices(t)).unwrap_or_default(); e.click_panel(c.y, &slots); }
        return;
    }
    let Some((x, y)) = cell_at(e, win, &ct) else { return };
    if mouse.just_pressed(MouseButton::Left) { e.press(x, y, Press::Left); }
    else if mouse.pressed(MouseButton::Left) { e.press(x, y, Press::LeftHeld); }
    if mouse.just_pressed(MouseButton::Right) { e.press(x, y, Press::Right); }
}

fn panel(ed: Res<Ed>, mut rows: Query<(&Row, &mut Text, &mut TextColor)>) {
    let slots = ed.tpl.as_ref().map(|t| ed.e.slot_choices(t)).unwrap_or_default();
    let all = ed.e.rows(&slots);
    for (r, mut t, mut c) in rows.iter_mut() {
        let row = all.get(r.0).cloned().unwrap_or_default();
        if t.0 != row.text { t.0 = row.text; }
        c.0 = if row.on { ACCENT } else if row.act.is_some() { INK } else { INK.with_alpha(0.62) };
    }
}

fn redraw(mut ed: ResMut<Ed>, mut images: ResMut<Assets<Image>>) {
    if !ed.e.redraw { return; }
    ed.e.redraw = false;
    let t = match ed.e.doc.template() { Ok(t) => t, Err(err) => { ed.e.msg = err; return; } };
    let px = ed.e.pixels(&t);
    let handle = ed.img.clone();
    if let Some(mut im) = images.get_mut(&handle) {
        if im.width() as usize != t.w || im.height() as usize != t.h { *im = blank(t.w, t.h); }
        if let Some(d) = im.data.as_mut() { d.copy_from_slice(&px); }
    }
    ed.tpl = Some(t);
}

/// `--shot=FILE.bmp`: one frame of the window after `--shot-at=N` frames (default 60), then quit.
fn shot(mut n: Local<u32>, mut ed: ResMut<Ed>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    let Some(path) = arg("--shot=") else { return };
    let at: u32 = arg("--shot-at=").and_then(|v| v.parse().ok()).unwrap_or(60);
    *n += 1;
    if *n == 5 { keys_from_args(&mut ed.e); }
    if *n == at {
        commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(move |c: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
            let im = &c.image; let Some(d) = im.data.as_ref() else { return };
            let bgra = matches!(im.texture_descriptor.format, TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm);
            let _ = write_bmp(std::path::Path::new(&path), im.width() as usize, im.height() as usize, d, bgra);
        });
    }
    if *n == at + 20 { exit.write(AppExit::Success); }
}
