//! Main menu (title screen) and the Esc pause menu. Plain text rows over a dim veil, same font and
//! ink as the HUD. Keyboard (W/S or arrows, Enter/Space, A/D or arrows to adjust, Esc back) and mouse.
//! The sim is paused while any menu is open. The headless scenario harness (molehill-sim) never sees this.
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use crate::{Game, audio::Audio};

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Screen { Off, Main, Pause, Settings, Controls }
#[derive(Clone, Copy, PartialEq, Debug)] enum Item { Continue, NewHolt, Seed, Settings, Controls, Quit, Resume, Restart, Save, Load, MainMenu, Volume, Zoom, Fullscreen, Back }

#[derive(Resource)] pub struct Menu { pub screen: Screen, back: Screen, sel: usize, pub seed_text: String, pub volume: f32, pub zoom: f32, pub fullscreen: bool, has_save: bool }
impl Default for Menu { fn default() -> Self { Menu { screen: Screen::Main, back: Screen::Main, sel: 0, seed_text: String::new(), volume: 0.8, zoom: 0.32, fullscreen: false, has_save: false } } }
impl Menu {
    pub fn open(&self) -> bool { self.screen != Screen::Off }
    fn items(&self) -> Vec<Item> {
        use Item::*;
        match self.screen {
            Screen::Main => { let mut v = vec![]; if self.has_save { v.push(Continue); } v.extend([NewHolt, Seed, Settings, Controls]); if cfg!(not(target_arch = "wasm32")) { v.push(Quit); } v }
            Screen::Pause => vec![Resume, Restart, NewHolt, Save, Load, Settings, Controls, MainMenu],
            Screen::Settings => { let mut v = vec![Volume, Zoom]; if cfg!(not(target_arch = "wasm32")) { v.push(Fullscreen); } v.push(Back); v }
            Screen::Controls => vec![Back], Screen::Off => vec![],
        }
    }
    fn label(&self, it: Item) -> String {
        match it {
            Item::Continue => "Continue".into(), Item::NewHolt => "New Holt".into(),
            Item::Seed => format!("Seed  {}", if self.seed_text.is_empty() { "random  (type digits)".into() } else { self.seed_text.clone() + "_" }),
            Item::Settings => "Settings".into(), Item::Controls => "Controls".into(), Item::Quit => "Quit".into(),
            Item::Resume => "Resume".into(), Item::Restart => "Restart (same seed)".into(), Item::Save => "Save".into(), Item::Load => "Load".into(), Item::MainMenu => "Main Menu".into(),
            Item::Volume => format!("Volume   <  {:>3}%  >", (self.volume * 100.).round()), Item::Zoom => format!("Zoom     <  {:.2}  >", self.zoom),
            Item::Fullscreen => format!("Fullscreen   {}", if self.fullscreen { "on" } else { "off" }), Item::Back => "Back".into(),
        }
    }
    fn persist(&self) { let _ = crate::storage_put("molehill-settings", format!("{} {} {}", self.volume, self.zoom, self.fullscreen as u8).as_bytes()); }
    pub fn load() -> Menu {
        let mut m = Menu::default();
        if let Some(b) = crate::storage_get("molehill-settings") { let s = String::from_utf8_lossy(&b).to_string(); let v: Vec<&str> = s.split_whitespace().collect();
            if v.len() == 3 { m.volume = v[0].parse().unwrap_or(0.8); m.zoom = v[1].parse().unwrap_or(0.32); m.fullscreen = v[2] == "1"; } }
        m.has_save = crate::storage_get("molehill-manual").is_some() || crate::storage_get("molehill-auto").is_some(); m
    }
    pub fn go(&mut self, s: Screen) { if matches!(s, Screen::Settings | Screen::Controls) { self.back = self.screen; } self.screen = s; self.sel = 0; if s == Screen::Main { self.has_save = crate::storage_get("molehill-manual").is_some() || crate::storage_get("molehill-auto").is_some(); } }
}

#[derive(Component)] pub struct MenuRoot;
#[derive(Component)] pub struct MenuTitle;
#[derive(Component)] pub struct MenuHint;
#[derive(Component)] pub struct MenuRow(usize);
const ROWS: usize = 9;
const INK: Color = Color::srgb(0.93, 0.90, 0.84);
const ACCENT: Color = Color::srgb(0.95, 0.72, 0.38);

pub fn setup(commands: &mut Commands, f: &Handle<Font>) {
    let tf = |s: f32| TextFont { font: f.clone().into(), font_size: s.into(), ..default() };
    commands.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.), height: Val::Percent(100.), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(6.), ..default() },
        BackgroundColor(Color::srgba(0.03, 0.025, 0.035, 0.78)), GlobalZIndex(50), MenuRoot)).with_children(|p| {
        p.spawn((Text::new("Molehill"), tf(56.), TextColor(INK), TextShadow::default(), Node { margin: UiRect::bottom(Val::Px(26.)), ..default() }, MenuTitle));
        for i in 0..ROWS { p.spawn((Text::new(""), tf(22.), TextColor(INK), TextShadow::default(), Interaction::None, Node { padding: UiRect::axes(Val::Px(10.), Val::Px(2.)), ..default() }, MenuRow(i))); }
        p.spawn((Text::new(""), tf(14.), TextColor(INK.with_alpha(0.55)), TextLayout { justify: Justify::Center, ..default() }, Node { margin: UiRect::top(Val::Px(24.)), ..default() }, MenuHint));
    });
}

