//! Minimal game HUD: vitals, quest line, toasts, hotbar, inventory grid (I), help (H), dev text (F3).
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use molehill_sim::craft::{Item, ITEMS, QUESTS};
use molehill_sim::logic::DEVS;
use crate::{Game, Thing, Sprites, NSLOT, thing_count, thing_name};

pub const S: f32 = 44.; pub const GAP: f32 = 4.;
#[derive(Resource)] pub struct UiFont(pub Handle<Font>);
#[derive(Component)] pub struct Slot(pub usize);
#[derive(Component)] pub struct SlotIcon(pub usize);
#[derive(Component, Clone, Copy, PartialEq)] pub enum Lbl { Count(usize), Tip, Quest, Toast, Sel, Center, Dev }
#[derive(Component)] pub struct InvBg;
#[derive(Component)] pub struct Held;
#[derive(Component)] pub struct Bar(pub u8);
#[derive(Component)] pub struct HelpPanel;

/// Top-left corner of slot k (0..10 hotbar along the bottom; 10.. the pack grid above it).
pub fn slot_pos(k: usize, ww: f32, wh: f32) -> Vec2 {
    let x0 = (ww - (10. * S + 9. * GAP)) / 2.; let col = (k % 10) as f32;
    let y = if k < 10 { wh - 14. - S } else { let rows = ((NSLOT - 10) / 10) as f32; wh - 14. - S - 20. - (rows - ((k - 10) / 10) as f32) * (S + GAP) };
    Vec2::new(x0 + col * (S + GAP), y)
}
pub fn slot_at(p: Vec2, ww: f32, wh: f32, inv_open: bool) -> Option<usize> {
    (0..if inv_open { NSLOT } else { 10 }).find(|&k| { let q = slot_pos(k, ww, wh); p.x >= q.x && p.x < q.x + S && p.y >= q.y && p.y < q.y + S })
}
fn tf(f: &Handle<Font>, s: f32) -> TextFont { TextFont { font: f.clone().into(), font_size: s.into(), ..default() } }
const INK: Color = Color::srgb(0.93, 0.90, 0.84);
const PANEL: Color = Color::srgba(0.05, 0.04, 0.05, 0.55);

