//! Procedural sound (recipes from the v2/ambience design notes): no audio files, oscillators and noise
//! buffers with exponential envelopes. Music: `music::Composer` writes notes a second or two ahead; they're
//! played as soft pads, kalimba-like plucks, bells and drones through a mood lowpass, a dark echo and a
//! long generated reverb, on their own bus with its own volume (Settings > Music).
//!
//! One path for every build: the recipes here (`sfx` for each event, `parts` for each note, the rumble
//! level, the reverb impulse and the noise buffers) say what to play. On the web they become WebAudio
//! nodes; natively `synth::Engine` renders the same graph in software and cpal hands it to the sound card.
//! Both start on the first key or click (browsers need a gesture; native does the same so they match).
use molehill_sim::world::Event;
use crate::music::{Note, Voice};

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Wave { Sine, Triangle, Square }
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Filt { Lowpass, Bandpass }
/// One effect voice, started now: a tone sweeping `f0` to `f1`, or a burst of white or brown noise
/// through a filter sweeping `f0` to `f1` (Q 1). Its gain rises from 0.0001 to `peak` in 10 ms and falls
/// exponentially to 0.0001 at `dur`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sfx { Tone { wave: Wave, f0: f32, f1: f32, dur: f64, peak: f32 }, Noise { brown: bool, filt: Filt, f0: f32, f1: f32, dur: f64, peak: f32 } }
impl Sfx { pub fn dur(&self) -> f64 { match *self { Sfx::Tone { dur, .. } | Sfx::Noise { dur, .. } => dur } } }
/// Where a music oscillator goes: the pad lowpass (pads, drones), the pluck bus (also into the echo), or
/// straight into the reverb (bells).
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Bus { Pad, Pluck, Verb }
/// One music oscillator: starts at `t` on the audio clock; its gain follows `env` ((time after `t`, level)
/// points, from silence, linear or exponential ramps) and it stops 50 ms after the last point.
#[derive(Clone, PartialEq, Debug)] pub struct Osc { pub bus: Bus, pub wave: Wave, pub hz: f32, pub t: f64, pub env: Vec<(f64, f32)>, pub exp: bool }
impl Osc { pub fn end(&self) -> f64 { self.env.last().map(|e| e.0).unwrap_or(1.) } }

/// Effects master at volume 1.
pub const MASTER: f32 = 0.55;
/// Music bus level at slider 100%: deliberately low, it sits under the effects.
pub const MUSIC_GAIN: f32 = 0.32;
/// Time constants (s) of the music volume, the rumble level and the mood cutoff.
pub const MUSIC_TC: f32 = 0.3; pub const RUMBLE_TC: f32 = 0.5; pub const CUTOFF_TC: f32 = 4.;
pub const RUMBLE_LP: f32 = 110.; pub const PAD_CUTOFF: f32 = 1500.; pub const PAD_Q: f32 = 0.4;
pub const PLUCK_GAIN: f32 = 0.8; pub const VERB_WET: f32 = 0.9;
pub const ECHO_DELAY: f32 = 0.62; pub const ECHO_FB: f32 = 0.33; pub const ECHO_LP: f32 = 1600.; pub const ECHO_OUT: f32 = 0.45;
/// The rumble's level for depth (0 at the top, 1 at the deepest of the holt).
pub fn rumble_level(depth: f32) -> f32 { 0.06 + 0.16 * depth.clamp(0., 1.) }
/// Where in its noise buffer a burst started at `t` begins (s), so repeats don't sound the same.
pub fn noise_offset(t: f64) -> f64 { (t * 1.7) % 1.0 }

