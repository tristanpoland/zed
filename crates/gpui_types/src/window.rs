//! Backend-neutral window configuration values.

use crate::geometry::{Pixels, Point};
use gpui_shared_string::SharedString;

/// A type to describe whether a window uses server- or client-side decorations.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
pub enum WindowDecorations {
    /// Server-side decorations.
    #[default]
    Server,
    /// Client-side decorations.
    Client,
}

/// A window control button supported by a titlebar layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowButton {
    /// The minimize button.
    Minimize,
    /// The maximize button.
    Maximize,
    /// The close button.
    Close,
}

impl WindowButton {
    /// Returns the stable identifier used by the platform layout format.
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
            Self::Close => "close",
        }
    }

    const fn index(&self) -> usize {
        match self {
            Self::Minimize => 0,
            Self::Maximize => 1,
            Self::Close => 2,
        }
    }
}

/// Maximum number of titlebar buttons on either side.
pub const MAX_BUTTONS_PER_SIDE: usize = 3;

/// Describes which buttons appear on each side of a titlebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowButtonLayout {
    /// Buttons on the left side of the titlebar.
    pub left: [Option<WindowButton>; MAX_BUTTONS_PER_SIDE],
    /// Buttons on the right side of the titlebar.
    pub right: [Option<WindowButton>; MAX_BUTTONS_PER_SIDE],
}

impl WindowButtonLayout {
    /// Returns GPUI's built-in fallback layout for Linux titlebars.
    pub const fn linux_default() -> Self {
        Self {
            left: [None; MAX_BUTTONS_PER_SIDE],
            right: [
                Some(WindowButton::Minimize),
                Some(WindowButton::Maximize),
                Some(WindowButton::Close),
            ],
        }
    }

    /// Parses a GNOME-style `button-layout` string.
    pub fn parse(layout_string: &str) -> anyhow::Result<Self> {
        fn parse_side(
            side: &str,
            seen_buttons: &mut [bool; MAX_BUTTONS_PER_SIDE],
            unrecognized: &mut Vec<String>,
        ) -> [Option<WindowButton>; MAX_BUTTONS_PER_SIDE] {
            let mut result = [None; MAX_BUTTONS_PER_SIDE];
            let mut index = 0;
            for name in side.split(',') {
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                let Some(button) = (match name {
                    "minimize" => Some(WindowButton::Minimize),
                    "maximize" => Some(WindowButton::Maximize),
                    "close" => Some(WindowButton::Close),
                    other => {
                        unrecognized.push(other.to_string());
                        None
                    }
                }) else {
                    continue;
                };
                if seen_buttons[button.index()] {
                    continue;
                }
                if let Some(slot) = result.get_mut(index) {
                    *slot = Some(button);
                    seen_buttons[button.index()] = true;
                    index += 1;
                }
            }
            result
        }

        let (left, right) = layout_string.split_once(':').unwrap_or(("", layout_string));
        let mut unrecognized = Vec::new();
        let mut seen_buttons = [false; MAX_BUTTONS_PER_SIDE];
        let layout = Self {
            left: parse_side(left, &mut seen_buttons, &mut unrecognized),
            right: parse_side(right, &mut seen_buttons, &mut unrecognized),
        };

        if !unrecognized.is_empty()
            && layout.left.iter().all(Option::is_none)
            && layout.right.iter().all(Option::is_none)
        {
            anyhow::bail!(
                "button layout string {:?} contains no valid buttons (unrecognized: {})",
                layout_string,
                unrecognized.join(", ")
            );
        }

        Ok(layout)
    }

    /// Formats the layout as a GNOME-style `button-layout` string.
    pub fn format(&self) -> String {
        fn format_side(buttons: &[Option<WindowButton>; MAX_BUTTONS_PER_SIDE]) -> String {
            buttons
                .iter()
                .flatten()
                .map(|button| button.id())
                .collect::<Vec<_>>()
                .join(",")
        }

        format!("{}:{}", format_side(&self.left), format_side(&self.right))
    }
}

/// Options that can be configured for a window's titlebar.
#[derive(Debug, Default)]
pub struct TitlebarOptions {
    /// The initial title of the window.
    pub title: Option<SharedString>,
    /// Whether the default system titlebar should be hidden.
    pub appears_transparent: bool,
    /// The position of the macOS traffic-light buttons.
    pub traffic_light_position: Option<Point<Pixels>>,
}

/// The appearance of a window as defined by the operating system.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowAppearance {
    /// A light appearance.
    #[default]
    Light,
    /// A light appearance with vibrant colors.
    VibrantLight,
    /// A dark appearance.
    Dark,
    /// A dark appearance with vibrant colors.
    VibrantDark,
}

/// The appearance of a window's background when its content is transparent.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowBackgroundAppearance {
    /// Opaque.
    #[default]
    Opaque,
    /// Plain alpha transparency.
    Transparent,
    /// Transparency with a blurred backdrop.
    Blurred,
    /// The Windows 11 Mica backdrop material.
    MicaBackdrop,
    /// The Windows 11 Mica Alt backdrop material.
    MicaAltBackdrop,
}
