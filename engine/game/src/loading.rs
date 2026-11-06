//! The loading line: a new holt is generated a slice per frame (`sim::world::NewHolt`) and a save is
//! read on the frame after the line is shown, so neither looks like a frozen window. One centred line
//! of text over a dark veil (what it is doing and how far along), nothing else.
use bevy::prelude::*;
use bevy::platform::time::Instant;
use molehill_sim::{survival::Mode, world::NewHolt};
use crate::{Game, menu::{Menu, Screen}};

/// Work per frame: steps run until this much time has gone (at least one step a frame).
const BUDGET_MS: f32 = 20.;
const INK: Color = Color::srgb(0.93, 0.90, 0.84);

pub enum Job {
    /// A new holt: built in steps, then started in `mode` (hunger off if asked).
    New { b: Box<NewHolt>, seed: u32, mode: Mode, hunger_off: bool, t0: Instant },
    /// The manual save (or the autosave): read once the line has been drawn.
    Load { shown: bool, t0: Instant },
}
#[derive(Resource, Default)] pub struct Loading { pub job: Option<Job> }
impl Loading {
    pub fn busy(&self) -> bool { self.job.is_some() }
    /// Start building a new holt (replaces anything in progress).
    pub fn new_holt(&mut self, seed: u32, mode: Mode, hunger_off: bool) {
        self.job = Some(Job::New { b: Box::new(NewHolt::new(crate::W, crate::H, seed)), seed, mode, hunger_off, t0: Instant::now() });
    }
    /// Load the save (the menu closes once it is in).
    pub fn load(&mut self) { self.job = Some(Job::Load { shown: false, t0: Instant::now() }); }
}
/// The loading line, from what a build is doing and how far along it is: "Laying down the rock…  12 %".
pub fn line(label: &str, progress: f32) -> String { format!("{label}…  {:>2} %", (progress.clamp(0., 1.) * 100.).floor() as u32) }

#[derive(Component)] pub struct LoadRoot;
#[derive(Component)] pub struct LoadText;
pub fn setup(commands: &mut Commands, f: &Handle<Font>) {
    commands.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.), height: Val::Percent(100.), align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(Color::srgba(0.03, 0.025, 0.035, 0.94)), GlobalZIndex(60), Visibility::Hidden, LoadRoot)).with_children(|p| {
        p.spawn((Text::new(""), TextFont { font: f.clone().into(), font_size: 22f32.into(), ..default() }, TextColor(INK), TextShadow::default(), LoadText));
    });
}

pub fn update(mut ld: ResMut<Loading>, mut g: ResMut<Game>, mut m: ResMut<Menu>, mut commands: Commands,
    mut root: Query<&mut Visibility, With<LoadRoot>>, mut text: Query<&mut Text, With<LoadText>>) {
    for mut v in &mut root { *v = if ld.busy() { Visibility::Inherited } else { Visibility::Hidden }; }
    let Some(job) = ld.job.as_mut() else { return };
    g.paused = true; g.w.input = Default::default();
    let mut done = false;
    let shown = match job {
        Job::New { b, .. } => {
            let t = Instant::now();
            loop { if b.step().is_some() { done = true; break; } if t.elapsed().as_secs_f32() * 1e3 >= BUDGET_MS { break; } }
            line(b.label(), b.progress())
        }
        Job::Load { shown, .. } => { if *shown { done = true; } *shown = true; "Opening the save…".to_string() }
    };
    for mut t in &mut text { t.0 = shown.clone(); }
    if !done { return; }
    match ld.job.take() {
        Some(Job::New { b, seed, mode, hunger_off, t0 }) => {
            crate::install(&mut g, &mut commands, b.take(), seed, mode); if hunger_off { g.w.set_hunger(false); }
            info!("new holt {seed} in {:.0} ms", t0.elapsed().as_secs_f32() * 1e3);
            m.go(Screen::Off);
        }
        Some(Job::Load { t0, .. }) => {
            if crate::load_saved(&mut g, &mut commands) { m.go(Screen::Off); }
            info!("save read in {:.0} ms", t0.elapsed().as_secs_f32() * 1e3);
        }
        None => {}
    }
    // back in the game: the key or click that started it doesn't act there
    if !m.open() { g.paused = false; g.w.swallow_held(); g.mouse_latch = true; }
    for mut v in &mut root { *v = Visibility::Hidden; }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_says_what_and_how_far() {
        assert_eq!(line("Laying down the rock", 0.4), "Laying down the rock…  40 %");
        assert_eq!(line("Shoring up the caves", 0.), "Shoring up the caves…   0 %");
        assert_eq!(line("x", 1.7), "x…  100 %");
    }
}
