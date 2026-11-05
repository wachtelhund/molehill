//! Generative ambient music: a slow, sparse composer that hands out timed notes for the WebAudio
//! synth in `audio.rs` to play. Pure Rust (no audio here) so it can be tested natively.
//!
//! Shape: a piece plays for one to two minutes, then a long near-silent rest, then another piece.
//! Each piece picks a texture (pads and plucks, a lone kalimba, pads and bells, or a drone with the odd
//! pluck) and a mood from where the mole is: airy and major up in the wet layers, earthy in the
//! middle, darker and lower in the deep, and low and smouldering near lava. Melodies are a gentle random walk
//! on a pentatonic-ish scale at a slow pulse, with lots of space. The synth adds the reverb and echo.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Voice { Pad, Pluck, Bell, Drone }
#[derive(Clone, Copy, Debug)] pub struct Note { pub t: f64, pub voice: Voice, pub hz: f32, pub dur: f32, pub vel: f32 }

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Mood { Seep, Loam, Deep, Ember }
pub struct MoodDef { pub root: i32, pub scale: &'static [i32], pub prog: &'static [i32], pub cutoff: f32, pub pluck_oct: i32, pub drone: f32, pub beat: f64 }
impl Mood {
    /// From depth in metres (positive = deeper) and nearby heat (0..1 share of lava around the mole).
    pub fn pick(depth_m: f32, heat: f32) -> Mood { if heat > 0.04 || depth_m > 70. { Mood::Ember } else if depth_m > 25. { Mood::Deep } else if depth_m < -15. { Mood::Seep } else { Mood::Loam } }
    pub fn def(self) -> MoodDef {
        match self {
            // D major pentatonic, bright and open: shallow, wet layers
            Mood::Seep => MoodDef { root: 50, scale: &[0, 2, 4, 7, 9], prog: &[0, 3, 1, 4], cutoff: 2400., pluck_oct: 2, drone: 0.25, beat: 0.85 },
            // A minor pentatonic with a dorian sixth: the home layers
            Mood::Loam => MoodDef { root: 45, scale: &[0, 2, 3, 7, 9], prog: &[0, 2, 4, 1], cutoff: 1500., pluck_oct: 2, drone: 0.4, beat: 0.95 },
            // E minor pentatonic, lower and darker
            Mood::Deep => MoodDef { root: 40, scale: &[0, 3, 5, 7, 10], prog: &[0, 3, 2, 4], cutoff: 900., pluck_oct: 2, drone: 0.7, beat: 1.1 },
            // C# with a flat second: slow and smouldering near lava
            Mood::Ember => MoodDef { root: 37, scale: &[0, 1, 5, 7, 8], prog: &[0, 3, 0, 4], cutoff: 600., pluck_oct: 1, drone: 1.0, beat: 1.3 },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Texture { PadsPlucks, Kalimba, PadsBells, DroneSparse }
#[derive(Clone, Copy, PartialEq, Debug)] enum State { Rest { until: f64 }, Play { until: f64 } }

pub struct Composer {
    rng: u32, state: State, pub mood: Mood, pub texture: Texture,
    next_chord: f64, chord: usize, next_pluck: f64, next_bell: f64, deg: i32, pub played: u32,
}

/// Equal-tempered frequency of a MIDI note.
pub fn hz(midi: i32) -> f32 { 440. * 2f32.powf((midi - 69) as f32 / 12.) }

impl Composer {
    /// `start` = audio clock now. The first piece begins after a short quiet so the game opens calm.
    pub fn new(seed: u32, start: f64) -> Self {
        Composer { rng: seed | 1, state: State::Rest { until: start + 12. }, mood: Mood::Loam, texture: Texture::PadsPlucks, next_chord: 0., chord: 0, next_pluck: 0., next_bell: 0., deg: 0, played: 0 }
    }
    fn r(&mut self) -> f32 { let mut s = self.rng; s ^= s << 13; s ^= s >> 17; s ^= s << 5; self.rng = s; (s >> 8) as f32 / (1u32 << 24) as f32 }
    fn range(&mut self, a: f64, b: f64) -> f64 { a + (b - a) * self.r() as f64 }
    pub fn playing(&self) -> bool { matches!(self.state, State::Play { .. }) }

    /// Write every note that starts before `horizon` into `out`. `mood` is where the mole is now.
    /// Call often with a horizon a second or two ahead of the audio clock.
    pub fn schedule(&mut self, now: f64, horizon: f64, mood: Mood, out: &mut Vec<Note>) {
        // safety: if the clock jumped (tab asleep), don't burst a backlog of notes
        if let State::Rest { until } | State::Play { until } = self.state { if until < now - 30. { self.state = State::Rest { until: now + 4. }; } }
        loop {
            match self.state {
                State::Rest { until } => {
                    if until > horizon { return; }
                    // begin a piece: mood from where we are, a texture by chance
                    self.mood = mood; let t = until;
                    self.texture = match (self.r() * 4.) as u32 { 0 => Texture::PadsPlucks, 1 => Texture::Kalimba, 2 => Texture::PadsBells, _ => if mood == Mood::Seep { Texture::PadsPlucks } else { Texture::DroneSparse } };
                    let len = self.range(70., 130.);
                    self.state = State::Play { until: t + len };
                    self.next_chord = t; self.chord = 0; self.next_pluck = t + self.range(3., 7.); self.next_bell = t + self.range(8., 20.); self.deg = 5 + (self.r() * 3.) as i32;
                    let d = self.mood.def();
                    if self.texture == Texture::DroneSparse || d.drone > 0.6 { out.push(Note { t, voice: Voice::Drone, hz: hz(d.root - 12), dur: len as f32 + 4., vel: 0.5 * d.drone }); }
                }
                State::Play { until } => {
                    let d = self.mood.def();
                    // the place changed a lot (e.g. down to lava): wind this piece down early for a fresh one
                    if mood != self.mood && until > self.next_chord + 6. { self.state = State::Play { until: self.next_chord + 6. }; continue; }
                    let t = self.next_chord.min(self.next_pluck).min(self.next_bell);
                    if t >= until { self.state = State::Rest { until: until + self.range(40., 95.) }; self.played += 1; continue; }
                    if t > horizon { return; }
                    if t == self.next_chord {
                        let len = self.range(12., 18.); let deg = d.prog[self.chord % d.prog.len()]; self.chord += 1;
                        if matches!(self.texture, Texture::PadsPlucks | Texture::PadsBells) {
                            // root, a scale third up and a fifth-ish: three soft voices that overlap the next chord
                            for (k, step) in [0, 2, 4].iter().enumerate() {
                                let m = d.root + 12 + note_at(d.scale, deg + step);
                                out.push(Note { t, voice: Voice::Pad, hz: hz(m), dur: len as f32 + 3., vel: [0.5, 0.36, 0.3][k] });
                            }
                        }
                        self.next_chord = t + len;
                    } else if t == self.next_pluck {
                        let dens = match self.texture { Texture::Kalimba => 1.0, Texture::PadsPlucks => 0.8, Texture::PadsBells => 0.35, Texture::DroneSparse => 0.25 };
                        if self.r() < dens {
                            // gentle walk: mostly steps, sometimes a leap, kept in a comfortable range
                            let step = [-2, -1, -1, 0, 1, 1, 2, 3][(self.r() * 8.) as usize % 8]; self.deg = (self.deg + step).clamp(2, 11);
                            let m = d.root + 12 * d.pluck_oct + note_at(d.scale, self.deg);
                            out.push(Note { t, voice: Voice::Pluck, hz: hz(m), dur: 2.2, vel: 0.45 + self.r() * 0.3 });
                            // now and then a soft answer a fifth or octave below
                            if self.r() < 0.15 { out.push(Note { t: t + d.beat * 2., voice: Voice::Pluck, hz: hz(m - 12), dur: 2.5, vel: 0.3 }); }
                        }
                        // space: mostly long gaps; a phrase sometimes rests for several beats
                        let beats = [1., 2., 2., 3., 4., 4., 6., 8.][(self.r() * 8.) as usize % 8] * if self.r() < 0.12 { 3. } else { 1. };
                        self.next_pluck = t + d.beat * beats;
                    } else {
                        if self.texture == Texture::PadsBells || self.r() < 0.25 {
                            let m = d.root + 36 + note_at(d.scale, (self.r() * 5.) as i32);
                            out.push(Note { t, voice: Voice::Bell, hz: hz(m), dur: 5., vel: 0.25 + self.r() * 0.15 });
                        }
                        self.next_bell = t + self.range(14., 30.);
                    }
                }
            }
        }
    }
}

/// Semitones above the root for scale degree `deg` (wraps into higher octaves).
pub fn note_at(scale: &[i32], deg: i32) -> i32 { let n = scale.len() as i32; let o = deg.div_euclid(n); scale[deg.rem_euclid(n) as usize] + 12 * o }

#[cfg(test)]
mod tests {
    use super::*;
    fn run(mood: Mood, secs: f64) -> Vec<Note> { let mut c = Composer::new(7, 0.); let mut v = vec![]; let mut t = 0.; while t < secs { c.schedule(t, t + 1.5, mood, &mut v); t += 0.25; } v }
    #[test] fn starts_quiet_then_plays_sparsely_with_long_rests() {
        let v = run(Mood::Loam, 600.);
        assert!(v.iter().all(|n| n.t >= 12.), "opens with a quiet moment");
        let plucks: Vec<f64> = v.iter().filter(|n| n.voice == Voice::Pluck).map(|n| n.t).collect();
        assert!(!plucks.is_empty());
        // sparse: well under one pluck per second over ten minutes
        assert!((plucks.len() as f64) < 600. * 0.5, "too busy: {} plucks", plucks.len());
        // long gaps of near-silence exist (a rest between pieces)
        let mut ts: Vec<f64> = v.iter().map(|n| n.t).collect(); ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(ts.windows(2).any(|w| w[1] - w[0] > 35.), "no long rest");
        // gentle levels, sensible ranges
        assert!(v.iter().all(|n| n.vel > 0. && n.vel <= 0.8 && n.hz > 30. && n.hz < 2500.));
    }
    #[test] fn depth_sets_the_register() {
        let avg = |m: Mood| { let v = run(m, 900.); let p: Vec<f32> = v.iter().filter(|n| n.voice == Voice::Pluck).map(|n| n.hz).collect(); p.iter().sum::<f32>() / p.len().max(1) as f32 };
        assert!(avg(Mood::Seep) > avg(Mood::Deep) && avg(Mood::Deep) > avg(Mood::Ember), "higher up sounds higher, lava lowest");
        assert_eq!(Mood::pick(-30., 0.), Mood::Seep); assert_eq!(Mood::pick(0., 0.), Mood::Loam); assert_eq!(Mood::pick(40., 0.), Mood::Deep); assert_eq!(Mood::pick(10., 0.2), Mood::Ember);
    }
    #[test] fn notes_are_in_order_and_never_scheduled_twice() {
        let mut c = Composer::new(3, 0.); let mut v = vec![];
        for k in 0..2000 { let t = k as f64 * 0.1; let n0 = v.len(); c.schedule(t, t + 1.5, Mood::Deep, &mut v); for n in &v[n0..] { assert!(n.t >= t - 1e-9, "note in the past"); } }
        let mut seen = std::collections::HashSet::new(); for n in &v { assert!(seen.insert(((n.t * 1000.) as i64, n.voice as u8, (n.hz * 10.) as i32)), "duplicate note"); }
    }
    #[test] fn a_clock_jump_does_not_burst() {
        let mut c = Composer::new(5, 0.); let mut v = vec![]; c.schedule(0., 1.5, Mood::Loam, &mut v); c.schedule(3600., 3601.5, Mood::Loam, &mut v);
        assert!(v.len() < 10, "{} notes after an hour asleep", v.len());
    }
}
