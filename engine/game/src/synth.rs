//! The native synth: the WebAudio graph that `audio.rs` builds on the web, rendered in software for
//! native builds. It plays the same recipes (`audio::Sfx`, `audio::Osc`) with the same envelopes, filters
//! (the WebAudio spec's biquads), echo and generated cave reverb (a convolution with the same impulse,
//! normalised as a browser's `ConvolverNode` does), so a native build sounds like the web one.
//! Pure DSP: `audio.rs` feeds it commands and hands its output to the sound card (cpal). Unit-tested
//! offline.
use crate::audio::{self, Bus, Filt, Osc, Sfx, Wave};

/// What the game thread sends the audio thread.
#[derive(Clone, Debug)]
pub enum Cmd { Sfx(Sfx), Osc(Osc), Master(f32), Music(f32), Rumble(f32), Cutoff(f32) }

/// A parameter moving toward a target as WebAudio's `setTargetAtTime` does (time constant `tc` s).
#[derive(Clone, Copy)] struct Param { v: f32, target: f32, k: f32 }
impl Param {
    fn new(v: f32, tc: f32, sr: f32) -> Self { Param { v, target: v, k: 1. - (-1. / (tc * sr)).exp() } }
    #[inline] fn next(&mut self) -> f32 { self.v += (self.target - self.v) * self.k; self.v }
}

/// A biquad with the WebAudio spec's coefficients (lowpass Q in dB, bandpass Q linear).
#[derive(Clone, Copy, Default)] pub struct Biquad { b0: f32, b1: f32, b2: f32, a1: f32, a2: f32, x1: f32, x2: f32, y1: f32, y2: f32 }
impl Biquad {
    pub fn set(&mut self, f: Filt, hz: f32, q: f32, sr: f32) {
        let w0 = 2. * std::f32::consts::PI * (hz / sr).clamp(1e-5, 0.4999); let (s, c) = w0.sin_cos();
        let (b0, b1, b2, a0, a1, a2) = match f {
            Filt::Lowpass => { let a = s / (2. * 10f32.powf(q / 20.)); ((1. - c) / 2., 1. - c, (1. - c) / 2., 1. + a, -2. * c, 1. - a) }
            Filt::Bandpass => { let a = s / (2. * q); (a, 0., -a, 1. + a, -2. * c, 1. - a) }
        };
        self.b0 = b0 / a0; self.b1 = b1 / a0; self.b2 = b2 / a0; self.a1 = a1 / a0; self.a2 = a2 / a0;
    }
    pub fn new(f: Filt, hz: f32, q: f32, sr: f32) -> Self { let mut b = Biquad::default(); b.set(f, hz, q, sr); b }
    #[inline] pub fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1; self.x1 = x; self.y2 = self.y1; self.y1 = y; y
    }
}

/// One sample of a WebAudio periodic wave at phase `p` (0..1); the square is band-limited (PolyBLEP).
#[inline] fn wave(w: Wave, p: f32, dp: f32) -> f32 {
    match w {
        Wave::Sine => (p * std::f32::consts::TAU).sin(),
        Wave::Triangle => if p < 0.25 { 4. * p } else if p < 0.75 { 2. - 4. * p } else { 4. * p - 4. },
        Wave::Square => {
            let blep = |t: f32| if t < dp { let t = t / dp; 2. * t - t * t - 1. } else if t > 1. - dp { let t = (t - 1.) / dp; t * t + 2. * t + 1. } else { 0. };
            (if p < 0.5 { 1. } else { -1. }) + blep(p) - blep((p + 0.5) % 1.)
        }
    }
}

/// Gain at `tau` s after the start of an envelope of (time, level) points from `v0` at 0, ramped
/// linearly or exponentially as WebAudio does; it holds the last level after the last point.
fn env_at(v0: f32, pts: &[(f64, f32)], exp: bool, tau: f64) -> f32 {
    let (mut t0, mut a) = (0f64, v0);
    for &(t1, b) in pts {
        let b = if exp { b.max(1e-4) } else { b };
        if tau < t1 { let f = ((tau - t0) / (t1 - t0).max(1e-9)) as f32; return if exp { a * (b / a).powf(f) } else { a + (b - a) * f }; }
        t0 = t1; a = b;
    }
    a
}