pub fn setup(commands: &mut Commands, f: &Handle<Font>) {
    let abs = |l: f32, t: f32| Node { position_type: PositionType::Absolute, left: Val::Px(l), top: Val::Px(t), ..default() };
    // vitals
    commands.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(14.), top: Val::Px(14.), flex_direction: FlexDirection::Column, row_gap: Val::Px(5.), ..default() })).with_children(|p| {
        for (k, c) in [Color::srgb(0.86, 0.25, 0.25), Color::srgb(0.88, 0.62, 0.25), Color::srgb(0.35, 0.65, 0.95)].into_iter().enumerate() {
            p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.), ..default() }).with_children(|r| {
                r.spawn((Node { width: Val::Px(9.), height: Val::Px(9.), border_radius: BorderRadius::all(Val::Px(if k == 2 { 5. } else { 2. })), ..default() }, BackgroundColor(c)));
                r.spawn((Node { width: Val::Px(120.), height: Val::Px(7.), border_radius: BorderRadius::all(Val::Px(2.)), ..default() }, BackgroundColor(Color::srgba(0., 0., 0., 0.45)))).with_children(|b| {
                    b.spawn((Node { width: Val::Percent(100.), height: Val::Percent(100.), border_radius: BorderRadius::all(Val::Px(2.)), ..default() }, BackgroundColor(c), Bar(k as u8)));
                });
            });
        }
        p.spawn((Text::new(""), tf(f, 15.), TextColor(INK.with_alpha(0.8)), TextShadow::default(), Node { margin: UiRect::top(Val::Px(4.)), ..default() }, Lbl::Quest));
    });
    commands.spawn((Text::new(""), tf(f, 16.), TextColor(INK), TextShadow::default(), TextLayout { justify: Justify::Center, ..default() }, Node { position_type: PositionType::Absolute, top: Val::Px(14.), width: Val::Percent(100.), justify_content: JustifyContent::Center, ..default() }, Lbl::Toast));
    commands.spawn((Text::new(""), tf(f, 15.), TextColor(INK.with_alpha(0.85)), TextShadow::default(), TextLayout { justify: Justify::Center, ..default() }, Node { position_type: PositionType::Absolute, bottom: Val::Px(66.), width: Val::Percent(100.), ..default() }, Lbl::Sel));
    commands.spawn((Text::new(""), tf(f, 28.), TextColor(INK), TextShadow::default(), TextLayout { justify: Justify::Center, ..default() }, Node { position_type: PositionType::Absolute, top: Val::Percent(38.), width: Val::Percent(100.), ..default() }, Lbl::Center));
    // pack background + slots
    commands.spawn((abs(0., 0.), BackgroundColor(PANEL), Visibility::Hidden, InvBg)).insert(Node { position_type: PositionType::Absolute, border_radius: BorderRadius::all(Val::Px(6.)), ..default() });
    for k in 0..NSLOT {
        commands.spawn((abs(0., 0.), Slot(k), BackgroundColor(Color::srgba(0.08, 0.07, 0.08, 0.5)), BorderColor::all(Color::srgba(1., 1., 1., 0.12)))).insert(Node { position_type: PositionType::Absolute, width: Val::Px(S), height: Val::Px(S), border: UiRect::all(Val::Px(2.)), border_radius: BorderRadius::all(Val::Px(5.)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|s| {
            s.spawn((ImageNode::default(), Node { width: Val::Px(30.), height: Val::Px(30.), ..default() }, SlotIcon(k)));
            s.spawn((Text::new(""), tf(f, 13.), TextColor(INK), TextShadow::default(), Node { position_type: PositionType::Absolute, right: Val::Px(3.), bottom: Val::Px(0.), ..default() }, Lbl::Count(k)));
            if k < 10 { s.spawn((Text::new(format!("{}", (k + 1) % 10)), tf(f, 11.), TextColor(INK.with_alpha(0.45)), Node { position_type: PositionType::Absolute, left: Val::Px(3.), top: Val::Px(0.), ..default() })); }
        });
    }
    commands.spawn((ImageNode::default(), Node { position_type: PositionType::Absolute, width: Val::Px(30.), height: Val::Px(30.), ..default() }, GlobalZIndex(10), Visibility::Hidden, Held));
    commands.spawn((Text::new(""), tf(f, 14.), TextColor(INK), TextShadow::default(), Node { position_type: PositionType::Absolute, padding: UiRect::axes(Val::Px(6.), Val::Px(3.)), border_radius: BorderRadius::all(Val::Px(4.)), ..default() }, BackgroundColor(PANEL), GlobalZIndex(11), Visibility::Hidden, Lbl::Tip));
    // help
    let help = "Move        A / D        Jump        Space / W\nWalls       hold into a wall: W/S climb, Space wall-jump\nDig down    S (on ground)            Zoom        + / -  or  Ctrl+wheel\nUse item    Left mouse   Interact    Right mouse (levers, thrum channels)\nHotbar      1-0 / wheel  Pack        I  (click to pick up and drop items)\nPick up     Ctrl+Right mouse (copperline, devices)\nPause Esc   Save K   Load L   New world N   Dev info F3   Close H";
    commands.spawn((Node { position_type: PositionType::Absolute, left: Val::Percent(50.), top: Val::Percent(50.), margin: UiRect { left: Val::Px(-300.), top: Val::Px(-110.), ..default() }, width: Val::Px(600.), padding: UiRect::all(Val::Px(18.)), border_radius: BorderRadius::all(Val::Px(8.)), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.), ..default() }, BackgroundColor(Color::srgba(0.05, 0.04, 0.05, 0.78)), Visibility::Hidden, GlobalZIndex(5), HelpPanel)).with_children(|p| {
        p.spawn((Text::new("Controls"), tf(f, 20.), TextColor(INK)));
        p.spawn((Text::new(help), tf(f, 15.), TextColor(INK.with_alpha(0.85))));
    });
    commands.spawn((Text::new(""), tf(f, 13.), TextColor(Color::srgb(0.7, 0.95, 0.7)), TextShadow::default(), Node { position_type: PositionType::Absolute, right: Val::Px(12.), top: Val::Px(12.), max_width: Val::Px(460.), ..default() }, Visibility::Hidden, Lbl::Dev));
}

