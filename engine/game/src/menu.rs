//! Main menu (title screen) and the Esc pause menu. Plain text rows over a dim veil, same font and
//! ink as the HUD. Keyboard (W/S or arrows, Enter/Space, A/D or arrows to adjust, Esc back) and mouse.
//! The sim is paused while any menu is open. The headless scenario harness (molehill-sim) never sees this.
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use crate::{Game, audio::Audio};
use molehill_sim::survival::Mode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Screen { Off, Main, Pause, Settings, Controls, NewHolt }
#[derive(Clone, Copy, PartialEq, Debug)] enum Item { Continue, NewHolt, Survival, Creative, NewHunger, Hunger, Seed, Settings, Controls, Quit, Resume, Restart, Save, Load, MainMenu, Volume, Music, Zoom, Headlamp, Fullscreen, Back }

#[derive(Resource)] pub struct Menu { pub screen: Screen, back: Screen, sel: usize, pub seed_text: String, pub volume: f32, pub music: f32, music_on: f32, pub zoom: f32, pub headlamp: bool, pub fullscreen: bool, has_save: bool,
    /// New Holt option: hunger on (Survival); `world_hunger` mirrors the current world's setting for Settings
    pub hunger: bool, world_hunger: bool, world_survival: bool }
impl Default for Menu { fn default() -> Self { Menu { screen: Screen::Main, back: Screen::Main, sel: 0, seed_text: String::new(), volume: 0.8, music: MUSIC_DEFAULT, music_on: MUSIC_DEFAULT, zoom: molehill_sim::view::ZOOM_DEFAULT, headlamp: true, fullscreen: false, has_save: false, hunger: true, world_hunger: true, world_survival: false } } }
impl Menu {
    pub fn open(&self) -> bool { self.screen != Screen::Off }
    fn items(&self) -> Vec<Item> {
        use Item::*;
        match self.screen {
            Screen::Main => { let mut v = vec![]; if self.has_save { v.push(Continue); } v.extend([NewHolt, Settings, Controls]); if cfg!(not(target_arch = "wasm32")) { v.push(Quit); } v }
            Screen::Pause => vec![Resume, Restart, NewHolt, Save, Load, Settings, Controls, MainMenu],
            Screen::Settings => { let mut v = vec![Volume, Music, Zoom, Headlamp]; if self.world_survival { v.push(Hunger); } if cfg!(not(target_arch = "wasm32")) { v.push(Fullscreen); } v.push(Back); v }
            // New Holt: pick the mode (Survival is the game; Creative has everything and nothing hurts)
            Screen::NewHolt => vec![Survival, Creative, NewHunger, Seed, Back],
            Screen::Controls => vec![Back], Screen::Off => vec![],
        }
    }
    fn label(&self, it: Item) -> String {
        match it {
            Item::Continue => "Continue".into(), Item::NewHolt => "New Holt".into(), Item::Survival => "Survival".into(), Item::Creative => "Creative".into(),
            Item::NewHunger => format!("Hunger   {}  (Survival)", if self.hunger { "on" } else { "off" }), Item::Hunger => format!("Hunger       {}  (this holt)", if self.world_hunger { "on" } else { "off" }),
            Item::Seed => format!("Seed  {}", if self.seed_text.is_empty() { "random  (type digits)".into() } else { self.seed_text.clone() + "_" }),
            Item::Settings => "Settings".into(), Item::Controls => "Controls".into(), Item::Quit => "Quit".into(),
            Item::Resume => "Resume".into(), Item::Restart => "Restart (same seed)".into(), Item::Save => "Save".into(), Item::Load => "Load".into(), Item::MainMenu => "Main Menu".into(),
            Item::Volume => format!("Volume   <  {:>3}%  >", (self.volume * 100.).round()),
            Item::Music => if self.music <= 0. { "Music    <   off  >".into() } else { format!("Music    <  {:>3}%  >", (self.music * 100.).round()) }, Item::Zoom => format!("Zoom     <  {:.1}x  >", molehill_sim::view::zoom_level(self.zoom)),
            Item::Headlamp => format!("Headlamp     {}  (G)", if self.headlamp { "lit" } else { "out" }), Item::Fullscreen => format!("Fullscreen   {}", if self.fullscreen { "on" } else { "off" }), Item::Back => "Back".into(),
        }
    }
    /// Zoom by `steps` notches (positive = closer), clamped, applied to the camera and saved.
    pub fn zoom_by(&mut self, steps: f32, cam: &mut Transform) { let z = molehill_sim::view::zoom_step(self.zoom, steps); if z != self.zoom { self.zoom = z; self.persist(); } cam.scale = Vec3::splat(self.zoom); }
    pub fn persist(&self) { let _ = crate::storage_put("molehill-settings", format!("{} {} {} {} {}", self.volume, self.zoom, self.fullscreen as u8, self.music, self.headlamp as u8).as_bytes()); }
    pub fn load() -> Menu {
        let mut m = Menu::default();
        if let Some(b) = crate::storage_get("molehill-settings") { let s = String::from_utf8_lossy(&b).to_string(); let v: Vec<&str> = s.split_whitespace().collect();
            if v.len() >= 3 { m.volume = v[0].parse().unwrap_or(0.8); m.zoom = molehill_sim::view::clamp_zoom(v[1].parse().unwrap_or(f32::NAN)); m.fullscreen = v[2] == "1"; }
            if let Some(mv) = v.get(3).and_then(|x| x.parse::<f32>().ok()).filter(|x| x.is_finite()) { m.music = mv.clamp(0., 1.); if m.music > 0. { m.music_on = m.music; } }
            if let Some(h) = v.get(4) { m.headlamp = *h != "0"; } }
        m.has_save = crate::storage_get("molehill-manual").is_some() || crate::storage_get("molehill-auto").is_some(); m
    }
    pub fn go(&mut self, s: Screen) { if matches!(s, Screen::Settings | Screen::Controls | Screen::NewHolt) { self.back = self.screen; } self.screen = s; self.sel = 0; if s == Screen::Main { self.has_save = crate::storage_get("molehill-manual").is_some() || crate::storage_get("molehill-auto").is_some(); } }
}