struct SfxVoice { s: Sfx, t: f64, phase: f32, pos: usize, f: Biquad }
struct OscVoice { o: Osc, phase: f32 }

/// The audio thread's whole graph.
pub struct Engine {
    sr: f32, n: u64, white: Vec<f32>, brown: Vec<f32>,
    sfx: Vec<SfxVoice>, osc: Vec<OscVoice>,
    master: f32, music: Param, rumble: Param, cutoff: Param, cut_now: f32,
    rumble_pos: usize, rumble_lp: Biquad, pad_f: Biquad,
    echo: Vec<f32>, echo_at: usize, echo_lp: Biquad, verb: Conv,
}
/// Most voices at once (a burst of events past this is dropped, as a browser would struggle there too).
const MAX_SFX: usize = 64; const MAX_OSC: usize = 256;
impl Engine {
    pub fn new(sr: u32, vol: f32, mvol: f32) -> Self {
        let srf = sr as f32;
        let ir = audio::reverb_ir(srf); let scale = convolver_scale(&ir, srf);
        Engine { sr: srf, n: 0, white: audio::noise_buf(srf, 2., false), brown: audio::noise_buf(srf, 4., true),
            sfx: vec![], osc: vec![],
            master: audio::MASTER * vol, music: Param::new(audio::MUSIC_GAIN * mvol, audio::MUSIC_TC, srf),
            rumble: Param::new(0., audio::RUMBLE_TC, srf), cutoff: Param::new(audio::PAD_CUTOFF, audio::CUTOFF_TC, srf), cut_now: audio::PAD_CUTOFF,
            rumble_pos: 0, rumble_lp: Biquad::new(Filt::Lowpass, audio::RUMBLE_LP, 1., srf),
            pad_f: Biquad::new(Filt::Lowpass, audio::PAD_CUTOFF, audio::PAD_Q, srf),
            echo: vec![0.; (audio::ECHO_DELAY * srf) as usize], echo_at: 0, echo_lp: Biquad::new(Filt::Lowpass, audio::ECHO_LP, 1., srf),
            verb: Conv::new(&ir, scale, 1024) }
    }
    /// The audio clock (s).
    pub fn time(&self) -> f64 { self.n as f64 / self.sr as f64 }
    /// Frames rendered so far.
    pub fn frames(&self) -> u64 { self.n }
    pub fn cmd(&mut self, c: Cmd) {
        match c {
            Cmd::Sfx(s) => if self.sfx.len() < MAX_SFX {
                let (f0, filt) = match s { Sfx::Noise { f0, filt, .. } => (f0, filt), Sfx::Tone { .. } => (1000., Filt::Lowpass) };
                let t = self.time(); let pos = (audio::noise_offset(t) * self.sr as f64) as usize;
                self.sfx.push(SfxVoice { s, t, phase: 0., pos, f: Biquad::new(filt, f0, 1., self.sr) });
            },
            Cmd::Osc(o) => if self.osc.len() < MAX_OSC { self.osc.push(OscVoice { o, phase: 0. }); },
            Cmd::Master(v) => self.master = v,
            Cmd::Music(v) => self.music.target = v,
            Cmd::Rumble(v) => self.rumble.target = v,
            Cmd::Cutoff(v) => self.cutoff.target = v,
        }
    }
    /// Render interleaved frames of `ch` channels (the first two get left and right; the rest, if any, the
    /// mix; one channel gets the mix).
    pub fn render(&mut self, out: &mut [f32], ch: usize) {
        let sr = self.sr;
        for frame in out.chunks_mut(ch.max(1)) {
            let now = self.time();
            // effects: tones and noise bursts, into the master
            let mut fx = 0f32;
            let recalc = self.n % 16 == 0;
            for v in &mut self.sfx {
                let tau = now - v.t;
                match v.s {
                    Sfx::Tone { wave: w, f0, f1, dur, peak } => {
                        let hz = if tau < dur { f0 * (f1 / f0).powf((tau / dur) as f32) } else { f1 };
                        let dp = hz / sr; v.phase = (v.phase + dp) % 1.;
                        fx += wave(w, v.phase, dp) * sfx_env(peak, dur, tau);
                    }
                    Sfx::Noise { brown, filt, f0, f1, dur, peak } => {
                        if recalc { let f1 = f1.max(20.); let hz = if tau < dur { f0 * (f1 / f0).powf((tau / dur) as f32) } else { f1 }; v.f.set(filt, hz, 1., sr); }
                        let buf = if brown { &self.brown } else { &self.white };
                        let x = buf.get(v.pos).copied().unwrap_or(0.); v.pos += 1;
                        fx += v.f.run(x) * sfx_env(peak, dur, tau);
                    }
                }
            }
            self.sfx.retain(|v| now - v.t < v.s.dur() + 0.05);
            // the rumble loop
            let r = self.brown[self.rumble_pos]; self.rumble_pos = (self.rumble_pos + 1) % self.brown.len();
            fx += self.rumble_lp.run(r) * self.rumble.next();
            let fx = fx * self.master;
            // music voices onto their buses
            let (mut pad, mut pluck, mut bell) = (0f32, 0f32, 0f32);
            for v in &mut self.osc {
                let tau = now - v.o.t; if tau < 0. { continue; }
                let dp = v.o.hz / sr; v.phase = (v.phase + dp) % 1.;
                let g = env_at(if v.o.exp { 1e-4 } else { 0. }, &v.o.env, v.o.exp, tau);
                let s = wave(v.o.wave, v.phase, dp) * g;
                match v.o.bus { Bus::Pad => pad += s, Bus::Pluck => pluck += s, Bus::Verb => bell += s }
            }
            self.osc.retain(|v| now - v.o.t < v.o.end() + 0.05);
            let c = self.cutoff.next(); if recalc && (c - self.cut_now).abs() > 0.5 { self.cut_now = c; self.pad_f.set(Filt::Lowpass, c, audio::PAD_Q, sr); }
            let pad = self.pad_f.run(pad); let pluck = pluck * audio::PLUCK_GAIN;
            // echo: delay -> lowpass -> feedback back in; its output to the bus and the reverb
            let d = self.echo[self.echo_at]; let e = self.echo_lp.run(d);
            self.echo[self.echo_at] = pluck + audio::ECHO_FB * e; self.echo_at = (self.echo_at + 1) % self.echo.len();
            let eo = e * audio::ECHO_OUT;
            let (wl, wr) = self.verb.run(pad + pluck + bell + eo);
            let dry = pad + pluck + eo; let mg = self.music.next();
            let l = (fx + (dry + audio::VERB_WET * wl) * mg).clamp(-1., 1.);
            let rr = (fx + (dry + audio::VERB_WET * wr) * mg).clamp(-1., 1.);
            match frame.len() { 1 => frame[0] = 0.5 * (l + rr), _ => { frame[0] = l; frame[1] = rr; for x in &mut frame[2..] { *x = 0.5 * (l + rr); } } }
            self.n += 1;
        }
    }
}
/// The effect envelope: from 0.0001 up to `peak` in 10 ms, then exponentially down to 0.0001 at `dur`.
fn sfx_env(peak: f32, dur: f64, tau: f64) -> f32 { env_at(1e-4, &[(0.01, peak), (dur, 1e-4)], true, tau) }

