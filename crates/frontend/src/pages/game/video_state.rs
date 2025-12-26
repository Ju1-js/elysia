use freya::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VideoSlot {
    Primary,
    Secondary,
}

impl VideoSlot {
    pub fn other(self) -> Self {
        match self {
            VideoSlot::Primary => VideoSlot::Secondary,
            VideoSlot::Secondary => VideoSlot::Primary,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SlotData {
    url: Signal<Option<String>>,
    ready: Signal<bool>,
    opacity: Signal<f64>,
}

impl SlotData {
    fn new() -> Self {
        Self {
            url: Signal::new(None),
            ready: Signal::new(false),
            opacity: Signal::new(0.0),
        }
    }

    fn clear(&mut self) {
        self.url.set(None);
        self.ready.set(false);
        self.opacity.set(0.0);
    }

    fn load(&mut self, url: Option<String>) {
        self.url.set(url);
        self.ready.set(false);
        self.opacity.set(0.0);
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct VideoState {
    primary: SlotData,
    secondary: SlotData,
    pub active_slot: Signal<VideoSlot>,
    pub transitioning: Signal<bool>,
}

impl VideoState {
    pub fn new() -> Self {
        let mut state = Self {
            primary: SlotData::new(),
            secondary: SlotData::new(),
            active_slot: Signal::new(VideoSlot::Primary),
            transitioning: Signal::new(false),
        };
        state.primary.opacity.set(1.0);
        state
    }

    fn get_slot(&self, slot: VideoSlot) -> &SlotData {
        match slot {
            VideoSlot::Primary => &self.primary,
            VideoSlot::Secondary => &self.secondary,
        }
    }

    fn get_slot_mut(&mut self, slot: VideoSlot) -> &mut SlotData {
        match slot {
            VideoSlot::Primary => &mut self.primary,
            VideoSlot::Secondary => &mut self.secondary,
        }
    }

    fn active(&self) -> &SlotData {
        self.get_slot(*self.active_slot.read())
    }

    fn inactive(&self) -> &SlotData {
        self.get_slot(self.active_slot.read().other())
    }

    fn inactive_mut(&mut self) -> &mut SlotData {
        let inactive_slot = self.active_slot.read().other();
        self.get_slot_mut(inactive_slot)
    }

    pub fn active_url(&self) -> Option<String> {
        self.active().url.read().clone()
    }

    pub fn active_ready(&self) -> bool {
        *self.active().ready.read()
    }

    pub fn inactive_ready(&self) -> bool {
        *self.inactive().ready.read()
    }

    pub fn is_transitioning(&self) -> bool {
        *self.transitioning.read()
    }

    pub fn reset(&mut self) {
        self.primary.clear();
        self.secondary.clear();
        self.active_slot.set(VideoSlot::Primary);
        self.primary.opacity.set(1.0);
        self.transitioning.set(false);
    }

    pub fn load_next_video(&mut self, url: Option<String>) {
        if self.is_transitioning() {
            return;
        }
        self.inactive_mut().load(url);
    }

    pub fn mark_ready(&mut self, slot: VideoSlot) {
        self.get_slot_mut(slot).ready.set(true);
    }

    pub fn start_transition(&mut self) {
        self.transitioning.set(true);
    }

    pub fn swap_and_clear(&mut self) {
        let old_active = *self.active_slot.read();
        self.active_slot.set(old_active.other());
        self.get_slot_mut(old_active).clear();
    }

    pub fn update_crossfade(&mut self, progress: f64) {
        let active = *self.active_slot.read();
        let (fade_out, fade_in) = match active {
            VideoSlot::Primary => (&mut self.primary, &mut self.secondary),
            VideoSlot::Secondary => (&mut self.secondary, &mut self.primary),
        };
        
        fade_out.opacity.set(1.0 - progress);
        fade_in.opacity.set(progress);
    }

    pub fn finish_transition(&mut self) {
        let active = *self.active_slot.read();
        let (active_slot, inactive_slot) = match active {
            VideoSlot::Primary => (&mut self.primary, &mut self.secondary),
            VideoSlot::Secondary => (&mut self.secondary, &mut self.primary),
        };
        
        active_slot.opacity.set(1.0);
        inactive_slot.opacity.set(0.0);
        self.transitioning.set(false);
    }

    pub fn primary_url(&self) -> Signal<Option<String>> {
        self.primary.url
    }

    pub fn secondary_url(&self) -> Signal<Option<String>> {
        self.secondary.url
    }

    pub fn primary_opacity(&self) -> Signal<f64> {
        self.primary.opacity
    }

    pub fn secondary_opacity(&self) -> Signal<f64> {
        self.secondary.opacity
    }
}