#[derive(Component)] pub struct MenuRoot;
#[derive(Component)] pub struct MenuTitle;
#[derive(Component)] pub struct MenuHint;
#[derive(Component)] pub struct MenuRow(usize);
const ROWS: usize = 9;
/// Music starts quiet: it should sit under everything else.
pub const MUSIC_DEFAULT: f32 = 0.35;
const INK: Color = Color::srgb(0.93, 0.90, 0.84);
const ACCENT: Color = Color::srgb(0.95, 0.72, 0.38);

pub fn setup(commands: &mut Commands, f: &Handle<Font>) {
    let tf = |s: f32| TextFont { font: f.clone().into(), font_size: s.into(), ..default() };
    commands.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.), height: Val::Percent(100.), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(6.), ..default() },
        BackgroundColor(Color::srgba(0.03, 0.025, 0.035, 0.78)), GlobalZIndex(50), MenuRoot)).with_children(|p| {
        p.spawn((Text::new("Molehill"), tf(56.), TextColor(INK), TextShadow::default(), Node { margin: UiRect::bottom(Val::Px(26.)), ..default() }, MenuTitle));
        for i in 0..ROWS { p.spawn((Text::new(""), tf(22.), TextColor(INK), TextShadow::default(), Interaction::None, Node { padding: UiRect::axes(Val::Px(10.), Val::Px(2.)), ..default() }, MenuRow(i))); }
        let c = spawn_controls(p, f, 16.); p.commands().entity(c).insert((MenuControls, Visibility::Hidden));
        p.spawn((Text::new(""), tf(14.), TextColor(INK.with_alpha(0.55)), TextLayout { justify: Justify::Center, ..default() }, Node { margin: UiRect::top(Val::Px(24.)), ..default() }, MenuHint));
    });
}

