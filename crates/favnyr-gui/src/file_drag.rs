//! File-transfer intent shared by internal drags and Windows OLE drops.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum DropAction {
    Choose = -1,
    Move = 0,
    Copy = 1,
}

impl DropAction {
    pub fn from_modifiers(control: bool, shift: bool) -> Self {
        // Preserve the existing Ctrl behavior when both modifiers are held.
        if control {
            Self::Copy
        } else if shift {
            Self::Move
        } else {
            Self::Choose
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_modifiers_override_the_action_menu() {
        assert_eq!(DropAction::from_modifiers(false, false), DropAction::Choose);
        assert_eq!(DropAction::from_modifiers(false, true), DropAction::Move);
        assert_eq!(DropAction::from_modifiers(true, false), DropAction::Copy);
        assert_eq!(DropAction::from_modifiers(true, true), DropAction::Copy);
    }
}