const CONTROLS: &str = "Move  A / D        Jump  Space (or W)\nWalls  hold into a wall: W / S climb, Space wall-jump\nDig down  S        Use item  Left mouse        Interact  Right mouse\nHotbar  1-0 / wheel        Pack  I        Zoom  + / -  or  Ctrl+wheel\nPause  Esc        Quick save K / load L        Dev info  F3";

#[allow(clippy::too_many_arguments)]
pub fn update(mut m: ResMut<Menu>, mut g: ResMut<Game>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, mut commands: Commands,
    mut au: NonSendMut<Audio>, mut cam: Query<&mut Transform, With<Camera2d>>, mut win: Query<&mut Window, With<PrimaryWindow>>,
    mut q: ParamSet<(Query<&mut Visibility, With<MenuRoot>>, Query<(&MenuRow, &mut Text, &mut TextColor, &Interaction, &mut Visibility)>, Query<&mut Text, With<MenuHint>>, Query<&mut Text, With<MenuTitle>>)>,
    mut exit: MessageWriter<AppExit>) {
    if !m.open() {
        if keys.just_pressed(KeyCode::Escape) && !g.inv_open && !g.help { m.go(Screen::Pause); g.paused = true; }
    } else {
        g.paused = true;
        if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() { au.unlock(); }
        let items = m.items(); let n = items.len();
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
        let mut z = |m: &mut Menu, d: f32| { m.zoom = (m.zoom * if d > 0. { 1.15 } else { 1. / 1.15 }).clamp(0.12, 1.5); if let Ok(mut ct) = cam.single_mut() { ct.scale = Vec3::splat(m.zoom); } m.persist(); };
        match it {
            Item::Volume if adj != 0. || act => { m.volume = if adj != 0. { (m.volume + adj * 0.1).clamp(0., 1.) } else if m.volume > 0. { 0. } else { 0.8 }; au.set_volume(m.volume); m.persist(); }
            Item::Zoom if adj != 0. || act => z(&mut m, if adj != 0. { adj } else { 1. }),
            _ if act => match it {
                Item::Continue => { if crate::load_saved(&mut g, &mut commands) { m.go(Screen::Off); } }
                Item::NewHolt | Item::Seed => { let seed = m.seed_text.parse().unwrap_or_else(|_| crate::seed_now() % 100000); crate::begin(&mut g, &mut commands, seed); m.seed_text.clear(); m.go(Screen::Off); if let Ok(mut ct) = cam.single_mut() { ct.scale = Vec3::splat(m.zoom); } }
                Item::Restart => { let s = g.seed; crate::begin(&mut g, &mut commands, s); m.go(Screen::Off); }
                Item::Resume => m.go(Screen::Off),
                Item::Save => { crate::save_now(&mut g); m.go(Screen::Off); }
                Item::Load => { if crate::load_saved(&mut g, &mut commands) { m.go(Screen::Off); } }
                Item::Settings => m.go(Screen::Settings), Item::Controls => m.go(Screen::Controls),
                Item::MainMenu => { crate::save_auto(&g); m.go(Screen::Main); }
                Item::Quit => { crate::save_auto(&g); exit.write(AppExit::Success); }
                Item::Fullscreen => { m.fullscreen = !m.fullscreen; if let Ok(mut w) = win.single_mut() { w.mode = if m.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed }; } m.persist(); }
                Item::Back => { let b = m.back; m.go(b); }
                _ => {}
            },
            _ => {}
        }
        if keys.just_pressed(KeyCode::Escape) { match m.screen { Screen::Pause => m.go(Screen::Off), Screen::Settings | Screen::Controls => { let b = m.back; m.go(b); } _ => {} } }
        if !m.open() { g.paused = false; }
    }
    // draw
    let open = m.open();
    for mut v in q.p0().iter_mut() { *v = if open { Visibility::Inherited } else { Visibility::Hidden }; }
    if !open { return; }
    let items = m.items(); let sel = m.sel;
    let labels: Vec<String> = items.iter().map(|i| m.label(*i)).collect();
    for (r, mut t, mut c, _, mut v) in q.p1().iter_mut() {
        if let Some(l) = labels.get(r.0) { *v = Visibility::Inherited; t.0 = if r.0 == sel { format!("›  {l}  ‹") } else { l.clone() }; c.0 = if r.0 == sel { ACCENT } else { INK.with_alpha(0.8) }; }
        else { *v = Visibility::Hidden; }
    }
    let title = match m.screen { Screen::Pause => "Paused".to_string(), Screen::Settings => "Settings".into(), Screen::Controls => "Controls".into(), _ => "Molehill".into() };
    for mut t in q.p3().iter_mut() { t.0 = title.clone(); }
    let hint = match m.screen { Screen::Controls => CONTROLS.to_string(), Screen::Settings => "A / D adjust     Esc back".into(), Screen::Main => format!("W / S choose     Enter select\nholt {}", g.seed), _ => "W / S choose     Enter select     Esc resume".into() };
    for mut t in q.p2().iter_mut() { t.0 = hint.clone(); }
}
