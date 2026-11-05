//! Procedural sound (recipes from the v2/ambience design notes): no audio files, WebAudio
//! oscillators + noise buffers with exponential envelopes. Web only; native builds are silent.
use molehill_sim::world::Event;

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::Event;
    use web_sys::*;
    pub struct Audio { ctx: Option<AudioContext>, white: Option<AudioBuffer>, brown: Option<AudioBuffer>, last: [f64; 18], master: Option<GainNode>, rumble: Option<GainNode>, vol: f32 }
    impl Audio {
        pub fn new() -> Self { Audio { ctx: None, white: None, brown: None, last: [0.; 18], master: None, rumble: None, vol: 0.8 } }
        pub fn set_volume(&mut self, v: f32) { self.vol = v; if let Some(m) = &self.master { m.gain().set_value(0.55 * v); } }
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
            self.master = m; self.ctx = Some(ctx);
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
    impl Audio { pub fn new() -> Self { Audio } pub fn unlock(&mut self) {} pub fn play(&mut self, _e: Event, _g: f32) {} pub fn ambient(&mut self, _d: f32) {} pub fn set_volume(&mut self, _v: f32) {} }
}
pub use imp::Audio;
