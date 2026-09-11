//! Backend-neutral window configuration values.

use crate::{
    geometry::{Bounds, Pixels, Point, Size},
    input::{Capslock, Modifiers},
    platform::DisplayId,
};
use gpui_shared_string::SharedString;
use std::{
    any::TypeId,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};
use uuid::Uuid;

slotmap::new_key_type! {
    /// A unique identifier for a window.
    pub struct WindowId;
}

impl WindowId {
    /// Converts this window ID to a `u64`.
    pub fn as_u64(&self) -> u64 {
        self.0.as_ffi()
    }
}

impl From<u64> for WindowId {
    fn from(value: u64) -> Self {
        WindowId(slotmap::KeyData::from_ffi(value))
    }
}

/// A handle to a window with any root view type.
///
/// This handle identifies a window but does not keep it alive. Operations that
/// access the window's state are implemented by the GPUI backend.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct AnyWindowHandle {
    id: WindowId,
    state_type: TypeId,
    root_entity_type_name: &'static str,
}

impl AnyWindowHandle {
    /// Get the ID of this window.
    pub fn window_id(&self) -> WindowId {
        self.id
    }

    /// Returns the name of the window's declared root entity type.
    pub fn root_entity_type_name(&self) -> &'static str {
        self.root_entity_type_name
    }

    /// Attempt to convert this handle to a window handle with a specific root view type.
    /// If the types do not match, this will return `None`.
    pub fn downcast<T: 'static>(&self) -> Option<WindowHandle<T>> {
        if TypeId::of::<T>() == self.state_type {
            Some(WindowHandle {
                any_handle: *self,
                state_type: PhantomData,
            })
        } else {
            None
        }
    }

    fn new<V: 'static>(id: WindowId) -> Self {
        Self {
            id,
            state_type: TypeId::of::<V>(),
            root_entity_type_name: std::any::type_name::<V>(),
        }
    }
}

/// A handle to a window with a specific root view type.
///
/// Note that this does not keep the window alive on its own. Operations that
/// access the window's state are implemented by the GPUI backend.
pub struct WindowHandle<V> {
    any_handle: AnyWindowHandle,
    state_type: PhantomData<fn(V) -> V>,
}

impl<V> Debug for WindowHandle<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowHandle")
            .field("any_handle", &self.any_handle.id.as_u64())
            .finish()
    }
}

impl<V: 'static> WindowHandle<V> {
    /// Creates a new handle from a window ID.
    /// This does not check if the root type of the window is `V`.
    pub fn new(id: WindowId) -> Self {
        Self {
            any_handle: AnyWindowHandle::new::<V>(id),
            state_type: PhantomData,
        }
    }

    /// Gets the root view out of this window.
    #[cfg(any(test, feature = "test-support"))]
    pub fn root<C>(&self, cx: &mut C) -> C::WindowResult<C::Entity<V>>
    where
        C: crate::AppContextWindow<
            AnyWindowHandle = AnyWindowHandle,
            WindowHandle<V> = WindowHandle<V>,
        >,
    {
        cx.spi_read_window(self, |root_view, _| root_view)
    }

    /// Updates the root view of this window.
    pub fn update<C, R>(
        &self,
        cx: &mut C,
        update: impl FnOnce(
            &mut V,
            &mut C::Window,
            &mut C::WindowContext<'_, V>,
        ) -> R,
    ) -> C::WindowResult<R>
    where
        C: crate::AppContextWindow<
            AnyWindowHandle = AnyWindowHandle,
            WindowHandle<V> = WindowHandle<V>,
        >,
    {
        cx.spi_update_window_entity(self, update)
    }

    /// Reads the root view of this window.
    pub fn read<'a, C>(&self, cx: &'a C) -> C::WindowResult<&'a V>
    where
        C: crate::WindowRootReadSpi<
            AnyWindowHandle = AnyWindowHandle,
            WindowHandle<V> = WindowHandle<V>,
        >,
    {
        cx.spi_read_window_root(self)
    }

    /// Reads the root view of this window with a callback.
    pub fn read_with<C, R>(
        &self,
        cx: &C,
        read_with: impl FnOnce(&V, &C::App) -> R,
    ) -> C::WindowResult<R>
    where
        C: crate::AppContextWindow<
            AnyWindowHandle = AnyWindowHandle,
            WindowHandle<V> = WindowHandle<V>,
        >,
    {
        cx.spi_read_window_root_with(self, read_with)
    }

    /// Reads the root entity of this window.
    pub fn entity<C>(&self, cx: &C) -> C::WindowResult<C::Entity<V>>
    where
        C: crate::AppContextWindow<
            AnyWindowHandle = AnyWindowHandle,
            WindowHandle<V> = WindowHandle<V>,
        >,
    {
        cx.spi_read_window(self, |root_view, _| root_view)
    }

    /// Checks if this window is active.
    pub fn is_active<C>(&self, cx: &mut C) -> Option<bool>
    where
        C: crate::AppContextWindow<AnyWindowHandle = AnyWindowHandle>,
    {
        cx.spi_window_is_active((*self).into())
    }
}

