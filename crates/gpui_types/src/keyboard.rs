use collections::HashMap;
use std::rc::Rc;

use crate::{KeybindingKeystroke, Keystroke};

/// A trait for platform-specific keyboard layouts
pub trait PlatformKeyboardLayoutSpi {
    /// Get the keyboard layout ID, which should be unique to the layout
    fn id(&self) -> &str;
    /// Get the keyboard layout display name
    fn name(&self) -> &str;
}

/// A trait for platform-specific keyboard mappings
pub trait PlatformKeyboardMapperSpi {
    /// Map a key equivalent to its platform-specific representation
    fn map_key_equivalent(
        &self,
        keystroke: Keystroke,
        use_key_equivalents: bool,
    ) -> KeybindingKeystroke;
    /// Get the key equivalents for the current keyboard layout,
    /// only used on macOS
    fn get_key_equivalents(&self) -> Option<&HashMap<char, char>>;
}

/// Keyboard operations supplied by a platform implementation.
pub trait PlatformKeyboardSpi {
    /// Returns the current keyboard layout.
    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayoutSpi>;

    /// Returns the current keyboard mapper.
    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapperSpi>;

    /// Registers a callback invoked when the keyboard layout changes.
    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>);
}

/// A dummy implementation of the platform keyboard mapper
pub struct DummyKeyboardMapper;

impl PlatformKeyboardMapperSpi for DummyKeyboardMapper {
    fn map_key_equivalent(
        &self,
        keystroke: Keystroke,
        _use_key_equivalents: bool,
    ) -> KeybindingKeystroke {
        KeybindingKeystroke::from_keystroke(keystroke)
    }

    fn get_key_equivalents(&self) -> Option<&HashMap<char, char>> {
        None
    }
}
