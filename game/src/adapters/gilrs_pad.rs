//! The controller through gilrs (XInput and Windows.Gaming.Input controllers): the first one
//! connected plays; its events are read once per frame. While the window is not in front the
//! controller is still followed but reads as nothing pressed.
use crate::domain::pad::PadButton;
use crate::ports::Gamepad;
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use std::collections::HashSet;

/// How far a stick goes to count as pressed, and back to count as let go.
const STICK_ON: f32 = 0.5;
const STICK_OFF: f32 = 0.3;

pub struct GilrsPad {
    gilrs: Option<Gilrs>,
    active: Option<GamepadId>,
    held: HashSet<PadButton>,
    pressed: HashSet<PadButton>,
    last: Option<PadButton>,
    in_front: bool,
}

fn button(b: Button) -> Option<PadButton> {
    use PadButton as P;
    Some(match b {
        Button::South => P::South,
        Button::East => P::East,
        Button::West => P::West,
        Button::North => P::North,
        Button::LeftTrigger => P::LeftShoulder,
        Button::RightTrigger => P::RightShoulder,
        Button::LeftTrigger2 => P::LeftTrigger,
        Button::RightTrigger2 => P::RightTrigger,
        Button::Select => P::Select,
        Button::Start => P::Start,
        Button::Mode => P::Guide,
        Button::LeftThumb => P::LeftThumb,
        Button::RightThumb => P::RightThumb,
        Button::DPadUp => P::DPadUp,
        Button::DPadDown => P::DPadDown,
        Button::DPadLeft => P::DPadLeft,
        Button::DPadRight => P::DPadRight,
        _ => return None,
    })
}

impl GilrsPad {
    /// No controller support at all when the system's controller services cannot be opened.
    pub fn new() -> GilrsPad {
        let gilrs = Gilrs::new().ok();
        let active = gilrs.as_ref().and_then(|g| g.gamepads().map(|(id, _)| id).next());
        GilrsPad { gilrs, active, held: HashSet::new(), pressed: HashSet::new(), last: None, in_front: true }
    }

    /// Read the events since the last frame; `in_front`: the window takes the input.
    pub fn poll(&mut self, in_front: bool) {
        self.in_front = in_front;
        self.pressed.clear();
        self.last = None;
        let Some(g) = &mut self.gilrs else { return };
        let mut events = Vec::new();
        while let Some(e) = g.next_event() {
            events.push(e);
        }
        for e in events {
            match e.event {
                EventType::Connected if self.active.is_none() => self.active = Some(e.id),
                EventType::Disconnected if self.active == Some(e.id) => {
                    self.held.clear();
                    self.active = self.gilrs.as_ref().and_then(|g| g.gamepads().map(|(id, _)| id).find(|&id| id != e.id));
                }
                _ if self.active != Some(e.id) => {}
                EventType::ButtonPressed(b, _) => {
                    if let Some(p) = button(b) {
                        self.press(p);
                    }
                }
                EventType::ButtonReleased(b, _) => {
                    if let Some(p) = button(b) {
                        self.held.remove(&p);
                    }
                }
                EventType::AxisChanged(axis, v, _) => match axis {
                    Axis::LeftStickX => self.stick(v, PadButton::StickRight, PadButton::StickLeft),
                    Axis::LeftStickY => self.stick(v, PadButton::StickUp, PadButton::StickDown),
                    Axis::DPadX => self.stick(v, PadButton::DPadRight, PadButton::DPadLeft),
                    Axis::DPadY => self.stick(v, PadButton::DPadUp, PadButton::DPadDown),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn press(&mut self, p: PadButton) {
        if self.held.insert(p) {
            self.pressed.insert(p);
            self.last = Some(p);
        }
    }

    /// An axis as two buttons: `plus` past `STICK_ON`, `minus` below `-STICK_ON`.
    fn stick(&mut self, v: f32, plus: PadButton, minus: PadButton) {
        for (b, on) in [(plus, v), (minus, -v)] {
            if on >= STICK_ON {
                self.press(b);
            } else if on < STICK_OFF {
                self.held.remove(&b);
            }
        }
    }
}

impl Gamepad for GilrsPad {
    fn connected(&self) -> bool {
        self.active.is_some()
    }
    fn pressed(&self, b: PadButton) -> bool {
        self.in_front && self.pressed.contains(&b)
    }
    fn held(&self, b: PadButton) -> bool {
        self.in_front && self.held.contains(&b)
    }
    fn last_pressed(&self) -> Option<PadButton> {
        self.last.filter(|_| self.in_front)
    }
}
