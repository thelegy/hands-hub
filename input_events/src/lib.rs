#![no_std]

use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/input_event_codes.rs"));

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum InputEventKind {
    SynEvent(Syn),
    KeyEvent(Key),
    RelEvent(RelAxis),
    UnknownEvent,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InputEvent {
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

impl InputEvent {
    pub const SYN_REPORT: Self = Self {
        type_: 0,
        code: 0,
        value: 0,
    };
    pub fn kind(&self) -> InputEventKind {
        match EventType::from(self.type_) {
            EventType::EvSyn => InputEventKind::SynEvent(self.code.into()),
            EventType::EvKey => InputEventKind::KeyEvent(self.code.into()),
            EventType::EvRel => InputEventKind::RelEvent(self.code.into()),
            _ => InputEventKind::UnknownEvent,
        }
    }
}

#[cfg(feature = "evdev")]
impl From<&evdev::InputEvent> for InputEvent {
    fn from(value: &evdev::InputEvent) -> Self {
        Self {
            type_: value.event_type().0,
            code: value.code(),
            value: value.value(),
        }
    }
}

#[cfg(feature = "evdev")]
impl From<&InputEvent> for evdev::InputEvent {
    fn from(value: &InputEvent) -> Self {
        evdev::InputEvent::new(evdev::EventType(value.type_), value.code, value.value)
    }
}

#[cfg(test)]
mod tests {}
