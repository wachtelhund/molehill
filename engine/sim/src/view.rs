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

/// The brush sizes the player picked, per brush item (dig tools and blocks), and grid snap for building.
/// Kept here so the size keys and the wheel are tested headless (`brush_size_keys.scn`).
#[derive(Clone, Debug, Default)]
pub struct BrushSizes { want: [f32; 7], pub snap: bool, snap_k: usize }
/// Grid snap square sizes in cells: a quarter, half and whole metre.
pub const SNAP_CELLS: [usize; 3] = [crate::mat::CPM / 4, crate::mat::CPM / 2, crate::mat::CPM];
impl BrushSizes {
    fn slot(b: crate::world::Brush) -> usize {
        use crate::{world::Brush, craft::Item};
        match b { Brush::Dig(n) => (n as usize).min(3), Brush::Place(Item::WoodPillar) => 4, Brush::Place(Item::Brick) => 5, _ => 6 }
    }
    /// Radius in cells for this brush (the pick, clamped to the item's range).
    pub fn radius(&self, b: crate::world::Brush) -> f32 { crate::world::brush_radius(b, self.want[Self::slot(b)]) }
    /// Is this brush placing on the grid (snap on, a block in hand)?
    pub fn snapping(&self, b: crate::world::Brush) -> bool { self.snap && matches!(b, crate::world::Brush::Place(_)) }
    /// Side of the snap square in cells.
    pub fn snap_cells(&self) -> usize { SNAP_CELLS[self.snap_k.min(SNAP_CELLS.len() - 1)] }
    /// Grow (+) or shrink (−) the brush by whole steps: a tile of radius, or with grid snap the next square size.
    pub fn step(&mut self, b: crate::world::Brush, steps: i32) {
        if steps == 0 { return; }
        if self.snapping(b) { self.snap_k = (self.snap_k as i32 + steps).clamp(0, SNAP_CELLS.len() as i32 - 1) as usize; return; }
        let k = Self::slot(b); let r = self.radius(b);
        let (lo, hi, _) = crate::world::brush_range(b).unwrap_or((r, r, r));
        self.want[k] = (r + steps as f32 * 0.5 * crate::mat::SF).clamp(lo, hi);
    }
    /// The size written beside the outline: the brush across, or the grid square, in metres.
    pub fn label(&self, b: crate::world::Brush) -> String {
        if self.snapping(b) { format!("grid {} m", fmt_m(self.snap_cells() as f32 * crate::mat::CELL_M)) }
        else { format!("{} m", fmt_m(2. * self.radius(b) * crate::mat::CELL_M)) }
    }
}
fn fmt_m(v: f32) -> String { let s = format!("{v:.2}"); s.trim_end_matches('0').trim_end_matches('.').to_string() }
/// Brush size keys: `[` shrinks, `]` grows. `typed` is the character the key produced (on a Swedish or
/// German Mac `[` is Option+8), `physical` the key right of P on a US layout; either counts.
pub fn brush_key_steps(typed_open: bool, typed_close: bool, phys_open: bool, phys_close: bool) -> i32 {
    (typed_close || phys_close) as i32 - (typed_open || phys_open) as i32
}
/// Digits pick a hotbar slot, except with Option/Alt held: on many Mac layouts that types a symbol (`[` `]`).
pub fn digit_selects_slot(m: Mods) -> bool { !m.alt }
