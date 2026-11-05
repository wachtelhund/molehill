//! Camera zoom rules shared by the settings menu, the keys and the wheel, kept here so they can be
//! tested headless. `scale` is world units per screen pixel (the Bevy camera scale): smaller is closer.
/// Closest and farthest camera scale. At 0.12 a cell is about 33 px; at 0.75 about 5 px (whole caverns).
pub const ZOOM_MIN: f32 = 0.12;
pub const ZOOM_MAX: f32 = 0.75;
pub const ZOOM_DEFAULT: f32 = 0.32;
/// One zoom notch changes the scale by this factor.
pub const ZOOM_STEP: f32 = 1.15;
/// Zoom by `steps` notches: positive zooms in (closer), negative zooms out. Always clamped.
pub fn zoom_step(scale: f32, steps: f32) -> f32 { clamp_zoom(scale / ZOOM_STEP.powf(steps)) }
/// Clamp a (possibly corrupt or out-of-date) scale into range; NaN and non-positive fall back to the default.
pub fn clamp_zoom(scale: f32) -> f32 { if scale.is_finite() && scale > 0. { scale.clamp(ZOOM_MIN, ZOOM_MAX) } else { ZOOM_DEFAULT } }
/// Zoom as a magnification for the settings label: 1.0 at the default, bigger is closer.
pub fn zoom_level(scale: f32) -> f32 { ZOOM_DEFAULT / clamp_zoom(scale) }

/// Mouse wheel → discrete notches (hotbar slot, brush size, zoom). Kept here so it is tested headless.
/// * No scroll is no notch (beware `f32::signum(0.0) == 1.0`).
/// * Line units (a notched wheel): one notch per frame with any movement.
/// * Pixel units (trackpads, smooth-scrolling browsers): deltas add up to `WHEEL_PX` for one notch,
///   then the gesture is spent until the wheel has been quiet for `WHEEL_QUIET` s, so a flick and its
///   momentum tail move one slot instead of spinning the hotbar.
#[derive(Clone, Copy, Default, Debug)]
pub struct Wheel { acc: f32, quiet: f32, spent: bool }
pub const WHEEL_PX: f32 = 24.;
pub const WHEEL_QUIET: f32 = 0.12;
impl Wheel {
    /// `dy` this frame (positive = up / away), in pixels when `pixels`, else lines. Returns -1, 0 or 1.
    pub fn feed(&mut self, dy: f32, pixels: bool, dt: f32) -> i32 {
        if !dy.is_finite() || dy == 0. {
            self.quiet += dt.max(0.);
            if self.quiet >= WHEEL_QUIET { self.acc = 0.; self.spent = false; }
            return 0;
        }
        self.quiet = 0.;
        if !pixels { self.acc = 0.; self.spent = false; return if dy > 0. { 1 } else { -1 }; }
        if self.acc != 0. && self.acc.signum() != dy.signum() { self.acc = 0.; self.spent = false; } // turned back: a new gesture
        if self.spent { return 0; }
        self.acc += dy;
        if self.acc.abs() >= WHEEL_PX { let n = if self.acc > 0. { 1 } else { -1 }; self.acc = 0.; self.spent = true; n } else { 0 }
    }
}

/// Modifier keys held when the wheel turns. `shift` is sprint: it never changes what the wheel does.
#[derive(Clone, Copy, Default, Debug)]
pub struct Mods { pub shift: bool, pub ctrl: bool, pub cmd: bool, pub alt: bool }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WheelUse { Hotbar, Zoom, Brush }
/// What a wheel notch does: `Ctrl`/`Cmd` + wheel zooms (a trackpad pinch arrives as Ctrl + wheel in browsers),
/// `Alt`/`Option` + wheel sizes the brush of the tool or block in hand, the plain wheel picks a hotbar slot.
pub fn wheel_use(m: Mods, has_brush: bool) -> WheelUse {
    if m.ctrl || m.cmd { WheelUse::Zoom } else if m.alt && has_brush { WheelUse::Brush } else { WheelUse::Hotbar }
}
/// The scroll amount to use. With Shift held, browsers and macOS turn a vertical wheel into a horizontal
/// one, so while sprinting a sideways-only scroll counts as the wheel.
pub fn wheel_delta(dx: f32, dy: f32, m: Mods) -> f32 { if dy == 0. && m.shift { -dx } else { dy } }
