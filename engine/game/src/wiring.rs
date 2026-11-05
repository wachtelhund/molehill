//! The wiring layer, drawn as sprites over the world: copperline and devices as pixel art, and
//! **wiring mode** (`V`): a chunky schematic (thick wires coloured by current with a travelling pulse,
//! devices by role, knot inputs/outputs and names, thrum links and reach). The world dims, a border, a
//! small label and a crosshair cursor make the mode unmistakable. One sprite per block and layer,
//! kept in a map so a big copperline net costs nothing to match each frame.
use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use std::collections::HashMap;
use molehill_sim::logic::{Dev, DEVS, B};
use molehill_sim::grid::F_SEEN;
use crate::devart::{self, E, N, S, W as WEST};
use crate::{cell_to_world, Game, Thing, CPX};

#[derive(Resource, Default)] pub struct Wiring { pub on: bool }
#[derive(Resource)] pub struct WireArt {
    wire: Vec<Handle<Image>>,          // mask*5 + (0 unlit, 1..=4 lit glint frames)
    dev: Vec<[Handle<Image>; 5]>,      // per Dev (index = Dev as u8): off, on frames 0..3
    s_wire: Vec<Handle<Image>>, s_dev: Vec<Handle<Image>>, arrow: Handle<Image>, px: Handle<Image>, ring: Handle<Image>,
}
#[derive(Component)] pub struct WireSpr;
/// Sprite and label entities by key, and the mode last applied.
#[derive(Default)] pub struct Caches { cache: HashMap<u64, Entity>, lcache: HashMap<u64, Entity>, was: Option<bool> }
#[derive(Component)] pub struct WireLabel;
#[derive(Component)] pub struct ModeFrame;
#[derive(Component)] pub struct ModeLabel;
#[derive(Component)] pub struct ModeHint;

/// Thrum channel colours (pattern 0..7).
const CH: [[f32; 3]; 8] = [[0.4, 0.9, 1.], [1., 0.55, 0.85], [0.6, 1., 0.45], [1., 0.85, 0.35], [0.7, 0.6, 1.], [1., 0.5, 0.4], [0.45, 1., 0.85], [0.95, 0.95, 0.95]];

pub fn setup(commands: &mut Commands, images: &mut Assets<Image>, font: &Handle<Font>, ring: Handle<Image>, px: Handle<Image>) {
    let mut wire = vec![];
    for m in 0..16u8 { wire.push(images.add(devart::wire(m, false, 0))); for f in 0..4 { wire.push(images.add(devart::wire(m, true, f))); } }
    let mut dev = vec![]; for k in 0..=DEVS.len() as u8 { let d = Dev::from_u8(k); dev.push([0u8, 1, 2, 3, 4].map(|s| images.add(devart::device(d, s > 0, s.saturating_sub(1))))); }
    let s_wire = (0..16u8).map(|m| images.add(devart::s_wire(m))).collect();
    let s_dev = (0..=DEVS.len() as u8).map(|k| images.add(devart::s_device(Dev::from_u8(k)))).collect();
    commands.insert_resource(WireArt { wire, dev, s_wire, s_dev, arrow: images.add(devart::s_arrow()), px, ring });
    // `--wiring` (native, for dumps) starts in wiring mode
    commands.insert_resource(Wiring { on: cfg!(not(target_arch = "wasm32")) && std::env::args().any(|a| a == "--wiring") });
    // mode frame: a thin bright border and the faintest cool tint over the whole view, plus a small label
    let tf = |s: f32| TextFont { font: font.clone().into(), font_size: s.into(), ..default() };
    commands.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.), height: Val::Percent(100.), border: UiRect::all(Val::Px(3.)), ..default() },
        BorderColor::all(Color::srgba(0.45, 0.85, 1., 0.85)), BackgroundColor(Color::srgba(0.1, 0.25, 0.4, 0.08)), GlobalZIndex(5), Visibility::Hidden, ModeFrame));
    commands.spawn((Text::new("WIRING   V to leave"), tf(16.), TextColor(Color::srgb(0.6, 0.92, 1.)), TextShadow::default(),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.), left: Val::Percent(50.), margin: UiRect::left(Val::Px(-80.)), ..default() }, GlobalZIndex(6), Visibility::Hidden, ModeLabel));
    commands.spawn((Text::new("V  wiring view"), tf(14.), TextColor(Color::srgba(0.75, 0.9, 1., 0.7)), TextShadow::default(),
        Node { position_type: PositionType::Absolute, top: Val::Px(10.), left: Val::Percent(50.), margin: UiRect::left(Val::Px(-55.)), ..default() }, GlobalZIndex(6), Visibility::Hidden, ModeHint));
}