impl<V> Deref for WindowHandle<V> {
    type Target = AnyWindowHandle;

    fn deref(&self) -> &Self::Target {
        &self.any_handle
    }
}

impl<V> DerefMut for WindowHandle<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.any_handle
    }
}

impl<V> Copy for WindowHandle<V> {}

impl<V> Clone for WindowHandle<V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V> PartialEq for WindowHandle<V> {
    fn eq(&self, other: &Self) -> bool {
        self.any_handle == other.any_handle
    }
}

impl<V> Eq for WindowHandle<V> {}

impl<V> Hash for WindowHandle<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.any_handle.hash(state);
    }
}

impl<V: 'static> From<WindowHandle<V>> for AnyWindowHandle {
    fn from(value: WindowHandle<V>) -> Self {
        value.any_handle
    }
}

impl AnyWindowHandle {
    /// Updates the state of the root view of this window.
    pub fn update<C, R>(
        self,
        cx: &mut C,
        update: impl FnOnce(C::AnyView, &mut C::Window, &mut C::App) -> R,
    ) -> C::WindowResult<R>
    where
        C: crate::AppContextWindow<AnyWindowHandle = AnyWindowHandle>,
    {
        cx.spi_update_window(self, update)
    }

    /// Reads the state of the root view of this window.
    pub fn read<T, C, R>(
        self,
        cx: &C,
        read: impl FnOnce(C::Entity<T>, &C::App) -> R,
    ) -> C::WindowResult<R>
    where
        C: crate::AppContextWindow<AnyWindowHandle = AnyWindowHandle>,
        T: 'static,
    {
        cx.spi_read_window_any(self, read)
    }
}

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

/// The display capabilities supplied by a platform backend.
pub trait PlatformDisplaySpi: Debug {
    /// Returns the display identifier.
    fn id(&self) -> DisplayId;

    /// Returns a stable identifier that can be persisted across restarts.
    fn uuid(&self) -> anyhow::Result<Uuid>;

    /// Returns the display bounds.
    fn bounds(&self) -> Bounds<Pixels>;

    /// Returns the usable bounds, excluding taskbar or dock areas.
    fn visible_bounds(&self) -> Bounds<Pixels> {
        self.bounds()
    }

    /// Returns the default bounds for placing a window on this display.
    fn default_bounds(&self) -> Bounds<Pixels> {
        let bounds = self.bounds();
        let center = Point {
            x: bounds.origin.x + bounds.size.width / 2.0,
            y: bounds.origin.y + bounds.size.height / 2.0,
        };
        let clipped_window_size = Size {
            width: Pixels(1536.0).min(bounds.size.width),
            height: Pixels(1095.0).min(bounds.size.height),
        };
        let offset = Size {
            width: clipped_window_size.width / 2.0,
            height: clipped_window_size.height / 2.0,
        };
        Bounds::new(
            point(center.x - offset.width, center.y - offset.height),
            clipped_window_size,
        )
    }
}

/// Platform-level display and window-management capabilities.
pub trait PlatformWindowingSpi {
    /// The display object returned by this platform.
    type Display: PlatformDisplaySpi + ?Sized;

    /// Returns all displays currently connected to the platform.
    fn displays(&self) -> Vec<Rc<Self::Display>>;

    /// Returns the display the platform considers primary.
    fn primary_display(&self) -> Option<Rc<Self::Display>>;

    /// Returns the appearance of the application's windows.
    fn window_appearance(&self) -> WindowAppearance;

    /// Overrides the appearance applied to the application's windows.
    fn set_window_appearance(&self, appearance: Option<WindowAppearance>);