pub fn update(g: Res<Game>, sp: Res<Sprites>, time: Res<Time>, win: Query<&Window, With<PrimaryWindow>>,
    mut q: ParamSet<(
        Query<(&Slot, &mut Node, &mut BackgroundColor, &mut BorderColor, &mut Visibility)>,
        Query<(&SlotIcon, &mut ImageNode, &mut Visibility)>,
        Query<(&Bar, &mut Node)>,
        Query<(&mut Node, &mut Visibility), With<InvBg>>,
        Query<(&mut Node, &mut ImageNode, &mut Visibility), With<Held>>,
        Query<(&Lbl, &mut Text, &mut TextColor, &mut Node, &mut Visibility)>,
        Query<&mut Visibility, With<HelpPanel>>,
    )>,
) {
    let Ok(win) = win.single() else { return }; let (ww, wh) = (win.width(), win.height());
    let w = &g.w; let me = &w.moles[0]; let now = time.elapsed_secs();
    for (s, mut n, mut bg, mut bc, mut vis) in &mut q.p0() {
        let p = slot_pos(s.0, ww, wh); if n.left != Val::Px(p.x) { n.left = Val::Px(p.x); } if n.top != Val::Px(p.y) { n.top = Val::Px(p.y); }
        *vis = if s.0 < 10 || g.inv_open { Visibility::Visible } else { Visibility::Hidden };
        let sel = s.0 == g.hot; *bc = BorderColor::all(if sel { Color::srgba(1., 0.88, 0.55, 0.9) } else { Color::srgba(1., 1., 1., 0.10) });
        bg.0 = if sel { Color::srgba(0.25, 0.2, 0.12, 0.6) } else { Color::srgba(0.08, 0.07, 0.08, 0.5) };
    }
    for (s, mut im, mut vis) in &mut q.p1() { match g.slots[s.0] { Some(t) => { let h = icon(&sp, t); if im.image != h { im.image = h; } *vis = Visibility::Inherited; } None => *vis = Visibility::Hidden } }
    let vals = [me.hp, me.hunger, me.breath];
    for (b, mut n) in &mut q.p2() { n.width = Val::Percent(vals[b.0 as usize].clamp(0., 100.)); }
    { let mut p = q.p3(); if let Ok((mut n, mut v)) = p.single_mut() { let a = slot_pos(10, ww, wh); let b = slot_pos(9, ww, wh); n.left = Val::Px(a.x - 10.); n.top = Val::Px(a.y - 10.); n.width = Val::Px(b.x + S - a.x + 20.); n.height = Val::Px(b.y + S - a.y + 20.); *v = if g.inv_open { Visibility::Visible } else { Visibility::Hidden }; } }
    let cur = win.cursor_position();
    { let mut p = q.p4(); if let Ok((mut n, mut im, mut v)) = p.single_mut() { match (g.held, cur) { (Some(t), Some(c)) => { im.image = icon(&sp, t); n.left = Val::Px(c.x - 8.); n.top = Val::Px(c.y - 8.); *v = Visibility::Visible; } _ => *v = Visibility::Hidden } } }
    let hov = cur.and_then(|c| slot_at(c, ww, wh, g.inv_open).map(|k| (c, k)));
    for (l, mut t, mut c, mut n, mut v) in &mut q.p5() {
        let mut set = |t: &mut Text, s: String| if t.0 != s { t.0 = s; };
        match *l {
            Lbl::Count(k) => { let x = g.slots[k].and_then(|th| thing_count(&g, th)).map(|n| format!("{}", n.min(999))).unwrap_or_default(); set(&mut t, x); }
            Lbl::Tip => match hov.and_then(|(c, k)| g.slots[k].map(|th| (c, th))) { Some((cp, th)) if g.inv_open && g.held.is_none() => { set(&mut t, thing_name(&g, th)); n.left = Val::Px(cp.x + 14.); n.top = Val::Px(cp.y + 14.); *v = Visibility::Visible; } _ => *v = Visibility::Hidden },
            Lbl::Quest => set(&mut t, if w.quest < QUESTS.len() { format!("{}{}", QUESTS[w.quest].text, if w.quest_done(w.quest) { "  (done)" } else { "" }) } else { String::new() }),
            Lbl::Toast => { let live: Vec<&(String, f32)> = g.toasts.iter().filter(|(_, t0)| now - t0 < 4.).collect();
                set(&mut t, live.iter().map(|(m, _)| m.as_str()).collect::<Vec<_>>().join("\n"));
                let age = live.last().map_or(9., |(_, t0)| now - t0); c.0 = INK.with_alpha((4. - age).clamp(0., 1.)); }
            Lbl::Sel => { set(&mut t, if g.inv_open { String::new() } else { g.slots[g.hot].map(|th| thing_name(&g, th)).unwrap_or_default() }); c.0 = INK.with_alpha(((2.2 - (now - g.sel_t)) * 1.5).clamp(0., 0.85)); }
            Lbl::Center => set(&mut t, if !me.alive { "You were lost to the deep.\nCrawling back to the burrow...".to_string() } else { String::new() }),
            Lbl::Dev => { *v = if g.show_dev { Visibility::Visible } else { Visibility::Hidden };
                if g.show_dev && g.frame % 10 == 0 {
                    set(&mut t, format!("fps {:.0}  seed {}  tick {}  oy {}\npos {},{}  depth {:+.1} m (deepest {:+.1})\nhp {:.0} food {:.0} breath {:.0}  air {:.2}  deaths {}\nmining {:?}  power {}  uploads {}\ndisaster {:?}\n{}",
                        1. / time.delta_secs().max(1e-3), w.info.seed, w.tick, w.oy, me.x, me.y, w.depth_m(me.x, me.y), w.max_depth, me.hp, me.hunger, me.breath, w.air.at(me.x, me.y.saturating_sub(1)), w.pl.deaths,
                        w.pl.mine.map(|(_, a, b)| (a, b)), w.dig_power(), g.uploads, w.disasters.phase, w.log.iter().rev().take(6).map(|(_, l)| l.as_str()).collect::<Vec<_>>().join("\n"))); } }
        }
    }
    if let Ok(mut v) = q.p6().single_mut() { *v = if g.help { Visibility::Visible } else { Visibility::Hidden }; }
}

pub fn icon(sp: &Sprites, t: Thing) -> Handle<Image> {
    match t {
        Thing::Pick(n) => sp.tools[(n as usize - 1).min(2)].clone(),
        Thing::Item(it) => if it == Item::Torch { sp.torch.clone() } else { sp.icons[it as usize].clone() },
        Thing::Dev(d) => match DEVS[d] { molehill_sim::logic::Dev::Tnt => sp.icons[Item::Tnt as usize].clone(), dv if dv.is_thrum() => sp.icons[Item::Thrummer as usize].clone(), dv if dv.is_gate() => sp.icons[Item::Gate as usize].clone(), molehill_sim::logic::Dev::Lamp => sp.icons[Item::Lamp as usize].clone(), molehill_sim::logic::Dev::Vent => sp.icons[Item::Vent as usize].clone(), _ => sp.dev.clone() },
        Thing::Station(s) => sp.station[s][1].clone(),
    }
}
#[allow(dead_code)] pub fn all_items() -> &'static [Item] { &ITEMS }