/// Things that make the wiring-mode hint show while held.
fn wiring_item(t: Option<Thing>) -> bool {
    use molehill_sim::craft::Item;
    match t { Some(Thing::Dev(_)) => true, Some(Thing::Item(it)) => matches!(it, Item::CopperWire | Item::Gate | Item::Thrummer | Item::Lamp | Item::Tnt), _ => false }
}

#[derive(Clone, Copy, PartialEq)] enum Role { Source, Knot, Sink, Thrum }
fn role(d: Dev) -> Role {
    match d { Dev::Lever | Dev::Plate | Dev::Sensor | Dev::Heat | Dev::Wheel | Dev::Rx => Role::Source, Dev::Tx | Dev::Rep => Role::Thrum,
        _ if d.is_gate() => Role::Knot, _ => Role::Sink }
}
fn knot_word(d: Dev) -> &'static str { match d { Dev::And => "BOTH", Dev::Or => "EITHER", Dev::Not => "TURN", Dev::Xor => "ODD", Dev::Latch => "CATCH", Dev::Timer => "TICKER", _ => "" } }

/// Rough reach of a knock (m) through the material the thrummer is braced in: where the 60 dB budget
/// runs out to spreading and absorption alone (boundaries cost more; the drawn links are the truth).
fn thrum_reach(m: molehill_sim::mat::Mat, loose: bool) -> f32 {
    let (_, att) = molehill_sim::mat::wave(m, loose); let per_m = att / molehill_sim::mat::CELL_M;
    let mut r = 1f32; while r < 400. { if molehill_sim::logic::THRUM_POWER - 10. * r.log10() - per_m * r < 0. { break; } r += 0.5; } r
}