/// Every control, as (action, keys). Shown as a two-column table on the Controls screen and the H overlay.
pub const CONTROL_ROWS: &[(&str, &str)] = &[
    ("Move", "A / D  (Shift: sprint)"), ("Jump", "Space  (or W off a wall)"), ("Walls", "hold into a wall: W / S climb, Space wall-jump"),
    ("Dig down", "S"), ("Use item", "Left mouse"), ("Interact", "Right mouse  (levers, light TNT; on an empty spot: set down a device or TNT)"), ("Throw", "hold Q, release  (TNT goes out lit)"), ("Pick up", "E  (devices, unlit TNT, torches; hold and sweep for copperline)"),
    ("Brush size", "[ / ]  or  Option/Alt + wheel"), ("Grid snap", "X  (blocks in squares; a drag lays a line)"), ("Hotbar", "1 - 0  or  wheel"), ("Pack", "I"), ("Headlamp", "G"), ("Wiring view", "V"), ("Zoom", "+ / -  or  Ctrl + wheel"),
    ("Pause", "Esc"), ("Save / load", "K / L"), ("Controls", "H"), ("Dev info", "F3"),
];
/// A two-column table of `CONTROL_ROWS`: actions dim and right-aligned, keys in ink.
pub fn spawn_controls(p: &mut ChildSpawnerCommands, f: &Handle<Font>, size: f32) -> Entity {
    let tf = |s: f32| TextFont { font: f.clone().into(), font_size: s.into(), ..default() };
    p.spawn(Node { display: Display::Grid, grid_template_columns: vec![GridTrack::auto(), GridTrack::auto()], column_gap: Val::Px(18.), row_gap: Val::Px(4.), ..default() }).with_children(|g| {
        for (a, k) in CONTROL_ROWS { g.spawn((Text::new(*a), tf(size), TextColor(INK.with_alpha(0.5)), Node { justify_self: JustifySelf::End, ..default() })); g.spawn((Text::new(*k), tf(size), TextColor(INK.with_alpha(0.9)))); }
    }).id()
}
#[derive(Component)] pub struct MenuControls;

