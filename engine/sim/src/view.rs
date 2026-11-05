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