/// The scale a browser's `ConvolverNode` (normalize = true) puts on an impulse: unit RMS power, then
/// -58 dB, then by 44.1 kHz over the sample rate.
pub fn convolver_scale(ir: &[Vec<f32>], sr: f32) -> f32 {
    let (mut sum, mut n) = (0f64, 0usize); for c in ir { for &x in c { sum += (x * x) as f64; } n += c.len(); }
    let power = ((sum / n.max(1) as f64).sqrt() as f32).max(0.000125);
    (1. / power) * 10f32.powf(-58. / 20.) * (44100. / sr)
}

/// A complex number for the FFT.
#[derive(Clone, Copy, Default, Debug)] struct C { re: f32, im: f32 }
impl C { #[inline] fn mul(self, o: C) -> C { C { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re } } }
/// In-place radix-2 FFT of a power-of-two length (`inv`: the unscaled inverse).
fn fft(a: &mut [C], tw: &[C], inv: bool) {
    let n = a.len(); let mut j = 0;
    for i in 1..n { let mut bit = n >> 1; while j & bit != 0 { j ^= bit; bit >>= 1; } j |= bit; if i < j { a.swap(i, j); } }
    let mut len = 2;
    while len <= n {
        let step = n / len;
        for s in (0..n).step_by(len) { for k in 0..len / 2 {
            let w = tw[k * step]; let w = if inv { C { re: w.re, im: -w.im } } else { w };
            let u = a[s + k]; let v = a[s + k + len / 2].mul(w);
            a[s + k] = C { re: u.re + v.re, im: u.im + v.im }; a[s + k + len / 2] = C { re: u.re - v.re, im: u.im - v.im };
        } }
        len <<= 1;
    }
}

/// Mono-in, stereo-out convolution with a long impulse: uniformly partitioned, overlap-save, so a
/// 3.5 s cave costs a few FFTs a block. Output runs one block (`b` samples) behind its input.
pub struct Conv { b: usize, tw: Vec<C>, h: [Vec<Vec<C>>; 2], fdl: Vec<Vec<C>>, at: usize, inp: Vec<f32>, prev: Vec<f32>, out: [Vec<f32>; 2], i: usize, buf: Vec<C>, acc: Vec<C> }
impl Conv {
    pub fn new(ir: &[Vec<f32>], scale: f32, b: usize) -> Self {
        let n = 2 * b; let tw = (0..n / 2).map(|k| { let a = -std::f32::consts::TAU * k as f32 / n as f32; C { re: a.cos(), im: a.sin() } }).collect::<Vec<_>>();
        let parts = ir.iter().map(|c| c.len()).max().unwrap_or(1).div_ceil(b).max(1);
        let h = [0, 1].map(|ch| { let c = &ir[ch.min(ir.len() - 1)];
            (0..parts).map(|p| { let mut a = vec![C::default(); n];
                for k in 0..b { if let Some(&x) = c.get(p * b + k) { a[k].re = x * scale; } }
                fft(&mut a, &tw, false); a }).collect::<Vec<_>>() });
        Conv { b, tw, h, fdl: vec![vec![C::default(); n]; parts], at: 0, inp: vec![0.; b], prev: vec![0.; b], out: [vec![0.; b], vec![0.; b]], i: 0, buf: vec![C::default(); n], acc: vec![C::default(); n] }
    }
    #[inline] pub fn run(&mut self, x: f32) -> (f32, f32) {
        let y = (self.out[0][self.i], self.out[1][self.i]); self.inp[self.i] = x; self.i += 1;
        if self.i == self.b { self.block(); self.i = 0; }
        y
    }
    fn block(&mut self) {
        let (b, n, parts) = (self.b, 2 * self.b, self.fdl.len());
        for k in 0..b { self.buf[k] = C { re: self.prev[k], im: 0. }; self.buf[b + k] = C { re: self.inp[k], im: 0. }; }
        fft(&mut self.buf, &self.tw, false);
        self.at = (self.at + parts - 1) % parts; self.fdl[self.at].copy_from_slice(&self.buf);
        for ch in 0..2 {
            self.acc.iter_mut().for_each(|c| *c = C::default());
            for p in 0..parts { let x = &self.fdl[(self.at + p) % parts]; let h = &self.h[ch][p];
                for k in 0..n { let m = x[k].mul(h[k]); self.acc[k].re += m.re; self.acc[k].im += m.im; } }
            fft(&mut self.acc, &self.tw, true);
            for k in 0..b { self.out[ch][k] = self.acc[b + k].re / n as f32; }
        }
        std::mem::swap(&mut self.prev, &mut self.inp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use molehill_sim::world::Event;
    const SR: u32 = 48000;
    fn render(e: &mut Engine, secs: f32) -> Vec<f32> { let mut v = vec![0.; (secs * SR as f32) as usize * 2]; e.render(&mut v, 2); v }
    fn peak(v: &[f32]) -> f32 { v.iter().fold(0f32, |a, &x| a.max(x.abs())) }
    fn rms(v: &[f32]) -> f32 { (v.iter().map(|x| x * x).sum::<f32>() / v.len().max(1) as f32).sqrt() }

    #[test] fn partitioned_convolution_matches_direct() {
        let ir = vec![(0..300).map(|k| ((k * 37 % 11) as f32 - 5.) / (1. + k as f32)).collect::<Vec<_>>(), (0..300).map(|k| (k as f32 * 0.1).sin() * 0.9f32.powi(k)).collect()];
        let x: Vec<f32> = (0..1000).map(|k| (k * 7919 % 101) as f32 / 50. - 1.).collect();
        let mut c = Conv::new(&ir, 0.5, 64); let got: Vec<(f32, f32)> = x.iter().map(|&s| c.run(s)).collect();
        for t in 64..1000 { for ch in 0..2 {
            let want: f32 = (0..ir[ch].len()).filter(|&k| k <= t - 64).map(|k| 0.5 * ir[ch][k] * x[t - 64 - k]).sum();
            let g = if ch == 0 { got[t].0 } else { got[t].1 }; assert!((g - want).abs() < 1e-3, "t {t} ch {ch}: {g} vs {want}"); } }
    }
    #[test] fn lowpass_passes_dc_and_cuts_high() {
        let mut f = Biquad::new(Filt::Lowpass, 200., 1., SR as f32); let mut y = 0.; for _ in 0..20000 { y = f.run(1.); } assert!((y - 1.).abs() < 1e-3, "dc {y}");
        let mut f = Biquad::new(Filt::Lowpass, 200., 1., SR as f32); let hi: Vec<f32> = (0..20000).map(|k| f.run(if k % 2 == 0 { 1. } else { -1. })).collect(); assert!(peak(&hi[10000..]) < 1e-3);
    }
    #[test] fn silent_until_something_plays_and_effects_end() {
        let mut e = Engine::new(SR, 0.8, 0.35); assert!(peak(&render(&mut e, 0.5)) < 1e-6, "silence first");
        for k in audio::sfx(Event::Explode, 1., e.time()) { e.cmd(Cmd::Sfx(k)); }
        let v = render(&mut e, 3.); assert!(peak(&v) > 0.05 && peak(&v) <= 1., "the blast is heard: {}", peak(&v));
        assert!(e.sfx.is_empty(), "its voices end"); assert!(peak(&render(&mut e, 0.2)) < 1e-4);
    }
    #[test] fn every_event_sounds() {
        for ev in audio::EVENTS { let mut e = Engine::new(SR, 0.8, 0.35); let s = audio::sfx(ev, 1., 0.); assert!(!s.is_empty(), "{ev:?}");
            for k in s { e.cmd(Cmd::Sfx(k)); } let v = render(&mut e, 0.5); assert!(peak(&v) > 1e-3, "{ev:?} is silent"); }
    }
    #[test] fn volume_scales_effects_and_music_apart() {
        let blast = |vol: f32| { let mut e = Engine::new(SR, vol, 0.35); for k in audio::sfx(Event::Craft, 1., 0.) { e.cmd(Cmd::Sfx(k)); } rms(&render(&mut e, 0.2)) };
        assert!(blast(0.) < 1e-7); assert!((blast(0.8) / blast(0.4) - 2.).abs() < 0.01, "effects follow Settings volume");
        let tune = |vol: f32, mvol: f32| { let mut e = Engine::new(SR, vol, mvol);
            for o in audio::parts(&crate::music::Note { t: 0., voice: crate::music::Voice::Pluck, hz: 440., dur: 2.2, vel: 0.6 }) { e.cmd(Cmd::Osc(o)); } rms(&render(&mut e, 1.)) };
        assert!(tune(0.8, 0.) < 1e-7, "music volume 0 mutes the music"); assert!(tune(0., 0.35) > 1e-3, "effects volume doesn't touch the music");
        assert!((tune(0.8, 0.7) / tune(0.8, 0.35) - 2.).abs() < 0.01);
    }
    #[test] fn music_rings_on_in_the_cave() {
        let mut e = Engine::new(SR, 0.8, 1.);
        for o in audio::parts(&crate::music::Note { t: 0.1, voice: crate::music::Voice::Bell, hz: 660., dur: 1., vel: 0.4 }) { e.cmd(Cmd::Osc(o)); }
        let v = render(&mut e, 3.); let tail = &v[(2.2 * SR as f32) as usize * 2..];
        assert!(rms(tail) > 1e-4, "the reverb carries the bell past its end: {}", rms(tail)); assert!(peak(&v) < 1.);
    }
    #[test] fn rumble_follows_depth() {
        let lvl = |d: f32| { let mut e = Engine::new(SR, 0.8, 0.35); e.cmd(Cmd::Rumble(audio::rumble_level(d))); render(&mut e, 3.); rms(&render(&mut e, 1.)) };
        assert!(lvl(1.) > 1.5 * lvl(0.), "deeper rumbles louder"); assert!(lvl(0.) > 1e-4);
    }
    /// The audio thread keeps well ahead: a busy second (a chord of pads, plucks, a drone, the reverb, rumble
    /// and a blast) renders many times faster than real time (about 25x on an M-series Mac). Wall-clock, so it
    /// is opt-in and run alone: `cargo test --release -p molehill renders_faster -- --ignored --nocapture`.
    #[test] #[ignore] fn renders_faster_than_real_time() {
        let mut e = Engine::new(SR, 0.8, 1.); e.cmd(Cmd::Rumble(0.2)); render(&mut e, 0.1);
        for v in [crate::music::Voice::Pad, crate::music::Voice::Pad, crate::music::Voice::Pad, crate::music::Voice::Drone, crate::music::Voice::Pluck, crate::music::Voice::Bell] {
            for o in audio::parts(&crate::music::Note { t: 0.1, voice: v, hz: 220., dur: 12., vel: 0.5 }) { e.cmd(Cmd::Osc(o)); } }
        for k in audio::sfx(Event::Explode, 1., 0.1) { e.cmd(Cmd::Sfx(k)); }
        let t = std::time::Instant::now(); render(&mut e, 4.); let x = 4. / t.elapsed().as_secs_f32();
        println!("synth: {x:.0}x real time"); assert!(x > if cfg!(debug_assertions) { 1. } else { 5. }, "only {x:.1}x real time");
    }
    #[test] fn a_generated_minute_of_music_stays_in_range() {
        let mut e = Engine::new(SR, 0.8, 1.); let mut c = crate::music::Composer::new(11, 0.); let mut notes = vec![]; let mut t = 0.;
        let mut out = vec![]; while t < 60. { notes.clear(); c.schedule(t + 0.05, t + 1.5, crate::music::Mood::Loam, &mut notes);
            for n in &notes { for o in audio::parts(n) { e.cmd(Cmd::Osc(o)); } } out.extend(render(&mut e, 0.25)); t = e.time(); }
        assert!(peak(&out) < 0.99, "no clipping: {}", peak(&out)); assert!(rms(&out[(30 * SR) as usize * 2..]) > 1e-4, "music plays");
    }
}