/// Every event, for tests.
#[cfg(test)]
pub const EVENTS: [Event; 18] = [Event::Explode, Event::Crumble, Event::Crack, Event::Dig, Event::Place, Event::Hurt, Event::Death, Event::Chest, Event::Quest, Event::Craft, Event::Discover, Event::Eat, Event::Warning, Event::Quake, Event::Click, Event::Harvest, Event::Gasp, Event::Lore];
/// The event's slot and the least time (s) between two of it.
pub fn cooldown(e: Event) -> (usize, f64) {
    match e { Event::Explode => (0, 0.3), Event::Crumble => (1, 1.2), Event::Crack => (2, 0.8), Event::Dig => (3, 0.07), Event::Place => (4, 0.1), Event::Hurt => (5, 0.3), Event::Death => (6, 1.), Event::Chest => (7, 0.5), Event::Quest => (8, 0.5), Event::Craft => (9, 0.3), Event::Discover => (10, 0.5), Event::Eat => (11, 0.3), Event::Warning => (12, 1.), Event::Quake => (13, 2.), Event::Click => (14, 0.05), Event::Harvest => (15, 0.3), Event::Gasp => (16, 1.5), Event::Lore => (17, 1.) }
}
/// When each event last sounded, on the audio clock. Starts long ago: the clock starts at 0 when the sound
/// opens, so a 0 here would swallow the first of each event in its gap (the click of the very key that opened it).
pub struct Gaps([f64; 18]);
impl Gaps {
    pub fn new() -> Self { Gaps([f64::NEG_INFINITY; 18]) }
    /// Whether `e` may sound at `now`, and if so note it.
    pub fn pass(&mut self, e: Event, now: f64) -> bool { let (k, gap) = cooldown(e); if now - self.0[k] < gap { return false; } self.0[k] = now; true }
}
/// What an event sounds like, `gain` = spatial attenuation from the listener (camera), `now` = the audio clock.
pub fn sfx(e: Event, gain: f32, now: f64) -> Vec<Sfx> {
    use Filt as F; use Wave as O;
    let tone = |wave, f0, f1, dur, peak: f32, gain: f32| Sfx::Tone { wave, f0, f1, dur, peak: peak * gain };
    let noise = |brown, filt, f0, f1, dur, peak: f32, gain: f32| Sfx::Noise { brown, filt, f0, f1, dur, peak: peak * gain };
    match e {
        Event::Explode => vec![noise(false, F::Lowpass, 2400., 60., 1.6, 0.9, gain), tone(O::Sine, 90., 28., 0.9, 0.8, gain), noise(true, F::Lowpass, 300., 80., 2.5, 0.7, gain)],
        Event::Crumble | Event::Quake => { let mut v = vec![noise(true, F::Lowpass, 500., 90., 2.8, 0.8, gain)];
            for i in 0..4 { v.push(noise(false, F::Bandpass, 900. + i as f32 * 300., 400., 0.08 + i as f64 * 0.2, 0.3, gain)); } v }
        Event::Crack => vec![noise(false, F::Bandpass, 2200., 900., 0.12, 0.25, gain)],
        Event::Dig => vec![noise(false, F::Bandpass, 900. + (now * 997.0 % 900.) as f32, 700., 0.07, 0.18, gain)],
        Event::Place => vec![tone(O::Triangle, 220., 180., 0.08, 0.3, gain), tone(O::Triangle, 260., 200., 0.1, 0.2, gain)],
        Event::Hurt => vec![tone(O::Square, 900., 600., 0.09, 0.12, gain)],
        Event::Death => vec![tone(O::Triangle, 880., 180., 0.7, 0.3, gain)],
        Event::Chest | Event::Discover => vec![tone(O::Sine, 660., 990., 0.18, 0.25, gain), tone(O::Sine, 990., 1320., 0.3, 0.18, gain)],
        Event::Quest => vec![tone(O::Triangle, 523., 523., 0.15, 0.3, 1.), tone(O::Triangle, 784., 1046., 0.4, 0.25, 1.)],
        Event::Craft => vec![tone(O::Triangle, 330., 300., 0.12, 0.25, gain)],
        Event::Eat => vec![noise(false, F::Bandpass, 1400., 700., 0.1, 0.2, gain)],
        Event::Warning => vec![tone(O::Square, 440., 440., 0.25, 0.12, 1.), tone(O::Square, 330., 330., 0.5, 0.1, 1.)],
        Event::Click => vec![tone(O::Square, 1800., 1200., 0.03, 0.08, 1.)],
        Event::Harvest => vec![tone(O::Sine, 440., 520., 0.1, 0.2, gain), noise(false, F::Bandpass, 2000., 900., 0.06, 0.12, gain)],
        // a clue read: a soft, low, hollow pair of notes and a breath of air
        Event::Lore => vec![tone(O::Sine, 196., 196., 1.2, 0.16, 1.), tone(O::Sine, 294., 294., 1.6, 0.1, 1.), noise(false, F::Lowpass, 600., 200., 1.4, 0.06, 1.)],
        Event::Gasp => vec![noise(false, F::Bandpass, 700., 300., 0.5, 0.25, gain), tone(O::Sine, 300., 220., 0.4, 0.12, gain)],
    }
}
/// The oscillators that play a note.
pub fn parts(n: &Note) -> Vec<Osc> {
    let (t, f, d, v) = (n.t, n.hz, n.dur as f64, n.vel);
    let o = |bus, wave, hz, env: Vec<(f64, f32)>, exp| Osc { bus, wave, hz, t, env, exp };
    match n.voice {
        // soft pad: triangle + slightly detuned sine, slow swell and long release
        Voice::Pad => { let (a, r) = (3.5f64.min(d / 3.), 5f64.min(d / 2.)); let env = vec![(a, v * 0.07), (d - r, v * 0.07), (d, 0.)];
            vec![o(Bus::Pad, Wave::Triangle, f, env.clone(), false), o(Bus::Pad, Wave::Sine, f * 1.004, env, false)] }
        // kalimba / marimba: a sine with a quick tick of 4th harmonic and a softer 2nd
        Voice::Pluck => vec![o(Bus::Pluck, Wave::Sine, f, vec![(0.006, v * 0.2), (d, 0.0001)], true),
            o(Bus::Pluck, Wave::Sine, f * 4., vec![(0.004, v * 0.035), (0.22, 0.0001)], true),
            o(Bus::Pluck, Wave::Sine, f * 2., vec![(0.006, v * 0.04), (0.7, 0.0001)], true)],
        // bell: inharmonic partial, mostly heard through the reverb
        Voice::Bell => vec![o(Bus::Verb, Wave::Sine, f, vec![(0.01, v * 0.08), (d, 0.0001)], true),
            o(Bus::Verb, Wave::Sine, f * 2.76, vec![(0.01, v * 0.025), (d * 0.4, 0.0001)], true)],
        // drone: root and fifth, very slow in and out
        Voice::Drone => { let env = vec![(8f64.min(d / 3.), v * 0.09), (d - 8f64.min(d / 3.), v * 0.09), (d, 0.)];
            let fifth = env.iter().map(|&(a, b)| (a, b * 0.4)).collect();
            vec![o(Bus::Pad, Wave::Sine, f, env, false), o(Bus::Pad, Wave::Sine, f * 1.5, fifth, false)] }
    }
}
/// The cave reverb's impulse: 3.5 s of decaying, darkened stereo noise.
pub fn reverb_ir(sr: f32) -> Vec<Vec<f32>> {
    let n = (sr * 3.5) as usize; let mut seed = 0x9e3779b9u32;
    (0..2).map(|_| { let mut d = vec![0f32; n]; let mut y = 0f32;
        for (i, v) in d.iter_mut().enumerate() { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; let r = (seed as f32 / u32::MAX as f32) * 2. - 1.;
            y += 0.3 * (r - y); *v = y * (1. - i as f32 / n as f32).powf(2.6); }
        d }).collect()
}
/// A noise buffer: `secs` of white noise, or brown (integrated) noise.
pub fn noise_buf(sr: f32, secs: f32, brown: bool) -> Vec<f32> {
    let mut d = vec![0f32; (sr * secs) as usize]; let mut s = 0x1234567u32; let mut acc = 0f32;
    for v in d.iter_mut() { s ^= s << 13; s ^= s >> 17; s ^= s << 5; let r = (s as f32 / u32::MAX as f32) * 2. - 1.; if brown { acc = (acc + 0.02 * r) / 1.02; *v = acc * 3.5; } else { *v = r; } }
    d
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use web_sys::{AudioBuffer, AudioContext, AudioContextState, AudioNode, BiquadFilterNode, BiquadFilterType, GainNode, OscillatorType};
    use crate::music::{Composer, Mood};
    /// The music graph: voices feed `pad_f` (pads, drones) or `pluck` (plucks, bells); both go dry to `bus`
    /// and into the reverb; plucks also into the echo.
    struct Mus { comp: Composer, bus: GainNode, pad_f: BiquadFilterNode, pluck: GainNode, verb_in: GainNode, mood: Option<Mood>, buf: Vec<Note> }
    pub struct Audio { ctx: Option<AudioContext>, white: Option<AudioBuffer>, brown: Option<AudioBuffer>, last: Gaps, master: Option<GainNode>, rumble: Option<GainNode>, vol: f32, mvol: f32, mus: Option<Mus> }
    fn wave(w: Wave) -> OscillatorType { match w { Wave::Sine => OscillatorType::Sine, Wave::Triangle => OscillatorType::Triangle, Wave::Square => OscillatorType::Square } }
    fn filt(f: Filt) -> BiquadFilterType { match f { Filt::Lowpass => BiquadFilterType::Lowpass, Filt::Bandpass => BiquadFilterType::Bandpass } }
    impl Audio {
        pub fn new() -> Self { Audio { ctx: None, white: None, brown: None, last: Gaps::new(), master: None, rumble: None, vol: 0.8, mvol: 0.35, mus: None } }
        pub fn set_volume(&mut self, v: f32) { if v == self.vol { return; } self.vol = v; if let Some(m) = &self.master { m.gain().set_value(MASTER * v); } }
        pub fn set_music(&mut self, v: f32) {
            if v == self.mvol { return; } self.mvol = v;
            if let (Some(c), Some(m)) = (&self.ctx, &self.mus) { let _ = m.bus.gain().set_target_at_time(MUSIC_GAIN * v, c.current_time(), MUSIC_TC as f64); }
        }
        fn music_graph(ctx: &AudioContext, mvol: f32) -> Option<Mus> {
            let sr = ctx.sample_rate();
            let bus = ctx.create_gain().ok()?; bus.gain().set_value(MUSIC_GAIN * mvol); bus.connect_with_audio_node(&ctx.destination()).ok()?;
            let d = reverb_ir(sr); let ir = ctx.create_buffer(2, d[0].len() as u32, sr).ok()?;
            for (ch, c) in d.iter().enumerate() { ir.copy_to_channel(c, ch as i32).ok()?; }
            let verb = ctx.create_convolver().ok()?; verb.set_buffer(Some(&ir));
            let verb_in = ctx.create_gain().ok()?; verb_in.gain().set_value(1.0); let wet = ctx.create_gain().ok()?; wet.gain().set_value(VERB_WET);
            verb_in.connect_with_audio_node(&verb).ok()?; verb.connect_with_audio_node(&wet).ok()?; wet.connect_with_audio_node(&bus).ok()?;
            // echo for plucks: 0.62 s, darkening feedback
            let dl = ctx.create_delay_with_max_delay_time(2.).ok()?; dl.delay_time().set_value(ECHO_DELAY);
            let fb = ctx.create_gain().ok()?; fb.gain().set_value(ECHO_FB); let lp = ctx.create_biquad_filter().ok()?; lp.set_type(BiquadFilterType::Lowpass); lp.frequency().set_value(ECHO_LP);
            dl.connect_with_audio_node(&lp).ok()?; lp.connect_with_audio_node(&fb).ok()?; fb.connect_with_audio_node(&dl).ok()?;
            let eo = ctx.create_gain().ok()?; eo.gain().set_value(ECHO_OUT); lp.connect_with_audio_node(&eo).ok()?; eo.connect_with_audio_node(&bus).ok()?; eo.connect_with_audio_node(&verb_in).ok()?;
            let pad_f = ctx.create_biquad_filter().ok()?; pad_f.set_type(BiquadFilterType::Lowpass); pad_f.frequency().set_value(PAD_CUTOFF); pad_f.q().set_value(PAD_Q);
            pad_f.connect_with_audio_node(&bus).ok()?; pad_f.connect_with_audio_node(&verb_in).ok()?;
            let pluck = ctx.create_gain().ok()?; pluck.gain().set_value(PLUCK_GAIN); pluck.connect_with_audio_node(&bus).ok()?; pluck.connect_with_audio_node(&verb_in).ok()?; pluck.connect_with_audio_node(&dl).ok()?;
            let seed = (js_sys::Math::random() * u32::MAX as f64) as u32;
            Some(Mus { comp: Composer::new(seed, ctx.current_time()), bus, pad_f, pluck, verb_in, mood: None, buf: vec![] })
        }
        /// One oscillator with a gain envelope: `env` = (time, level) points after `t` (linear ramps), ending silent.
        fn osc(ctx: &AudioContext, m: &Mus, p: &Osc) {
            let (Ok(o), Ok(g)) = (ctx.create_oscillator(), ctx.create_gain()) else { return };
            let to: &AudioNode = match p.bus { Bus::Pad => &m.pad_f, Bus::Pluck => &m.pluck, Bus::Verb => &m.verb_in };
            let t = p.t; o.set_type(wave(p.wave)); o.frequency().set_value(p.hz);
            let _ = g.gain().set_value_at_time(if p.exp { 0.0001 } else { 0. }, t);
            for &(dt, v) in &p.env { if p.exp { let _ = g.gain().exponential_ramp_to_value_at_time(v.max(0.0001), t + dt); } else { let _ = g.gain().linear_ramp_to_value_at_time(v, t + dt); } }
            let _ = o.connect_with_audio_node(&g); let _ = g.connect_with_audio_node(to); let _ = o.start_with_when(t); let _ = o.stop_with_when(t + p.end() + 0.05);
        }
        /// Call every frame: keeps the composer a little ahead of the audio clock.
        pub fn music(&mut self, depth_m: f32, heat: f32) {
            let Some(ctx) = &self.ctx else { return }; let Some(m) = self.mus.as_mut() else { return };
            if self.mvol <= 0. || ctx.state() != AudioContextState::Running { return; }
            let now = ctx.current_time(); let mood = Mood::pick(depth_m, heat);
            let mut buf = std::mem::take(&mut m.buf); buf.clear();
            m.comp.schedule(now + 0.05, now + 1.5, mood, &mut buf);
            if m.mood != Some(m.comp.mood) { m.mood = Some(m.comp.mood); let _ = m.pad_f.frequency().set_target_at_time(m.comp.mood.def().cutoff, now, CUTOFF_TC as f64); }
            for n in &buf { for p in parts(n) { Self::osc(ctx, m, &p); } }
            m.buf = buf;
        }
        /// Browsers only allow audio after a user gesture: call on first input.
        pub fn unlock(&mut self) {
            if self.ctx.is_some() { if let Some(c) = &self.ctx { let _ = c.resume(); } return; }
            let Ok(ctx) = AudioContext::new() else { return };
            let sr = ctx.sample_rate();
            let mk = |secs: f32, brown: bool| -> Option<AudioBuffer> {
                let d = noise_buf(sr, secs, brown); let b = ctx.create_buffer(1, d.len() as u32, sr).ok()?; b.copy_to_channel(&d, 0).ok()?; Some(b)
            };
            self.white = mk(2., false); self.brown = mk(4., true);
            let m = ctx.create_gain().ok(); if let Some(m) = &m { m.gain().set_value(MASTER * self.vol); let _ = m.connect_with_audio_node(&ctx.destination()); }
            self.master = m; self.mus = Self::music_graph(&ctx, self.mvol); self.ctx = Some(ctx);
            // ambient rumble loop
            if let (Some(ctx), Some(b), Some(m)) = (&self.ctx, &self.brown, &self.master) {
                if let (Ok(src), Ok(f), Ok(g)) = (ctx.create_buffer_source(), ctx.create_biquad_filter(), ctx.create_gain()) {
                    src.set_buffer(Some(b)); src.set_loop(true); f.set_type(BiquadFilterType::Lowpass); f.frequency().set_value(RUMBLE_LP); g.gain().set_value(0.0);
                    let _ = src.connect_with_audio_node(&f); let _ = f.connect_with_audio_node(&g); let _ = g.connect_with_audio_node(m); let _ = src.start(); self.rumble = Some(g);
                }
            }
        }
        pub fn ambient(&mut self, depth: f32) { if let (Some(c), Some(g)) = (&self.ctx, &self.rumble) { let _ = g.gain().set_target_at_time(rumble_level(depth), c.current_time(), RUMBLE_TC as f64); } }
        fn sfx(&self, s: &Sfx) {
            let (Some(ctx), Some(m)) = (&self.ctx, &self.master) else { return }; let t = ctx.current_time();
            match *s {
                Sfx::Noise { brown, filt: ft, f0, f1, dur, peak } => {
                    let Some(b) = (if brown { &self.brown } else { &self.white }) else { return };
                    let (Ok(src), Ok(f), Ok(g)) = (ctx.create_buffer_source(), ctx.create_biquad_filter(), ctx.create_gain()) else { return };
                    src.set_buffer(Some(b)); f.set_type(filt(ft)); f.frequency().set_value(f0); let _ = f.frequency().exponential_ramp_to_value_at_time(f1.max(20.), t + dur);
                    g.gain().set_value(0.0001); let _ = g.gain().exponential_ramp_to_value_at_time(peak, t + 0.01); let _ = g.gain().exponential_ramp_to_value_at_time(0.0001, t + dur);
                    let _ = src.connect_with_audio_node(&f); let _ = f.connect_with_audio_node(&g); let _ = g.connect_with_audio_node(m);
                    let _ = src.start_with_when_and_grain_offset(t, noise_offset(t)); let _ = src.stop_with_when(t + dur + 0.05);
                }
                Sfx::Tone { wave: w, f0, f1, dur, peak } => {
                    let (Ok(o), Ok(g)) = (ctx.create_oscillator(), ctx.create_gain()) else { return };
                    o.set_type(wave(w)); o.frequency().set_value(f0); let _ = o.frequency().exponential_ramp_to_value_at_time(f1, t + dur);
                    g.gain().set_value(0.0001); let _ = g.gain().exponential_ramp_to_value_at_time(peak, t + 0.01); let _ = g.gain().exponential_ramp_to_value_at_time(0.0001, t + dur);
                    let _ = o.connect_with_audio_node(&g); let _ = g.connect_with_audio_node(m); let _ = o.start(); let _ = o.stop_with_when(t + dur + 0.05);
                }
            }
        }
        /// `gain` = spatial attenuation from the listener (camera).
        pub fn play(&mut self, e: Event, gain: f32) {
            let Some(ctx) = &self.ctx else { return }; let now = ctx.current_time();
            if !self.last.pass(e, now) { return; }
            for s in sfx(e, gain, now) { self.sfx(&s); }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    use crate::music::{Composer, Mood};
    use crate::synth::{Cmd, Engine};
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc, atomic::{AtomicU64, Ordering}, mpsc};
    /// The open sound card: the stream (kept alive), the way to the audio thread, and its clock.
    struct Out { _stream: cpal::Stream, tx: mpsc::Sender<Cmd>, clock: Arc<AtomicU64>, sr: f64 }
    struct Mus { comp: Composer, mood: Option<Mood>, buf: Vec<Note> }
    pub struct Audio { out: Option<Out>, tried: bool, last: Gaps, vol: f32, mvol: f32, mus: Option<Mus> }
    impl Audio {
        pub fn new() -> Self { Audio { out: None, tried: false, last: Gaps::new(), vol: 0.8, mvol: 0.35, mus: None } }
        fn send(&self, c: Cmd) { if let Some(o) = &self.out { let _ = o.tx.send(c); } }
        fn now(&self) -> f64 { self.out.as_ref().map_or(0., |o| o.clock.load(Ordering::Relaxed) as f64 / o.sr) }
        #[cfg(test)] pub fn is_open(&self) -> bool { self.out.is_some() }
        #[cfg(test)] pub fn clock_secs(&self) -> f64 { self.now() }
        pub fn set_volume(&mut self, v: f32) { if v == self.vol { return; } self.vol = v; self.send(Cmd::Master(MASTER * v)); }
        pub fn set_music(&mut self, v: f32) { if v == self.mvol { return; } self.mvol = v; self.send(Cmd::Music(MUSIC_GAIN * v)); }
        /// Open the default output on the first input, as the web build does (once: no device, no sound).
        pub fn unlock(&mut self) {
            if self.tried { return; } self.tried = true;
            match open(self.vol, self.mvol) {
                Ok(o) => {
                    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |d| d.subsec_nanos() ^ d.as_secs() as u32);
                    self.mus = Some(Mus { comp: Composer::new(seed, 0.), mood: None, buf: vec![] }); self.out = Some(o);
                }
                Err(e) => bevy::log::warn!("no sound: {e}"),
            }
        }
        pub fn ambient(&mut self, depth: f32) { self.send(Cmd::Rumble(rumble_level(depth))); }
        /// Call every frame: keeps the composer a little ahead of the audio clock.
        pub fn music(&mut self, depth_m: f32, heat: f32) {
            if self.out.is_none() || self.mvol <= 0. { return; }
            let now = self.now(); let Some(m) = self.mus.as_mut() else { return };
            let mood = Mood::pick(depth_m, heat);
            let mut buf = std::mem::take(&mut m.buf); buf.clear();
            m.comp.schedule(now + 0.05, now + 1.5, mood, &mut buf);
            if let Some(o) = &self.out {
                if m.mood != Some(m.comp.mood) { m.mood = Some(m.comp.mood); let _ = o.tx.send(Cmd::Cutoff(m.comp.mood.def().cutoff)); }
                for n in &buf { for p in parts(n) { let _ = o.tx.send(Cmd::Osc(p)); } }
            }
            m.buf = buf;
        }
        /// `gain` = spatial attenuation from the listener (camera).
        pub fn play(&mut self, e: Event, gain: f32) {
            if self.out.is_none() { return; } let now = self.now();
            if !self.last.pass(e, now) { return; }
            for s in sfx(e, gain, now) { self.send(Cmd::Sfx(s)); }
        }
    }
    /// The audio thread's side: the engine, the commands coming in, and the clock it reports back.
    struct Feed { eng: Engine, rx: mpsc::Receiver<Cmd>, clock: Arc<AtomicU64>, tmp: Vec<f32>, ch: usize }
    impl Feed {
        fn fill(&mut self, n: usize) -> &[f32] {
            while let Ok(c) = self.rx.try_recv() { self.eng.cmd(c); }
            self.tmp.resize(n, 0.); self.eng.render(&mut self.tmp, self.ch); self.clock.store(self.eng.frames(), Ordering::Relaxed); &self.tmp
        }
    }
    /// The default output device, rendering `Engine` in its callback.
    fn open(vol: f32, mvol: f32) -> Result<Out, String> {
        let dev = cpal::default_host().default_output_device().ok_or("no output device")?;
        let sup = dev.default_output_config().map_err(|e| e.to_string())?;
        let fmt = sup.sample_format(); let cfg: cpal::StreamConfig = sup.into();
        let (sr, ch) = (cfg.sample_rate, cfg.channels as usize);
        let (tx, rx) = mpsc::channel::<Cmd>(); let clock = Arc::new(AtomicU64::new(0));
        let feed = Feed { eng: Engine::new(sr, vol, mvol), rx, clock: clock.clone(), tmp: vec![], ch };
        let err = |e| bevy::log::warn!("sound: {e}");
        macro_rules! build { ($t:ty) => { { let mut f = feed; dev.build_output_stream(&cfg, move |d: &mut [$t], _: &cpal::OutputCallbackInfo| { let s = f.fill(d.len()); for (o, &x) in d.iter_mut().zip(s) { *o = <$t as cpal::FromSample<f32>>::from_sample_(x); } }, err, None) } } }
        let stream = match fmt {
            cpal::SampleFormat::F32 => build!(f32), cpal::SampleFormat::F64 => build!(f64),
            cpal::SampleFormat::I16 => build!(i16), cpal::SampleFormat::I32 => build!(i32),
            cpal::SampleFormat::U16 => build!(u16), cpal::SampleFormat::U8 => build!(u8), cpal::SampleFormat::I8 => build!(i8),
            f => return Err(format!("unsupported sample format {f}")),
        }.map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Out { _stream: stream, tx, clock, sr: sr as f64 })
    }
}
pub use imp::Audio;

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn every_event_has_its_own_slot_and_a_recipe() {
        let mut seen = [false; 18];
        for e in EVENTS { let (k, gap) = cooldown(e); assert!(!seen[k], "{e:?} shares slot {k}"); seen[k] = true; assert!(gap > 0.);
            let s = sfx(e, 0.5, 1.); assert!(!s.is_empty(), "{e:?}"); assert!(s.iter().all(|x| x.dur() > 0.)); }
    }
    /// The clock starts at 0 when the sound opens: the first of every event sounds then, even inside its gap,
    /// and a second one inside the gap doesn't.
    #[test] fn the_first_of_each_event_sounds_at_once() {
        let mut g = Gaps::new();
        for e in EVENTS { assert!(g.pass(e, 0.), "{e:?} swallowed at the start"); let (_, gap) = cooldown(e);
            assert!(!g.pass(e, gap * 0.5), "{e:?} twice in its gap"); assert!(g.pass(e, gap), "{e:?} after its gap"); }
    }
    /// On a machine with speakers: `cargo test --release -p molehill opens_the_sound_card -- --ignored`
    /// opens the default output, plays a blast, a chord and some rumble, and checks the audio clock runs.
    #[cfg(not(target_arch = "wasm32"))]
    #[test] #[ignore] fn opens_the_sound_card() {
        let mut a = Audio::new(); a.unlock(); assert!(a.is_open(), "no output device");
        a.ambient(1.); a.play(Event::Explode, 1.); a.play(Event::Quest, 1.);
        let t0 = std::time::Instant::now(); std::thread::sleep(std::time::Duration::from_millis(2500));
        let ran = a.clock_secs(); assert!((ran - t0.elapsed().as_secs_f64()).abs() < 0.5, "audio clock {ran:.2} s after {:.2} s", t0.elapsed().as_secs_f64());
    }
    #[test] fn notes_become_oscillators_on_the_right_bus() {
        let n = |voice| Note { t: 2., voice, hz: 220., dur: 12., vel: 0.5 };
        assert!(parts(&n(Voice::Pad)).iter().all(|o| o.bus == Bus::Pad && !o.exp && o.t == 2.));
        assert!(parts(&n(Voice::Pluck)).iter().all(|o| o.bus == Bus::Pluck && o.exp));
        assert!(parts(&n(Voice::Bell)).iter().all(|o| o.bus == Bus::Verb));
        assert_eq!(parts(&n(Voice::Drone)).len(), 2);
        for v in [Voice::Pad, Voice::Pluck, Voice::Bell, Voice::Drone] { for o in parts(&n(v)) { assert!(o.env.windows(2).all(|w| w[0].0 <= w[1].0), "{v:?} envelope out of order"); } }
    }
}