#[allow(clippy::too_many_arguments)]
pub fn update(mut m: ResMut<Menu>, mut g: ResMut<Game>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, mut commands: Commands,
    mut au: NonSendMut<Audio>, mut cam: Query<&mut Transform, With<Camera2d>>, mut win: Query<&mut Window, With<PrimaryWindow>>,
    mut ctl: Query<&mut Visibility, (With<MenuControls>, Without<MenuRoot>, Without<MenuRow>)>, mut q: ParamSet<(Query<&mut Visibility, With<MenuRoot>>, Query<(&MenuRow, &mut Text, &mut TextColor, &Interaction, &mut Visibility)>, Query<&mut Text, With<MenuHint>>, Query<&mut Text, With<MenuTitle>>)>,
    mut exit: MessageWriter<AppExit>) {
    if !m.open() {
        if keys.just_pressed(KeyCode::Escape) && !g.inv_open && !g.help { m.go(Screen::Pause); g.paused = true; }
    } else {
        g.paused = true;
        if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() { au.unlock(); }
        m.world_hunger = !g.w.rules.no_hunger; m.world_survival = g.w.mode == Mode::Survival;
        let items = m.items(); let n = items.len(); let (sel0, scr0) = (m.sel, m.screen);
        if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) { m.sel = (m.sel + 1) % n; }
        if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) { m.sel = (m.sel + n - 1) % n; }
        let mut act = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::NumpadEnter);
        // mouse: hover selects, click activates
        let mut clicked = false;
        for (r, _, _, i, _) in q.p1().iter() { if r.0 < n && *i != Interaction::None { m.sel = r.0; if mouse.just_pressed(MouseButton::Left) { clicked = true; } } }
        act |= clicked;
        let adj = if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) { 1. } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) { -1. } else { 0. };
        let it = items[m.sel.min(n - 1)];
        // seed field: digits type, backspace deletes
        if it == Item::Seed {
            let digs = [KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9];
            for (d, k) in digs.iter().enumerate() { if keys.just_pressed(*k) && m.seed_text.len() < 9 { m.seed_text.push(char::from(b'0' + d as u8)); } }
            if keys.just_pressed(KeyCode::Backspace) { m.seed_text.pop(); }
        }
        // zoom row: A/D or arrows step out/in; a click on the left half zooms out, on the right half in
        let click_dir = win.single().ok().and_then(|w| w.cursor_position().map(|c| if c.x < w.width() / 2. { -1. } else { 1. })).unwrap_or(1.);
        let mut z = |m: &mut Menu, d: f32| { if let Ok(mut ct) = cam.single_mut() { m.zoom_by(d, &mut ct); } };
        match it {
            Item::Volume if adj != 0. || act => { m.volume = if adj != 0. { (m.volume + adj * 0.1).clamp(0., 1.) } else if m.volume > 0. { 0. } else { 0.8 }; au.set_volume(m.volume); m.persist(); }
            // music: A/D in 10% steps; Enter or a click mutes, and again brings back the last level
            Item::Music if adj != 0. || act => { if adj != 0. { m.music = (m.music + adj * 0.1).clamp(0., 1.); if m.music > 0. { m.music_on = m.music; } } else { m.music = if m.music > 0. { 0. } else { m.music_on.max(0.1) }; } m.persist(); }
            Item::Zoom if adj != 0. || act => z(&mut m, if adj != 0. { adj } else if clicked { click_dir } else { 1. }),
            _ if act => match it {
                Item::Continue => { if crate::load_saved(&mut g, &mut commands) { m.go(Screen::Off); } }
                Item::NewHolt => m.go(Screen::NewHolt),
                Item::Survival | Item::Creative | Item::Seed => { let mode = if it == Item::Creative { Mode::Creative } else { Mode::Survival }; let seed = m.seed_text.parse().unwrap_or_else(|_| crate::seed_now() % 100000); crate::begin(&mut g, &mut commands, seed, mode); if mode == Mode::Survival && !m.hunger { g.w.set_hunger(false); } m.seed_text.clear(); m.go(Screen::Off); if let Ok(mut ct) = cam.single_mut() { ct.scale = Vec3::splat(m.zoom); } }
                Item::Restart => { let (s, mode, nh) = (g.seed, g.w.mode, g.w.rules.no_hunger); crate::begin(&mut g, &mut commands, s, mode); if nh { g.w.set_hunger(false); } m.go(Screen::Off); }
                Item::Resume => m.go(Screen::Off),
                Item::Save => { crate::save_now(&mut g); m.go(Screen::Off); }
                Item::Load => { if crate::load_saved(&mut g, &mut commands) { m.go(Screen::Off); } }
                Item::Settings => m.go(Screen::Settings), Item::Controls => m.go(Screen::Controls),
                Item::MainMenu => { crate::save_auto(&mut g); m.go(Screen::Main); }
                Item::Quit => { crate::save_auto(&mut g); exit.write(AppExit::Success); }
                Item::Headlamp => { m.headlamp = !m.headlamp; m.persist(); }
                Item::NewHunger => m.hunger = !m.hunger,
                Item::Hunger => { let on = g.w.rules.no_hunger; g.w.set_hunger(on); m.world_hunger = on; }
                Item::Fullscreen => { m.fullscreen = !m.fullscreen; if let Ok(mut w) = win.single_mut() { w.mode = if m.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed }; } m.persist(); }
                Item::Back => { let b = m.back; m.go(b); }
                _ => {}
            },
            _ => {}
        }
        // a soft tick when the choice moves or a screen changes: immediate feedback for every press
        if m.sel != sel0 || m.screen != scr0 || act && n > 0 { au.play(molehill_sim::world::Event::Click, 0.35); }
        if keys.just_pressed(KeyCode::Escape) { match m.screen { Screen::Pause => m.go(Screen::Off), Screen::Settings | Screen::Controls | Screen::NewHolt => { let b = m.back; m.go(b); } _ => {} } }
        if !m.open() { g.paused = false; g.w.swallow_held(); g.mouse_latch = true; } // the key or click that closed the menu doesn't act in the game
    }
    // draw
    let open = m.open();
    for mut v in q.p0().iter_mut() { *v = if open { Visibility::Inherited } else { Visibility::Hidden }; }
    for mut v in &mut ctl { *v = if m.screen == Screen::Controls { Visibility::Inherited } else { Visibility::Hidden }; }
    if !open { return; }
    let items = m.items(); let sel = m.sel;
    let labels: Vec<String> = items.iter().map(|i| m.label(*i)).collect();
    for (r, mut t, mut c, _, mut v) in q.p1().iter_mut() {
        if let Some(l) = labels.get(r.0) { *v = Visibility::Inherited; t.0 = if r.0 == sel { format!("›  {l}  ‹") } else { l.clone() }; c.0 = if r.0 == sel { ACCENT } else { INK.with_alpha(0.8) }; }
        else { *v = Visibility::Hidden; }
    }
    let title = match m.screen { Screen::Pause => "Paused".to_string(), Screen::Settings => "Settings".into(), Screen::Controls => "Controls".into(), Screen::NewHolt => "New Holt".into(), _ => "Molehill".into() };
    for mut t in q.p3().iter_mut() { t.0 = title.clone(); }
    let hint = match m.screen { Screen::Controls => "Esc back".to_string(), Screen::NewHolt => "Survival: find food, oil and tools.   Creative: everything, nothing hurts.\nEsc back".to_string(), Screen::Settings => "A / D adjust     Esc back".into(), Screen::Main => format!("W / S choose     Enter select\nholt {}", g.seed), _ => "W / S choose     Enter select     Esc resume".into() };
    for mut t in q.p2().iter_mut() { t.0 = hint.clone(); }
}
