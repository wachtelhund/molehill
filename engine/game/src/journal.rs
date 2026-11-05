//! The journal (`J`): the Lid fragments found so far, newest first, as a plain list. No cards: a title
//! line per fragment (kind, title) and its text under it, in the HUD's font and ink.
use bevy::prelude::*;
use crate::Game;
use molehill_sim::lore::FRAGMENTS;

#[derive(Resource, Default)] pub struct Journal { pub open: bool }
#[derive(Component)] pub struct JournalRoot;
#[derive(Component)] pub struct JournalText;
const INK: Color = Color::srgb(0.93, 0.90, 0.84);

pub fn setup(commands: &mut Commands, f: &Handle<Font>) {
    let tf = |s: f32| TextFont { font: f.clone().into(), font_size: s.into(), ..default() };
    commands.init_resource::<Journal>();
    commands.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(28.), top: Val::Px(70.), width: Val::Px(470.), max_height: Val::Percent(78.), padding: UiRect::all(Val::Px(14.)), flex_direction: FlexDirection::Column, row_gap: Val::Px(8.), overflow: Overflow::clip(), ..default() },
        BackgroundColor(Color::srgba(0.04, 0.035, 0.045, 0.82)), GlobalZIndex(20), Visibility::Hidden, JournalRoot)).with_children(|p| {
        p.spawn((Text::new("Journal"), tf(22.), TextColor(INK), TextShadow::default()));
        p.spawn((Text::new(""), tf(14.), TextColor(Color::srgb(0.82, 0.79, 0.72)), TextShadow::default(), JournalText));
    });
}

pub fn update(g: Res<Game>, mut j: ResMut<Journal>, menu: Res<crate::menu::Menu>, keys: Res<ButtonInput<KeyCode>>,
    mut root: Query<&mut Visibility, With<JournalRoot>>, mut text: Query<&mut Text, With<JournalText>>, mut seen: Local<usize>) {
    if !menu.open() && keys.just_pressed(KeyCode::KeyJ) { j.open = !j.open; }
    if menu.open() && j.open { j.open = false; }
    if let Ok(mut v) = root.single_mut() { *v = if j.open { Visibility::Inherited } else { Visibility::Hidden }; }
    if !j.open || (*seen == g.w.journal.len() && g.w.journal.len() > 0) { return; }
    *seen = g.w.journal.len();
    let s = if g.w.journal.is_empty() { "Nothing yet. Old Delvers left marks in the rock, and the higher you go, the more of them there are.".to_string() }
        else { let mut s = format!("{} found\n", g.w.journal.len());
            for &f in g.w.journal.iter().rev() { let fr = &FRAGMENTS[f as usize]; s += &format!("\n{}  ·  {}\n{}\n", fr.kind.name(), fr.title, fr.text); } s };
    if let Ok(mut t) = text.single_mut() { if t.0 != s { t.0 = s; } }
}