#[allow(clippy::too_many_arguments)]
pub fn update(mut commands: Commands, g: Res<Game>, art: Res<WireArt>, mut wm: ResMut<Wiring>, menu: Res<crate::menu::Menu>, keys: Res<ButtonInput<KeyCode>>, time: Res<Time>,
    font: Res<crate::ui::UiFont>, cam: Query<(&Transform, &Projection), With<Camera2d>>, mut win: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut spr: Query<(&mut Transform, &mut Sprite), (With<WireSpr>, Without<Camera2d>)>, mut labels: Query<(&mut Transform, &mut Text2d, &mut TextColor), (With<WireLabel>, Without<WireSpr>, Without<Camera2d>)>,
    mut world: Query<&mut Sprite, (Or<(With<crate::WorldSprite>, With<crate::BgLayer>)>, Without<WireSpr>)>,
    mut ui: ParamSet<(Query<&mut Visibility, With<ModeFrame>>, Query<&mut Visibility, With<ModeLabel>>, Query<&mut Visibility, With<ModeHint>>)>,
    mut st: Local<Caches>) {
    let Caches { cache, lcache, was } = &mut *st;
    if !menu.open() && keys.just_pressed(KeyCode::KeyV) { wm.on = !wm.on; }
    let mode = wm.on && !menu.open();
    // mode chrome: dim the world, frame + label, crosshair; the hint shows when holding wiring things
    if *was != Some(mode) {
        *was = Some(mode);
        let k = if mode { 0.42 } else { 1. }; for mut s in &mut world { let a = s.color.alpha(); s.color = Color::srgba(k, k, k, a); }
        let v = if mode { Visibility::Inherited } else { Visibility::Hidden };
        for mut x in &mut ui.p0() { *x = v; } for mut x in &mut ui.p1() { *x = v; }
        if let Ok((e, _)) = win.single_mut() { commands.entity(e).insert(if mode { CursorIcon::System(SystemCursorIcon::Crosshair) } else { CursorIcon::System(SystemCursorIcon::Default) }); }
    }
    { let show = !mode && !menu.open() && wiring_item(g.slots.get(g.hot).copied().flatten()); for mut x in &mut ui.p2() { *x = if show { Visibility::Inherited } else { Visibility::Hidden }; } }

    let (Ok((ct, _)), Ok((_, wn))) = (cam.single(), win.single()) else { return };
    let w = &g.w; let lg = &w.logic; let bw = lg.bw; let now = time.elapsed_secs();
    // blocks in view (+1 margin)
    let sc = ct.scale.x; let (hw, hh) = (wn.width() / 2. * sc, wn.height() / 2. * sc);
    let (c0x, c0y) = crate::world_to_cell(Vec2::new(ct.translation.x - hw, ct.translation.y + hh)); let (c1x, c1y) = crate::world_to_cell(Vec2::new(ct.translation.x + hw, ct.translation.y - hh));
    let bx0 = (c0x.max(0) as usize / B).saturating_sub(1); let by0 = (c0y.max(0) as usize / B).saturating_sub(1);
    let bx1 = ((c1x.max(0) as usize / B) + 1).min(bw - 1); let by1 = ((c1y.max(0) as usize / B) + 1).min(lg.bh - 1);
    let con = |c: usize| c < lg.wire.len() && (lg.wire[c] != 0 || lg.dev[c] != 0);
    let light = |b: usize| { let (x, y) = ((b % bw) * B + B / 2, (b / bw) * B + B / 2); let l = w.light.at(y.min(w.g.h - 1) * w.g.w + x.min(w.g.w - 1)); (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).clamp(0.22, 1.) };
    // (key, pos, z, image, size, colour, rotation)
    let mut want: Vec<(u64, Vec2, f32, Handle<Image>, Vec2, Color, f32)> = vec![];
    let mut texts: Vec<(u64, Vec2, String, Color)> = vec![];
    let bs = B as f32 * CPX; // block size in world units (16)
    let key = |b: usize, layer: u64| (b as u64) << 8 | layer;
    for by in by0..=by1 { for bx in bx0..=bx1 {
        let b = by * bw + bx; if lg.wire[b] == 0 && lg.dev[b] == 0 { continue; }
        let (cx, cy) = (bx * B + B / 2, by * B + B / 2); if w.g.flags[cy * w.g.w + cx] & F_SEEN == 0 { continue; }
        let pos = cell_to_world((bx * B) as f32 + B as f32 / 2., (by * B) as f32 + B as f32 / 2.);
        let mask = (if by > 0 && con(b - bw) { N } else { 0 }) | (if bx + 1 < bw && con(b + 1) { E } else { 0 }) | (if by + 1 < lg.bh && con(b + bw) { S } else { 0 }) | (if bx > 0 && con(b - 1) { WEST } else { 0 });
        let d = Dev::from_u8(lg.dev[b]); let lit = lg.net_lit(b);
        let on = d != Dev::None && (lg.dev_s[b] != 0 || lg.powered[b] != 0 || lg.burn[b] > 0);
        let f = (((now * 6.) as usize + bx + by) % 4) as u8;
        if !mode {
            let l = light(b);
            if lg.wire[b] != 0 { let k = if lit { l.max(0.85) } else { l }; want.push((key(b, 0), pos, 1.4, art.wire[mask as usize * 5 + if lit { 1 + f as usize } else { 0 }].clone(), Vec2::splat(bs), Color::srgb(k, k, k), 0.)); }
            if d != Dev::None {
                let k = if on { l.max(0.9) } else { l }; let st = if on { 1 + f as usize } else { 0 };
                want.push((key(b, 1), pos, 1.5, art.dev[d as usize][st].clone(), Vec2::splat(bs), Color::srgb(k, k, k), 0.));
                if d.is_thrum() { let c = CH[(lg.dev_p[b].max(0) as usize) % 8]; want.push((key(b, 2), pos + Vec2::new(5., 5.), 1.6, art.px.clone(), Vec2::splat(3.), Color::srgb(c[0] * k, c[1] * k, c[2] * k), 0.)); }
            }
        } else {
            // schematic: wires by current (amber, pulsing along the net) or idle (steel blue)
            if lg.wire[b] != 0 || mask != 0 {
                let c = if lit { let p = 0.78 + 0.22 * (now * 6. - (bx + by) as f32 * 0.9).sin(); Color::srgb(1. * p, 0.78 * p, 0.25 * p) } else { Color::srgb(0.32, 0.5, 0.68) };
                if lg.wire[b] != 0 { want.push((key(b, 10), pos, 7.0, art.s_wire[mask as usize].clone(), Vec2::splat(bs), c, 0.)); }
            }
            if d != Dev::None {
                let r = role(d); let base = match r { Role::Source => [0.45, 1., 0.55], Role::Knot => [0.82, 0.62, 1.], Role::Sink => [1., 0.62, 0.3], Role::Thrum => CH[(lg.dev_p[b].max(0) as usize) % 8] };
                let k = if on { 1. } else { 0.5 }; let col = Color::srgb(base[0] * k, base[1] * k, base[2] * k);
                want.push((key(b, 11), pos, 7.05, art.px.clone(), Vec2::splat(bs - 2.), Color::srgba(0.02, 0.03, 0.05, 0.85), 0.));
                want.push((key(b, 12), pos, 7.1, art.s_dev[d as usize].clone(), Vec2::splat(bs), col, 0.));
                if r == Role::Knot {
                    // inputs from the west and north, output to the east: arrows on the block's edges
                    let a_in = bx > 0 && lg.net_lit(b - 1); let n_in = by > 0 && lg.net_lit(b - bw);
                    let ac = |v: bool| if v { Color::srgb(1., 0.8, 0.3) } else { Color::srgb(0.55, 0.65, 0.8) };
                    want.push((key(b, 13), pos + Vec2::new(-bs / 2. - 1., 0.), 7.2, art.arrow.clone(), Vec2::splat(6.), ac(a_in), 0.));
                    if d != Dev::Not { want.push((key(b, 14), pos + Vec2::new(0., bs / 2. + 1.), 7.2, art.arrow.clone(), Vec2::splat(6.), ac(n_in), -std::f32::consts::FRAC_PI_2)); }
                    want.push((key(b, 15), pos + Vec2::new(bs / 2. + 2., 0.), 7.2, art.arrow.clone(), Vec2::splat(6.), ac(on), 0.));
                    texts.push((key(b, 16), pos + Vec2::new(0., -bs / 2. - 5.), knot_word(d).into(), Color::srgb(0.85, 0.75, 1.)));
                }
                if r == Role::Thrum || d == Dev::Rx {
                    let ch = (lg.dev_p[b].max(0) as usize) % 8; let c = CH[ch];
                    texts.push((key(b, 16), pos + Vec2::new(0., -bs / 2. - 5.), format!("CH {}", ch + 1), Color::srgb(c[0], c[1], c[2])));
                    if r == Role::Thrum { // reach ring through the rock it's braced in
                        let (x, y) = (cx, (by * B + B + 1).min(w.g.h - 1)); let i = y * w.g.w + x; let reach = thrum_reach(w.g.m(i), w.g.loose(i));
                        let dia = 2. * reach * molehill_sim::mat::CPM as f32 * CPX; want.push((key(b, 17), pos, 6.9, art.ring.clone(), Vec2::splat(dia), Color::srgba(c[0], c[1], c[2], 0.35), 0.));
                    }
                }
            }
        }
    } }
    if mode { // thrum links: a line per live pair, in the sender's channel colour
        for (k, &(a, b2, _)) in lg.links.iter().enumerate() {
            let p = |b: usize| cell_to_world(((b % bw) * B) as f32 + B as f32 / 2., ((b / bw) * B) as f32 + B as f32 / 2.);
            let (pa, pb) = (p(a), p(b2)); let d = pb - pa; let c = CH[(lg.dev_p[a].max(0) as usize) % 8];
            want.push(((1u64 << 60) | k as u64, (pa + pb) / 2., 6.95, art.px.clone(), Vec2::new(d.length(), 1.5), Color::srgba(c[0], c[1], c[2], 0.6), d.to_angle()));
        }
    }
    // apply: update what exists, spawn what's new, drop what's gone
    let mut keep = std::collections::HashSet::new();
    for (k, pos, z, img, size, col, rot) in want {
        keep.insert(k);
        if let Some(&e) = cache.get(&k) { if let Ok((mut t, mut s)) = spr.get_mut(e) {
            t.translation = pos.extend(z); t.rotation = Quat::from_rotation_z(rot); if s.image != img { s.image = img; } s.custom_size = Some(size); s.color = col; continue; } }
        let e = commands.spawn((Sprite { image: img, custom_size: Some(size), color: col, ..default() }, Transform::from_translation(pos.extend(z)).with_rotation(Quat::from_rotation_z(rot)), WireSpr)).id();
        cache.insert(k, e);
    }
    cache.retain(|k, e| { if keep.contains(k) { true } else { commands.entity(*e).despawn(); false } });
    let mut lkeep = std::collections::HashSet::new();
    for (k, pos, s, col) in texts {
        lkeep.insert(k);
        if let Some(&e) = lcache.get(&k) { if let Ok((mut t, mut tx, mut tc)) = labels.get_mut(e) { t.translation = pos.extend(7.3); if tx.0 != s { tx.0 = s; } tc.0 = col; continue; } }
        let e = commands.spawn((Text2d::new(s), TextFont { font: font.0.clone().into(), font_size: 7.0.into(), ..default() }, TextColor(col), Transform::from_translation(pos.extend(7.3)), WireLabel)).id();
        lcache.insert(k, e);
    }
    lcache.retain(|k, e| { if lkeep.contains(k) { true } else { commands.entity(*e).despawn(); false } });
}
