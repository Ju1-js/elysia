/// Animation helpers for consistent transition effects
///
/// This module provides pre-configured animations used throughout the game page
/// to reduce duplication and ensure consistent timing and easing.
use freya::prelude::*;

/// Standard fade animation configuration (0.0 to 1.0 over 700ms)
pub fn create_fade_animation() -> impl Fn(&mut AnimConfiguration) -> AnimNum {
    move |_| {
        AnimNum::new(0.0, 1.0)
            .time(700)
            .ease(Ease::InOut)
            .function(Function::Cubic)
    }
}

/// Scale animation for modal/settings popups (0.92 to 1.0 over 200ms)
pub fn create_scale_animation() -> impl Fn(&mut AnimConfiguration) -> AnimNum {
    move |_| {
        AnimNum::new(0.92, 1.0)
            .time(200)
            .ease(Ease::Out)
            .function(Function::Cubic)
    }
}
