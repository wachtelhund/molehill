//! Procedural sound (recipes from the v2/ambience design notes): no audio files, WebAudio
//! oscillators + noise buffers with exponential envelopes. Web only; native builds are silent.
//! Music: `music::Composer` writes notes a second or two ahead; they're played here as soft pads,
//! kalimba-like plucks, bells and drones through a mood lowpass, a dark echo and a long generated reverb,
//! on their own bus with its own volume (Settings > Music).
use molehill_sim::world::Event;

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::Event;
    use web_sys::*;
    use crate::music::{Composer, Mood, Note, Voice};
    /// The music graph: voices feed `pad_f` (pads, drones) or `pluck` (plucks, bells); both go dry to `bus`
    /// and into the reverb; plucks also into the echo.
    struct Mus { comp: Composer, bus: GainNode, pad_f: BiquadFilterNode, pluck: GainNode, verb_in: GainNode, mood: Option<Mood>, buf: Vec<Note> }
    pub struct Audio { ctx: Option<AudioContext>, white: Option<AudioBuffer>, brown: Option<AudioBuffer>, last: [f64; 18], master: Option<GainNode>, rumble: Option<GainNode>, vol: f32, mvol: f32, mus: Option<Mus> }
    /// Music bus level at slider 100%: deliberately low, it sits under the effects.
    const MUSIC_GAIN: f32 = 0.32;
    impl Audio {
        pub fn new() -> Self { Audio { ctx: None, white: None, brown: None, last: [0.; 18], master: None, rumble: None, vol: 0.8, mvol: 0.35, mus: None } }
        pub fn set_volume(&mut self, v: f32) { if v == self.vol { return; } self.vol = v; if let Some(m) = &self.master { m.gain().set_value(0.55 * v); } }
        pub fn set_music(&mut self, v: f32) {
            if v == self.mvol { return; } self.mvol = v;
            if let (Some(c), Some(m)) = (&self.ctx, &self.mus) { let _ = m.bus.gain().set_target_at_time(MUSIC_GAIN * v, c.current_time(), 0.3); }
        }
        fn music_graph(ctx: &AudioContext, mvol: f32) -> Option<Mus> {
            let sr = ctx.sample_rate();
            let bus = ctx.create_gain().ok()?; bus.gain().set_value(MUSIC_GAIN * mvol); bus.connect_with_audio_node(&ctx.destination()).ok()?;
            // reverb: 3.5 s of decaying, darkened stereo noise (a big soft cave)
            let n = (sr * 3.5) as u32; let ir = ctx.create_buffer(2, n, sr).ok()?; let mut seed = 0x9e3779b9u32;
            for ch in 0..2 { let mut d = vec![0f32; n as usize]; let mut y = 0f32;
                for (i, v) in d.iter_mut().enumerate() { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; let r = (seed as f32 / u32::MAX as f32) * 2. - 1.;
                    y += 0.3 * (r - y); *v = y * (1. - i as f32 / n as f32).powf(2.6); }
                ir.copy_to_channel(&d, ch).ok()?; }
            let verb = ctx.create_convolver().ok()?; verb.set_buffer(Some(&ir));
            let verb_in = ctx.create_gain().ok()?; verb_in.gain().set_value(1.0); let wet = ctx.create_gain().ok()?; wet.gain().set_value(0.9);
            verb_in.connect_with_audio_node(&verb).ok()?; verb.connect_with_audio_node(&wet).ok()?; wet.connect_with_audio_node(&bus).ok()?;
            // echo for plucks: 0.62 s, darkening feedback
            let dl = ctx.create_delay_with_max_delay_time(2.).ok()?; dl.delay_time().set_value(0.62);
            let fb = ctx.create_gain().ok()?; fb.gain().set_value(0.33); let lp = ctx.create_biquad_filter().ok()?; lp.set_type(BiquadFilterType::Lowpass); lp.frequency().set_value(1600.);
            dl.connect_with_audio_node(&lp).ok()?; lp.connect_with_audio_node(&fb).ok()?; fb.connect_with_audio_node(&dl).ok()?;
            let eo = ctx.create_gain().ok()?; eo.gain().set_value(0.45); lp.connect_with_audio_node(&eo).ok()?; eo.connect_with_audio_node(&bus).ok()?; eo.connect_with_audio_node(&verb_in).ok()?;
            let pad_f = ctx.create_biquad_filter().ok()?; pad_f.set_type(BiquadFilterType::Lowpass); pad_f.frequency().set_value(1500.); pad_f.q().set_value(0.4);
            pad_f.connect_with_audio_node(&bus).ok()?; pad_f.connect_with_audio_node(&verb_in).ok()?;
            let pluck = ctx.create_gain().ok()?; pluck.gain().set_value(0.8); pluck.connect_with_audio_node(&bus).ok()?; pluck.connect_with_audio_node(&verb_in).ok()?; pluck.connect_with_audio_node(&dl).ok()?;
            let seed = (js_sys::Math::random() * u32::MAX as f64) as u32;
            Some(Mus { comp: Composer::new(seed, ctx.current_time()), bus, pad_f, pluck, verb_in, mood: None, buf: vec![] })
        }
        /// One oscillator with a gain envelope: `env` = (time, level) points after `t` (linear ramps), ending silent.
        fn osc(ctx: &AudioContext, to: &AudioNode, ty: OscillatorType, hz: f32, t: f64, env: &[(f64, f32)], exp: bool) {
            let (Ok(o), Ok(g)) = (ctx.create_oscillator(), ctx.create_gain()) else { return };
            o.set_type(ty); o.frequency().set_value(hz);
            let _ = g.gain().set_value_at_time(if exp { 0.0001 } else { 0. }, t);
            for &(dt, v) in env { if exp { let _ = g.gain().exponential_ramp_to_value_at_time(v.max(0.0001), t + dt); } else { let _ = g.gain().linear_ramp_to_value_at_time(v, t + dt); } }
            let end = env.last().map(|e| e.0).unwrap_or(1.);
            let _ = o.connect_with_audio_node(&g); let _ = g.connect_with_audio_node(to); let _ = o.start_with_when(t); let _ = o.stop_with_when(t + end + 0.05);
        }
        fn voice(ctx: &AudioContext, m: &Mus, n: &Note) {
            let (t, f, d, v) = (n.t, n.hz, n.dur as f64, n.vel);
            use OscillatorType as O;
            match n.voice {
                // soft pad: triangle + slightly detuned sine, slow swell and long release
                Voice::Pad => { let (a, r) = (3.5f64.min(d / 3.), 5f64.min(d / 2.)); let env = [(a, v * 0.07), (d - r, v * 0.07), (d, 0.)];
                    Self::osc(ctx, &m.pad_f, O::Triangle, f, t, &env, false); Self::osc(ctx, &m.pad_f, O::Sine, f * 1.004, t, &env, false); }
                // kalimba / marimba: a sine with a quick tick of 4th harmonic and a softer 2nd
                Voice::Pluck => { Self::osc(ctx, &m.pluck, O::Sine, f, t, &[(0.006, v * 0.2), (d, 0.0001)], true);
                    Self::osc(ctx, &m.pluck, O::Sine, f * 4., t, &[(0.004, v * 0.035), (0.22, 0.0001)], true);
                    Self::osc(ctx, &m.pluck, O::Sine, f * 2., t, &[(0.006, v * 0.04), (0.7, 0.0001)], true); }
                // bell: inharmonic partial, mostly heard through the reverb
                Voice::Bell => { Self::osc(ctx, &m.verb_in, O::Sine, f, t, &[(0.01, v * 0.08), (d, 0.0001)], true);
                    Self::osc(ctx, &m.verb_in, O::Sine, f * 2.76, t, &[(0.01, v * 0.025), (d * 0.4, 0.0001)], true); }
                // drone: root and fifth, very slow in and out
                Voice::Drone => { let env = [(8f64.min(d / 3.), v * 0.09), (d - 8f64.min(d / 3.), v * 0.09), (d, 0.)];
                    Self::osc(ctx, &m.pad_f, O::Sine, f, t, &env, false); Self::osc(ctx, &m.pad_f, O::Sine, f * 1.5, t, &env.map(|(a, b)| (a, b * 0.4)), false); }
            }
        }
        /// Call every frame: keeps the composer a little ahead of the audio clock.
        pub fn music(&mut self, depth_m: f32, heat: f32) {
            let Some(ctx) = &self.ctx else { return }; let Some(m) = self.mus.as_mut() else { return };
            if self.mvol <= 0. || ctx.state() != AudioContextState::Running { return; }
            let now = ctx.current_time(); let mood = Mood::pick(depth_m, heat);
            let mut buf = std::mem::take(&mut m.buf); buf.clear();
            m.comp.schedule(now + 0.05, now + 1.5, mood, &mut buf);
            if m.mood != Some(m.comp.mood) { m.mood = Some(m.comp.mood); let _ = m.pad_f.frequency().set_target_at_time(m.comp.mood.def().cutoff, now, 4.); }
            for n in &buf { Self::voice(ctx, m, n); }
            m.buf = buf;
        }
        /// Browsers only allow audio after a user gesture: call on first input.
        pub fn unlock(&mut self) {
            if self.ctx.is_some() { if let Some(c) = &self.ctx { let _ = c.resume(); } return; }
            let Ok(ctx) = AudioContext::new() else { return };
            let sr = ctx.sample_rate();
            let mk = |secs: f32, brown: bool| -> Option<AudioBuffer> {
                let n = (sr * secs) as u32; let b = ctx.create_buffer(1, n, sr).ok()?;
                let mut d = vec![0f32; n as usize]; let mut s = 0x1234567u32; let mut acc = 0f32;
                for v in d.iter_mut() { s ^= s << 13; s ^= s >> 17; s ^= s << 5; let r = (s as f32 / u32::MAX as f32) * 2. - 1.; if brown { acc = (acc + 0.02 * r) / 1.02; *v = acc * 3.5; } else { *v = r; } }
                b.copy_to_channel(&d, 0).ok()?; Some(b)
            };
            self.white = mk(2., false); self.brown = mk(4., true);
            let m = ctx.create_gain().ok(); if let Some(m) = &m { m.gain().set_value(0.55 * self.vol); let _ = m.connect_with_audio_node(&ctx.destination()); }
            self.master = m; self.mus = Self::music_graph(&ctx, self.mvol); self.ctx = Some(ctx);
            // ambient rumble loop
            if let (Some(ctx), Some(b), Some(m)) = (&self.ctx, &self.brown, &self.master) {
                if let (Ok(src), Ok(f), Ok(g)) = (ctx.create_buffer_source(), ctx.create_biquad_filter(), ctx.create_gain()) {
                    src.set_buffer(Some(b)); src.set_loop(true); f.set_type(BiquadFilterType::Lowpass); f.frequency().set_value(110.); g.gain().set_value(0.0);
                    let _ = src.connect_with_audio_node(&f); let _ = f.connect_with_audio_node(&g); let _ = g.connect_with_audio_node(m); let _ = src.start(); self.rumble = Some(g);
                }
            }
        }
        pub fn ambient(&mut self, depth: f32) { if let (Some(c), Some(g)) = (&self.ctx, &self.rumble) { let _ = g.gain().set_target_at_time(0.06 + 0.16 * depth.clamp(0., 1.), c.current_time(), 0.5); } }
        fn noise(&self, brown: bool, ft: BiquadFilterType, f0: f32, f1: f32, dur: f64, peak: f32, gain: f32) {
            let (Some(ctx), Some(m)) = (&self.ctx, &self.master) else { return }; let Some(b) = (if brown { &self.brown } else { &self.white }) else { return };
            let t = ctx.current_time();
            let (Ok(src), Ok(f), Ok(g)) = (ctx.create_buffer_source(), ctx.create_biquad_filter(), ctx.create_gain()) else { return };
            src.set_buffer(Some(b)); f.set_type(ft); f.frequency().set_value(f0); let _ = f.frequency().exponential_ramp_to_value_at_time(f1.max(20.), t + dur);
            g.gain().set_value(0.0001); let _ = g.gain().exponential_ramp_to_value_at_time(peak * gain, t + 0.01); let _ = g.gain().exponential_ramp_to_value_at_time(0.0001, t + dur);
            let _ = src.connect_with_audio_node(&f); let _ = f.connect_with_audio_node(&g); let _ = g.connect_with_audio_node(m);
            let off = (t * 1.7) % 1.0; let _ = src.start_with_when_and_grain_offset(t, off); let _ = src.stop_with_when(t + dur + 0.05);
        }
        fn tone(&self, ty: OscillatorType, f0: f32, f1: f32, dur: f64, peak: f32, gain: f32) {
            let (Some(ctx), Some(m)) = (&self.ctx, &self.master) else { return }; let t = ctx.current_time();
            let (Ok(o), Ok(g)) = (ctx.create_oscillator(), ctx.create_gain()) else { return };
            o.set_type(ty); o.frequency().set_value(f0); let _ = o.frequency().exponential_ramp_to_value_at_time(f1, t + dur);
            g.gain().set_value(0.0001); let _ = g.gain().exponential_ramp_to_value_at_time(peak * gain, t + 0.01); let _ = g.gain().exponential_ramp_to_value_at_time(0.0001, t + dur);
            let _ = o.connect_with_audio_node(&g); let _ = g.connect_with_audio_node(m); let _ = o.start(); let _ = o.stop_with_when(t + dur + 0.05);
        }
        /// `gain` = spatial attenuation from the listener (camera).
        pub fn play(&mut self, e: Event, gain: f32) {
            let Some(ctx) = &self.ctx else { return }; let now = ctx.current_time();
            let (k, gap) = match e { Event::Explode => (0, 0.3), Event::Crumble => (1, 1.2), Event::Crack => (2, 0.8), Event::Dig => (3, 0.07), Event::Place => (4, 0.1), Event::Hurt => (5, 0.3), Event::Death => (6, 1.), Event::Chest => (7, 0.5), Event::Quest => (8, 0.5), Event::Craft => (9, 0.3), Event::Discover => (10, 0.5), Event::Eat => (11, 0.3), Event::Warning => (12, 1.), Event::Quake => (13, 2.), Event::Click => (14, 0.05), Event::Harvest => (15, 0.3), Event::Gasp => (16, 1.5) };
            if now - self.last[k] < gap { return; } self.last[k] = now;
            use BiquadFilterType as F; use OscillatorType as O;
            match e {
                Event::Explode => { self.noise(false, F::Lowpass, 2400., 60., 1.6, 0.9, gain); self.tone(O::Sine, 90., 28., 0.9, 0.8, gain); self.noise(true, F::Lowpass, 300., 80., 2.5, 0.7, gain); }
                Event::Crumble | Event::Quake => { self.noise(true, F::Lowpass, 500., 90., 2.8, 0.8, gain); for i in 0..4 { let _ = i; self.noise(false, F::Bandpass, 900. + i as f32 * 300., 400., 0.08 + i as f64 * 0.2, 0.3, gain); } }
                Event::Crack => self.noise(false, F::Bandpass, 2200., 900., 0.12, 0.25, gain),
                Event::Dig => self.noise(false, F::Bandpass, 900. + (now * 997.0 % 900.) as f32, 700., 0.07, 0.18, gain),
                Event::Place => { self.tone(O::Triangle, 220., 180., 0.08, 0.3, gain); self.tone(O::Triangle, 260., 200., 0.1, 0.2, gain); }
                Event::Hurt => self.tone(O::Square, 900., 600., 0.09, 0.12, gain),
                Event::Death => self.tone(O::Triangle, 880., 180., 0.7, 0.3, gain),
                Event::Chest | Event::Discover => { self.tone(O::Sine, 660., 990., 0.18, 0.25, gain); self.tone(O::Sine, 990., 1320., 0.3, 0.18, gain); }
                Event::Quest => { self.tone(O::Triangle, 523., 523., 0.15, 0.3, 1.); self.tone(O::Triangle, 784., 1046., 0.4, 0.25, 1.); }
                Event::Craft => self.tone(O::Triangle, 330., 300., 0.12, 0.25, gain),
                Event::Eat => self.noise(false, F::Bandpass, 1400., 700., 0.1, 0.2, gain),
                Event::Warning => { self.tone(O::Square, 440., 440., 0.25, 0.12, 1.); self.tone(O::Square, 330., 330., 0.5, 0.1, 1.); }
                Event::Click => self.tone(O::Square, 1800., 1200., 0.03, 0.08, 1.),
                Event::Harvest => { self.tone(O::Sine, 440., 520., 0.1, 0.2, gain); self.noise(false, F::Bandpass, 2000., 900., 0.06, 0.12, gain); }
                Event::Gasp => { self.noise(false, F::Bandpass, 700., 300., 0.5, 0.25, gain); self.tone(O::Sine, 300., 220., 0.4, 0.12, gain); }
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::Event;
    pub struct Audio;
    impl Audio { pub fn new() -> Self { Audio } pub fn unlock(&mut self) {} pub fn play(&mut self, _e: Event, _g: f32) {} pub fn ambient(&mut self, _d: f32) {} pub fn set_volume(&mut self, _v: f32) {} pub fn set_music(&mut self, _v: f32) {} pub fn music(&mut self, _depth_m: f32, _heat: f32) {} }
}
pub use imp::Audio;
