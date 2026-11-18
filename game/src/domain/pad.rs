//! A controller (gamepad) as the game knows it, independent of any library: its buttons by
//! place (the face buttons named as on an Xbox controller), the left stick's four directions,
//! their ids in `settings.cfg` and labels in the menu, and which input plays.
use super::keys::{self, KEYS, PAD_KEYS};
use crate::ports::Gamepad;
use gb_core::Buttons;

/// What plays the game.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Input {
    #[default]
    Keyboard,
    Controller,
}

impl Input {
    pub fn id(self) -> &'static str {
        match self {
            Input::Keyboard => "keyboard",
            Input::Controller => "controller",
        }
    }

    pub fn from_id(id: &str) -> Option<Input> {
        [Input::Keyboard, Input::Controller].into_iter().find(|i| i.id() == id)
    }
}

macro_rules! pad_buttons {
    ($($b:ident $id:literal $label:literal;)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        pub enum PadButton {
            $($b,)*
            /// Not bindable: opens the pause menu anywhere.
            Guide,
        }

        /// Buttons that can be bound: button, file id, menu label.
        pub const PAD_BINDABLE: &[(PadButton, &str, &str)] = &[$((PadButton::$b, $id, $label),)*];
    };
}

pad_buttons! {
    South "South" "A";
    East "East" "B";
    West "West" "X";
    North "North" "Y";
    LeftShoulder "LeftShoulder" "LB";
    RightShoulder "RightShoulder" "RB";
    LeftTrigger "LeftTrigger" "LT";
    RightTrigger "RightTrigger" "RT";
    Select "Select" "BACK";
    Start "Start" "START";
    LeftThumb "LeftThumb" "L3";
    RightThumb "RightThumb" "R3";
    DPadUp "DPadUp" "PAD UP";
    DPadDown "DPadDown" "PAD DOWN";
    DPadLeft "DPadLeft" "PAD LEFT";
    DPadRight "DPadRight" "PAD RIGHT";
    StickUp "StickUp" "LS UP";
    StickDown "StickDown" "LS DOWN";
    StickLeft "StickLeft" "LS LEFT";
    StickRight "StickRight" "LS RIGHT";
}

/// The controller's bindings by place on a Game Boy: the D-pad, the right face button for A
/// and the bottom one for B, Start, the left face button for Select, the shoulders for save
/// and load state, the left trigger to rewind, the right one to fast-forward, and Select
/// (Back, View, Share) for the pause menu.
pub const DEFAULT_PAD: [PadButton; KEYS] = {
    use PadButton::*;
    [DPadUp, DPadDown, DPadLeft, DPadRight, East, South, Start, West, LeftShoulder, RightShoulder, LeftTrigger, RightTrigger, Select]
};

/// The left stick's directions, in `keys` order (Up, Down, Left, Right).
const STICK: [PadButton; 4] = [PadButton::StickUp, PadButton::StickDown, PadButton::StickLeft, PadButton::StickRight];

impl PadButton {
    pub fn bindable(self) -> bool {
        PAD_BINDABLE.iter().any(|e| e.0 == self)
    }

    pub fn id(self) -> Option<&'static str> {
        PAD_BINDABLE.iter().find(|e| e.0 == self).map(|e| e.1)
    }

    pub fn from_id(id: &str) -> Option<PadButton> {
        PAD_BINDABLE.iter().find(|e| e.1 == id).map(|e| e.0)
    }

    /// The name shown in the menu (the same in every language).
    pub fn label(self) -> &'static str {
        PAD_BINDABLE.iter().find(|e| e.0 == self).map_or("GUIDE", |e| e.2)
    }
}

/// The Game Boy buttons held on the controller. The left stick moves like the D-pad unless
/// one of its directions is bound to something.
pub fn pad_buttons(pad: &dyn Gamepad, bindings: &[PadButton; KEYS]) -> Buttons {
    let mut b = Buttons::NONE;
    for (i, &flag) in keys::PAD_BUTTONS.iter().enumerate().take(PAD_KEYS) {
        if pad.held(bindings[i]) {
            b |= flag;
        }
    }
    if !STICK.iter().any(|s| bindings.contains(s)) {
        for (i, &s) in STICK.iter().enumerate() {
            if pad.held(s) {
                b |= keys::PAD_BUTTONS[i];
            }
        }
    }
    keys::no_opposites(b)
}

/// The left stick's direction `i` (in `keys` order) as an alias of the D-pad for moving in
/// the menu.
pub fn stick(i: usize) -> PadButton {
    STICK[i]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Held(Vec<PadButton>);
    impl Gamepad for Held {
        fn connected(&self) -> bool {
            true
        }
        fn pressed(&self, b: PadButton) -> bool {
            self.0.contains(&b)
        }
        fn held(&self, b: PadButton) -> bool {
            self.0.contains(&b)
        }
        fn last_pressed(&self) -> Option<PadButton> {
            self.0.last().copied()
        }
    }

    #[test]
    fn ids_and_labels() {
        for &(b, id, label) in PAD_BINDABLE {
            assert_eq!(PadButton::from_id(id), Some(b));
            assert_eq!(b.label(), label);
        }
        assert!(!PadButton::Guide.bindable());
        assert_eq!(Input::from_id("controller"), Some(Input::Controller));
    }

    #[test]
    fn the_defaults_play_and_the_stick_walks() {
        let b = pad_buttons(&Held(vec![PadButton::East, PadButton::StickRight]), &DEFAULT_PAD);
        assert_eq!(b, Buttons::A | Buttons::RIGHT);
        let mut bound = DEFAULT_PAD;
        bound[keys::A] = PadButton::StickUp;
        assert_eq!(pad_buttons(&Held(vec![PadButton::StickRight]), &bound), Buttons::NONE, "a bound stick direction stops the stick walking");
    }
}