    /// Returns the supported titlebar button layout, when available.
    fn button_layout(&self) -> Option<WindowButtonLayout>;
}

/// Backend-neutral window capabilities used by GPUI's window runtime.
pub trait PlatformWindowSpi {
    /// The display object returned by this window.
    type Display: PlatformDisplaySpi + ?Sized;

    /// Returns the outer window bounds.
    fn bounds(&self) -> Bounds<Pixels>;

    /// Returns whether the window is maximized.
    fn is_maximized(&self) -> bool;

    /// Returns the content size.
    fn content_size(&self) -> Size<Pixels>;

    /// Returns the visible viewport relative to the content origin.
    fn visual_viewport_bounds(&self) -> Bounds<Pixels> {
        Bounds::new(Point::default(), self.content_size())
    }

    /// Registers a callback for changes to the visible viewport.
    fn on_visual_viewport_changed(&self, _callback: Box<dyn FnMut()>) {}

    /// Samples platform geometry before a draw and reports whether caches changed.
    fn prepare_frame(&self) -> bool {
        false
    }

    /// Resizes the window's content.
    fn resize(&mut self, size: Size<Pixels>);

    /// Returns the window scale factor.
    fn scale_factor(&self) -> f32;

    /// Returns the current window appearance.
    fn appearance(&self) -> WindowAppearance;

    /// Returns the display containing this window.
    fn display(&self) -> Option<Rc<Self::Display>>;

    /// Returns the current mouse position in window coordinates.
    fn mouse_position(&self) -> Point<Pixels>;

    /// Returns the current keyboard modifiers.
    fn modifiers(&self) -> Modifiers;

    /// Returns the current caps-lock state.
    fn capslock(&self) -> Capslock;

    /// Activates the window.
    fn activate(&self);

    /// Requests that the operating system draw attention to the window.
    fn request_attention(&self) {}

    /// Returns whether the window is active.
    fn is_active(&self) -> bool;

    /// Returns whether the pointer is over the window.
    fn is_hovered(&self) -> bool;

    /// Returns the current background appearance.
    fn background_appearance(&self) -> WindowBackgroundAppearance;

    /// Sets the window title.
    fn set_title(&mut self, title: &str);

    /// Sets the window background appearance.
    fn set_background_appearance(&self, background_appearance: WindowBackgroundAppearance);

    /// Minimizes the window.
    fn minimize(&self);

    /// Zooms or maximizes the window.
    fn zoom(&self);

    /// Toggles fullscreen mode.
    fn toggle_fullscreen(&self);

    /// Returns whether the window is fullscreen.
    fn is_fullscreen(&self) -> bool;

    /// Returns a callback that wakes the frame scheduler, when available.
    fn frame_waker(&self) -> Option<Rc<dyn Fn()>> {
        None
    }

    /// Registers a callback for frame requests.
    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>);

    /// Registers a callback for active-state changes.
    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>);

    /// Registers a callback for hover-state changes.
    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>);

    /// Registers a callback for resize events.
    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>);

    /// Registers a callback for move events.
    fn on_moved(&self, callback: Box<dyn FnMut()>);

    /// Registers a callback that can veto closing the window.
    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>);

    /// Registers a callback invoked after the window closes.
    fn on_close(&self, callback: Box<dyn FnOnce()>);

    /// Registers a callback for appearance changes.
    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>);

    /// Registers a callback for titlebar button-layout changes.
    fn on_button_layout_changed(&self, _callback: Box<dyn FnMut()>) {}

    /// Requests a frame from the platform.
    fn schedule_frame(&self) {}

    /// Returns whether subpixel text rendering is supported.
    fn is_subpixel_rendering_supported(&self) -> bool;

    /// Requests that the soft keyboard be shown.
    fn show_soft_keyboard(&self) {}

    /// Requests that the soft keyboard be hidden.
    fn hide_soft_keyboard(&self) {}

    /// Plays the platform system bell.
    fn play_system_bell(&self) {}
}

/// Options passed to a frame-request callback.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct RequestFrameOptions {
    /// Whether presentation is required.
    pub require_presentation: bool,
    /// Whether all rendering state must be refreshed.
    pub force_render: bool,
}

fn point(x: Pixels, y: Pixels) -> Point<Pixels> {
    Point { x, y }
}
