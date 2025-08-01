//! The [`Window`] trait and associated types.
use std::fmt;

use bitflags::bitflags;
use cursor_icon::CursorIcon;
use dpi::{
    LogicalPosition, LogicalSize, PhysicalInsets, PhysicalPosition, PhysicalSize, Position, Size,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::as_any::AsAny;
use crate::cursor::Cursor;
use crate::error::RequestError;
use crate::icon::Icon;
use crate::monitor::{Fullscreen, MonitorHandle};

/// Identifier of a window. Unique for each window.
///
/// Can be obtained with [`window.id()`][`Window::id`].
///
/// Whenever you receive an event specific to a window, this event contains a `WindowId` which you
/// can then compare to the ids of your windows.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(usize);

impl WindowId {
    /// Convert the `WindowId` into the underlying integer.
    ///
    /// This is useful if you need to pass the ID across an FFI boundary, or store it in an atomic.
    pub const fn into_raw(self) -> usize {
        self.0
    }

    /// Construct a `WindowId` from the underlying integer.
    ///
    /// This should only be called with integers returned from [`WindowId::into_raw`].
    pub const fn from_raw(id: usize) -> Self {
        Self(id)
    }
}

impl fmt::Debug for WindowId {
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(fmtr)
    }
}

/// Attributes used when creating a window.
#[derive(Debug)]
#[non_exhaustive]
pub struct WindowAttributes {
    pub surface_size: Option<Size>,
    pub min_surface_size: Option<Size>,
    pub max_surface_size: Option<Size>,
    pub surface_resize_increments: Option<Size>,
    pub position: Option<Position>,
    pub resizable: bool,
    pub enabled_buttons: WindowButtons,
    pub title: String,
    pub maximized: bool,
    pub visible: bool,
    pub transparent: bool,
    pub blur: bool,
    pub decorations: bool,
    pub window_icon: Option<Icon>,
    pub preferred_theme: Option<Theme>,
    pub content_protected: bool,
    pub window_level: WindowLevel,
    pub active: bool,
    pub cursor: Cursor,
    pub(crate) parent_window: Option<SendSyncRawWindowHandle>,
    pub fullscreen: Option<Fullscreen>,
    pub platform: Option<Box<dyn PlatformWindowAttributes>>,
}

impl WindowAttributes {
    /// Get the parent window stored on the attributes.
    pub fn parent_window(&self) -> Option<&rwh_06::RawWindowHandle> {
        self.parent_window.as_ref().map(|handle| &handle.0)
    }

    /// Requests the surface to be of specific dimensions.
    ///
    /// If this is not set, some platform-specific dimensions will be used.
    ///
    /// See [`Window::request_surface_size`] for details.
    #[inline]
    pub fn with_surface_size<S: Into<Size>>(mut self, size: S) -> Self {
        self.surface_size = Some(size.into());
        self
    }

    /// Sets the minimum dimensions the surface can have.
    ///
    /// If this is not set, the surface will have no minimum dimensions (aside from reserved).
    ///
    /// See [`Window::set_min_surface_size`] for details.
    #[inline]
    pub fn with_min_surface_size<S: Into<Size>>(mut self, min_size: S) -> Self {
        self.min_surface_size = Some(min_size.into());
        self
    }

    /// Sets the maximum dimensions the surface can have.
    ///
    /// If this is not set, the surface will have no maximum, or the maximum will be restricted to
    /// the primary monitor's dimensions by the platform.
    ///
    /// See [`Window::set_max_surface_size`] for details.
    #[inline]
    pub fn with_max_surface_size<S: Into<Size>>(mut self, max_size: S) -> Self {
        self.max_surface_size = Some(max_size.into());
        self
    }

    /// Build window with resize increments hint.
    ///
    /// The default is `None`.
    ///
    /// See [`Window::set_surface_resize_increments`] for details.
    #[inline]
    pub fn with_surface_resize_increments<S: Into<Size>>(
        mut self,
        surface_resize_increments: S,
    ) -> Self {
        self.surface_resize_increments = Some(surface_resize_increments.into());
        self
    }

    /// Sets a desired initial position for the window.
    ///
    /// If this is not set, some platform-specific position will be chosen.
    ///
    /// See [`Window::set_outer_position`] for details.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** The top left corner position of the window content, the window's "inner"
    ///   position. The window title bar will be placed above it. The window will be positioned such
    ///   that it fits on screen, maintaining set `surface_size` if any. If you need to precisely
    ///   position the top left corner of the whole window you have to use
    ///   [`Window::set_outer_position`] after creating the window.
    /// - **Windows:** The top left corner position of the window title bar, the window's "outer"
    ///   position. There may be a small gap between this position and the window due to the
    ///   specifics of the Window Manager.
    /// - **X11:** The top left corner of the window, the window's "outer" position.
    /// - **Others:** Ignored.
    #[inline]
    pub fn with_position<P: Into<Position>>(mut self, position: P) -> Self {
        self.position = Some(position.into());
        self
    }

    /// Sets whether the window is resizable or not.
    ///
    /// The default is `true`.
    ///
    /// See [`Window::set_resizable`] for details.
    #[inline]
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Sets the enabled window buttons.
    ///
    /// The default is [`WindowButtons::all`]
    ///
    /// See [`Window::set_enabled_buttons`] for details.
    #[inline]
    pub fn with_enabled_buttons(mut self, buttons: WindowButtons) -> Self {
        self.enabled_buttons = buttons;
        self
    }

    /// Sets the initial title of the window in the title bar.
    ///
    /// The default is `"winit window"`.
    ///
    /// See [`Window::set_title`] for details.
    #[inline]
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = title.into();
        self
    }

    /// Sets whether the window should be put into fullscreen upon creation.
    ///
    /// The default is `None`.
    ///
    /// See [`Window::set_fullscreen`] for details.
    #[inline]
    pub fn with_fullscreen(mut self, fullscreen: Option<Fullscreen>) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// Request that the window is maximized upon creation.
    ///
    /// The default is `false`.
    ///
    /// See [`Window::set_maximized`] for details.
    #[inline]
    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Sets whether the window will be initially visible or hidden.
    ///
    /// The default is to show the window.
    ///
    /// See [`Window::set_visible`] for details.
    #[inline]
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sets whether the background of the window should be transparent.
    ///
    /// If this is `true`, writing colors with alpha values different than
    /// `1.0` will produce a transparent window. On some platforms this
    /// is more of a hint for the system and you'd still have the alpha
    /// buffer. To control it see [`Window::set_transparent`].
    ///
    /// The default is `false`.
    #[inline]
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Sets whether the background of the window should be blurred by the system.
    ///
    /// The default is `false`.
    ///
    /// See [`Window::set_blur`] for details.
    #[inline]
    pub fn with_blur(mut self, blur: bool) -> Self {
        self.blur = blur;
        self
    }

    /// Get whether the window will support transparency.
    #[inline]
    pub fn transparent(&self) -> bool {
        self.transparent
    }

    /// Sets whether the window should have a border, a title bar, etc.
    ///
    /// The default is `true`.
    ///
    /// See [`Window::set_decorations`] for details.
    #[inline]
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    /// Sets the window level.
    ///
    /// This is just a hint to the OS, and the system could ignore it.
    ///
    /// The default is [`WindowLevel::Normal`].
    ///
    /// See [`WindowLevel`] for details.
    #[inline]
    pub fn with_window_level(mut self, level: WindowLevel) -> Self {
        self.window_level = level;
        self
    }

    /// Sets the window icon.
    ///
    /// The default is `None`.
    ///
    /// See [`Window::set_window_icon`] for details.
    #[inline]
    pub fn with_window_icon(mut self, window_icon: Option<Icon>) -> Self {
        self.window_icon = window_icon;
        self
    }

    /// Sets a specific theme for the window.
    ///
    /// If `None` is provided, the window will use the system theme.
    ///
    /// The default is `None`.
    ///
    /// ## Platform-specific
    ///
    /// - **Wayland:** This controls only CSD. When using `None` it'll try to use dbus to get the
    ///   system preference. When explicit theme is used, this will avoid dbus all together.
    /// - **x11:** Build window with `_GTK_THEME_VARIANT` hint set to `dark` or `light`.
    /// - **iOS / Android / Web / x11 / Orbital:** Ignored.
    #[inline]
    pub fn with_theme(mut self, theme: Option<Theme>) -> Self {
        self.preferred_theme = theme;
        self
    }

    /// Prevents the window contents from being captured by other apps.
    ///
    /// The default is `false`.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS**: if `false`, [`NSWindowSharingNone`] is used but doesn't completely prevent all
    ///   apps from reading the window content, for instance, QuickTime.
    /// - **iOS / Android / Web / x11 / Orbital:** Ignored.
    ///
    /// [`NSWindowSharingNone`]: https://developer.apple.com/documentation/appkit/nswindowsharingtype/nswindowsharingnone
    #[inline]
    pub fn with_content_protected(mut self, protected: bool) -> Self {
        self.content_protected = protected;
        self
    }

    /// Whether the window will be initially focused or not.
    ///
    /// The window should be assumed as not focused by default
    /// following by the [`WindowEvent::Focused`].
    ///
    /// ## Platform-specific:
    ///
    /// **Android / iOS / X11 / Wayland / Orbital:** Unsupported.
    ///
    /// [`WindowEvent::Focused`]: crate::event::WindowEvent::Focused.
    #[inline]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Modifies the cursor icon of the window.
    ///
    /// The default is [`CursorIcon::Default`].
    ///
    /// See [`Window::set_cursor()`] for more details.
    #[inline]
    pub fn with_cursor(mut self, cursor: impl Into<Cursor>) -> Self {
        self.cursor = cursor.into();
        self
    }

    /// Build window with parent window.
    ///
    /// The default is `None`.
    ///
    /// ## Safety
    ///
    /// `parent_window` must be a valid window handle.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows** : A child window has the WS_CHILD style and is confined
    ///   to the client area of its parent window. For more information, see
    ///   <https://docs.microsoft.com/en-us/windows/win32/winmsg/window-features#child-windows>
    /// - **X11**: A child window is confined to the client area of its parent window.
    /// - **Android / iOS / Wayland / Web:** Unsupported.
    #[inline]
    pub unsafe fn with_parent_window(
        mut self,
        parent_window: Option<rwh_06::RawWindowHandle>,
    ) -> Self {
        self.parent_window = parent_window.map(SendSyncRawWindowHandle);
        self
    }

    /// Set the platform specific opaque attribute object.
    ///
    /// The interpretation will depend on the underlying backend that will be used.
    #[inline]
    pub fn with_platform_attributes(mut self, platform: Box<dyn PlatformWindowAttributes>) -> Self {
        self.platform = Some(platform);
        self
    }
}

impl Clone for WindowAttributes {
    fn clone(&self) -> Self {
        Self {
            surface_size: self.surface_size,
            min_surface_size: self.min_surface_size,
            max_surface_size: self.max_surface_size,
            surface_resize_increments: self.surface_resize_increments,
            position: self.position,
            resizable: self.resizable,
            enabled_buttons: self.enabled_buttons,
            title: self.title.clone(),
            maximized: self.maximized,
            visible: self.visible,
            transparent: self.transparent,
            blur: self.blur,
            decorations: self.decorations,
            window_icon: self.window_icon.clone(),
            preferred_theme: self.preferred_theme,
            content_protected: self.content_protected,
            window_level: self.window_level,
            active: self.active,
            cursor: self.cursor.clone(),
            parent_window: self.parent_window.clone(),
            fullscreen: self.fullscreen.clone(),
            platform: self.platform.as_ref().map(|platform| platform.box_clone()),
        }
    }
}

impl Default for WindowAttributes {
    #[inline]
    fn default() -> WindowAttributes {
        WindowAttributes {
            enabled_buttons: WindowButtons::all(),
            title: String::from("winit window"),
            decorations: true,
            resizable: true,
            visible: true,
            active: true,
            surface_resize_increments: Default::default(),
            content_protected: Default::default(),
            min_surface_size: Default::default(),
            max_surface_size: Default::default(),
            preferred_theme: Default::default(),
            parent_window: Default::default(),
            surface_size: Default::default(),
            window_level: Default::default(),
            window_icon: Default::default(),
            transparent: Default::default(),
            fullscreen: Default::default(),
            maximized: Default::default(),
            position: Default::default(),
            platform: Default::default(),
            cursor: Cursor::default(),
            blur: Default::default(),
        }
    }
}

/// Wrapper for [`rwh_06::RawWindowHandle`] for [`WindowAttributes::parent_window`].
///
/// # Safety
///
/// The user has to account for that when using [`WindowAttributes::with_parent_window()`],
/// which is `unsafe`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SendSyncRawWindowHandle(pub(crate) rwh_06::RawWindowHandle);

unsafe impl Send for SendSyncRawWindowHandle {}
unsafe impl Sync for SendSyncRawWindowHandle {}

pub trait PlatformWindowAttributes: AsAny + std::fmt::Debug + Send + Sync {
    fn box_clone(&self) -> Box<dyn PlatformWindowAttributes>;
}

impl_dyn_casting!(PlatformWindowAttributes);

/// Represents a window.
///
/// The window is closed when dropped.
///
/// ## Threading
///
/// This is `Send + Sync`, meaning that it can be freely used from other
/// threads.
///
/// However, some platforms (macOS, Web and iOS) only allow user interface
/// interactions on the main thread, so on those platforms, if you use the
/// window from a thread other than the main, the code is scheduled to run on
/// the main thread, and your thread may be blocked until that completes.
///
/// ## Platform-specific
///
/// **Web:** The [`Window`], which is represented by a `HTMLElementCanvas`, can
/// not be closed by dropping the [`Window`].
pub trait Window: AsAny + Send + Sync + fmt::Debug {
    /// Returns an identifier unique to the window.
    fn id(&self) -> WindowId;

    /// Returns the scale factor that can be used to map logical pixels to physical pixels, and
    /// vice versa.
    ///
    /// Note that this value can change depending on user action (for example if the window is
    /// moved to another screen); as such, tracking [`WindowEvent::ScaleFactorChanged`] events is
    /// the most robust way to track the DPI you need to use to draw.
    ///
    /// This value may differ from [`MonitorHandleProvider::scale_factor`].
    ///
    /// See the [`dpi`] crate for more information.
    ///
    /// ## Platform-specific
    ///
    /// The scale factor is calculated differently on different platforms:
    ///
    /// - **Windows:** On Windows 8 and 10, per-monitor scaling is readily configured by users from
    ///   the display settings. While users are free to select any option they want, they're only
    ///   given a selection of "nice" scale factors, i.e. 1.0, 1.25, 1.5... on Windows 7. The scale
    ///   factor is global and changing it requires logging out. See [this article][windows_1] for
    ///   technical details.
    /// - **macOS:** Recent macOS versions allow the user to change the scaling factor for specific
    ///   displays. When available, the user may pick a per-monitor scaling factor from a set of
    ///   pre-defined settings. All "retina displays" have a scaling factor above 1.0 by default,
    ///   but the specific value varies across devices.
    /// - **X11:** Many man-hours have been spent trying to figure out how to handle DPI in X11.
    ///   Winit currently uses a three-pronged approach:
    ///   + Use the value in the `WINIT_X11_SCALE_FACTOR` environment variable if present.
    ///   + If not present, use the value set in `Xft.dpi` in Xresources.
    ///   + Otherwise, calculate the scale factor based on the millimeter monitor dimensions
    ///     provided by XRandR.
    ///
    ///   If `WINIT_X11_SCALE_FACTOR` is set to `randr`, it'll ignore the `Xft.dpi` field and use
    ///   the   XRandR scaling method. Generally speaking, you should try to configure the
    ///   standard system   variables to do what you want before resorting to
    ///   `WINIT_X11_SCALE_FACTOR`.
    /// - **Wayland:** The scale factor is suggested by the compositor for each window individually
    ///   by using the wp-fractional-scale protocol if available. Falls back to integer-scale
    ///   factors otherwise.
    ///
    ///   The monitor scale factor may differ from the window scale factor.
    /// - **iOS:** Scale factors are set by Apple to the value that best suits the device, and range
    ///   from `1.0` to `3.0`. See [this article][apple_1] and [this article][apple_2] for more
    ///   information.
    ///
    ///   This uses the underlying `UIView`'s [`contentScaleFactor`].
    /// - **Android:** Scale factors are set by the manufacturer to the value that best suits the
    ///   device, and range from `1.0` to `4.0`. See [this article][android_1] for more information.
    ///
    ///   This is currently unimplemented, and this function always returns 1.0.
    /// - **Web:** The scale factor is the ratio between CSS pixels and the physical device pixels.
    ///   In other words, it is the value of [`window.devicePixelRatio`][web_1]. It is affected by
    ///   both the screen scaling and the browser zoom level and can go below `1.0`.
    /// - **Orbital:** This is currently unimplemented, and this function always returns 1.0.
    ///
    /// [`WindowEvent::ScaleFactorChanged`]: crate::event::WindowEvent::ScaleFactorChanged
    /// [windows_1]: https://docs.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows
    /// [apple_1]: https://developer.apple.com/library/archive/documentation/DeviceInformation/Reference/iOSDeviceCompatibility/Displays/Displays.html
    /// [apple_2]: https://developer.apple.com/design/human-interface-guidelines/macos/icons-and-images/image-size-and-resolution/
    /// [android_1]: https://developer.android.com/training/multiscreen/screendensities
    /// [web_1]: https://developer.mozilla.org/en-US/docs/Web/API/Window/devicePixelRatio
    /// [`contentScaleFactor`]: https://developer.apple.com/documentation/uikit/uiview/1622657-contentscalefactor?language=objc
    /// [`MonitorHandleProvider::scale_factor`]: crate::monitor::MonitorHandleProvider::scale_factor.
    fn scale_factor(&self) -> f64;

    /// Queues a [`WindowEvent::RedrawRequested`] event to be emitted that aligns with the windowing
    /// system drawing loop.
    ///
    /// This is the **strongly encouraged** method of redrawing windows, as it can integrate with
    /// OS-requested redraws (e.g. when a window gets resized). To improve the event delivery
    /// consider using [`Window::pre_present_notify`] as described in docs.
    ///
    /// Applications should always aim to redraw whenever they receive a `RedrawRequested` event.
    ///
    /// There are no strong guarantees about when exactly a `RedrawRequest` event will be emitted
    /// with respect to other events, since the requirements can vary significantly between
    /// windowing systems.
    ///
    /// However as the event aligns with the windowing system drawing loop, it may not arrive in
    /// same or even next event loop iteration.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows** This API uses `RedrawWindow` to request a `WM_PAINT` message and
    ///   `RedrawRequested` is emitted in sync with any `WM_PAINT` messages.
    /// - **Wayland:** The events are aligned with the frame callbacks when
    ///   [`Window::pre_present_notify`] is used.
    /// - **Web:** [`WindowEvent::RedrawRequested`] will be aligned with the
    ///   `requestAnimationFrame`.
    ///
    /// [`WindowEvent::RedrawRequested`]: crate::event::WindowEvent::RedrawRequested
    fn request_redraw(&self);

    /// Notify the windowing system before presenting to the window.
    ///
    /// You should call this event after your drawing operations, but before you submit
    /// the buffer to the display or commit your drawings. Doing so will help winit to properly
    /// schedule and make assumptions about its internal state. For example, it could properly
    /// throttle [`WindowEvent::RedrawRequested`].
    ///
    /// ## Example
    ///
    /// This example illustrates how it looks with OpenGL, but it applies to other graphics
    /// APIs and software rendering.
    ///
    /// ```no_run
    /// # use winit_core::window::Window;
    /// # fn swap_buffers() {}
    /// # fn scope(window: &dyn Window) {
    /// // Do the actual drawing with OpenGL.
    ///
    /// // Notify winit that we're about to submit buffer to the windowing system.
    /// window.pre_present_notify();
    ///
    /// // Submit buffer to the windowing system.
    /// swap_buffers();
    /// # }
    /// ```
    ///
    /// ## Platform-specific
    ///
    /// - **Android / iOS / X11 / Web / Windows / macOS / Orbital:** Unsupported.
    /// - **Wayland:** Schedules a frame callback to throttle [`WindowEvent::RedrawRequested`].
    ///
    /// [`WindowEvent::RedrawRequested`]: crate::event::WindowEvent::RedrawRequested
    fn pre_present_notify(&self);

    /// Reset the dead key state of the keyboard.
    ///
    /// This is useful when a dead key is bound to trigger an action. Then
    /// this function can be called to reset the dead key state so that
    /// follow-up text input won't be affected by the dead key.
    ///
    /// ## Platform-specific
    /// - **Web, macOS:** Does nothing
    // ---------------------------
    // Developers' Note: If this cannot be implemented on every desktop platform
    // at least, then this function should be provided through a platform specific
    // extension trait
    fn reset_dead_keys(&self);

    /// The position of the top-left hand corner of the surface relative to the top-left hand corner
    /// of the window.
    ///
    /// This, combined with [`outer_position`], can be useful for calculating the position of the
    /// surface relative to the desktop.
    ///
    /// This may also be useful for figuring out the size of the window's decorations (such as
    /// buttons, title, etc.), but may also not correspond to that (e.g. if the title bar is made
    /// transparent on macOS, or your are drawing window
    /// decorations yourself).
    ///
    /// This may be negative.
    ///
    /// If the window does not have any decorations, and the surface is in the exact same position
    /// as the window itself, this simply returns `(0, 0)`.
    ///
    /// [`outer_position`]: Self::outer_position
    fn surface_position(&self) -> PhysicalPosition<i32>;

    /// The position of the top-left hand corner of the window relative to the top-left hand corner
    /// of the desktop.
    ///
    /// Note that the top-left hand corner of the desktop is not necessarily the same as
    /// the screen. If the user uses a desktop with multiple monitors, the top-left hand corner
    /// of the desktop is the top-left hand corner of the primary monitor of the desktop.
    ///
    /// The coordinates can be negative if the top-left hand corner of the window is outside
    /// of the visible screen region, or on another monitor than the primary.
    ///
    /// ## Platform-specific
    ///
    /// - **Web:** Returns the top-left coordinates relative to the viewport.
    /// - **Android / Wayland:** Always returns [`RequestError::NotSupported`].
    fn outer_position(&self) -> Result<PhysicalPosition<i32>, RequestError>;

    /// Sets the position of the window on the desktop.
    ///
    /// See [`Window::outer_position`] for more information about the coordinates.
    /// This automatically un-maximizes the window if it's maximized.
    ///
    /// ```no_run
    /// # use dpi::{LogicalPosition, PhysicalPosition};
    /// # use winit_core::window::Window;
    /// # fn scope(window: &dyn Window) {
    /// // Specify the position in logical dimensions like this:
    /// window.set_outer_position(LogicalPosition::new(400.0, 200.0).into());
    ///
    /// // Or specify the position in physical dimensions like this:
    /// window.set_outer_position(PhysicalPosition::new(400, 200).into());
    /// # }
    /// ```
    ///
    /// ## Platform-specific
    ///
    /// - **iOS:** Sets the top left coordinates of the window in the screen space coordinate
    ///   system.
    /// - **Web:** Sets the top-left coordinates relative to the viewport. Doesn't account for CSS
    ///   [`transform`].
    /// - **Android / Wayland:** Unsupported.
    ///
    /// [`transform`]: https://developer.mozilla.org/en-US/docs/Web/CSS/transform
    fn set_outer_position(&self, position: Position);

    /// Returns the size of the window's render-able surface.
    ///
    /// This is the dimensions you should pass to things like Wgpu or Glutin when configuring the
    /// surface for drawing. See [`WindowEvent::SurfaceResized`] for listening to changes to this
    /// field.
    ///
    /// Note that to ensure that your content is not obscured by things such as notches or the title
    /// bar, you will likely want to only draw important content inside a specific area of the
    /// surface, see [`safe_area()`] for details.
    ///
    /// ## Platform-specific
    ///
    /// - **Web:** Returns the size of the canvas element. Doesn't account for CSS [`transform`].
    ///
    /// [`transform`]: https://developer.mozilla.org/en-US/docs/Web/CSS/transform
    /// [`WindowEvent::SurfaceResized`]: crate::event::WindowEvent::SurfaceResized
    /// [`safe_area()`]: Window::safe_area
    fn surface_size(&self) -> PhysicalSize<u32>;

    /// Request the new size for the surface.
    ///
    /// On platforms where the size is entirely controlled by the user the
    /// applied size will be returned immediately, resize event in such case
    /// may not be generated.
    ///
    /// On platforms where resizing is disallowed by the windowing system, the current surface size
    /// is returned immediately, and the user one is ignored.
    ///
    /// When `None` is returned, it means that the request went to the display system,
    /// and the actual size will be delivered later with the [`WindowEvent::SurfaceResized`].
    ///
    /// See [`Window::surface_size`] for more information about the values.
    ///
    /// The request could automatically un-maximize the window if it's maximized.
    ///
    /// ```no_run
    /// # use dpi::{LogicalSize, PhysicalSize};
    /// # use winit_core::window::Window;
    /// # fn scope(window: &dyn Window) {
    /// // Specify the size in logical dimensions like this:
    /// let _ = window.request_surface_size(LogicalSize::new(400.0, 200.0).into());
    ///
    /// // Or specify the size in physical dimensions like this:
    /// let _ = window.request_surface_size(PhysicalSize::new(400, 200).into());
    /// # }
    /// ```
    ///
    /// ## Platform-specific
    ///
    /// - **Web:** Sets the size of the canvas element. Doesn't account for CSS [`transform`].
    ///
    /// [`WindowEvent::SurfaceResized`]: crate::event::WindowEvent::SurfaceResized
    /// [`transform`]: https://developer.mozilla.org/en-US/docs/Web/CSS/transform
    #[must_use]
    fn request_surface_size(&self, size: Size) -> Option<PhysicalSize<u32>>;

    /// Returns the size of the entire window.
    ///
    /// These dimensions include window decorations like the title bar and borders. If you don't
    /// want that (and you usually don't), use [`Window::surface_size`] instead.
    ///
    /// ## Platform-specific
    ///
    /// - **Web:** Returns the size of the canvas element. _Note: this returns the same value as
    ///   [`Window::surface_size`]._
    fn outer_size(&self) -> PhysicalSize<u32>;

    /// The inset area of the surface that is unobstructed.
    ///
    /// On some devices, especially mobile devices, the screen is not a perfect rectangle, and may
    /// have rounded corners, notches, bezels, and so on. When drawing your content, you usually
    /// want to draw your background and other such unimportant content on the entire surface, while
    /// you will want to restrict important content such as text, interactable or visual indicators
    /// to the part of the screen that is actually visible; for this, you use the safe area.
    ///
    /// The safe area is a rectangle that is defined relative to the origin at the top-left corner
    /// of the surface, and the size extending downwards to the right. The area will not extend
    /// beyond [the bounds of the surface][Window::surface_size].
    ///
    /// Note that the safe area does not take occlusion from other windows into account; in a way,
    /// it is only a "hardware"-level occlusion.
    ///
    /// If the entire content of the surface is visible, this returns `(0, 0, 0, 0)`.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / Orbital / Wayland / Windows / X11:** Unimplemented, returns `(0, 0, 0, 0)`.
    ///
    /// ## Example
    ///
    /// Convert safe area insets to a size and a position.
    ///
    /// ```
    /// use dpi::{PhysicalPosition, PhysicalSize};
    ///
    /// # let surface_size = dpi::PhysicalSize::new(0, 0);
    /// # #[cfg(requires_window)]
    /// let surface_size = window.surface_size();
    /// # let insets = dpi::PhysicalInsets::new(0, 0, 0, 0);
    /// # #[cfg(requires_window)]
    /// let insets = window.safe_area();
    ///
    /// let origin = PhysicalPosition::new(insets.left, insets.top);
    /// let size = PhysicalSize::new(
    ///     surface_size.width - insets.left - insets.right,
    ///     surface_size.height - insets.top - insets.bottom,
    /// );
    /// ```
    fn safe_area(&self) -> PhysicalInsets<u32>;

    /// Sets a minimum dimensions of the window's surface.
    ///
    /// ```no_run
    /// # use dpi::{LogicalSize, PhysicalSize};
    /// # use winit_core::window::Window;
    /// # fn scope(window: &dyn Window) {
    /// // Specify the size in logical dimensions like this:
    /// window.set_min_surface_size(Some(LogicalSize::new(400.0, 200.0).into()));
    ///
    /// // Or specify the size in physical dimensions like this:
    /// window.set_min_surface_size(Some(PhysicalSize::new(400, 200).into()));
    /// # }
    /// ```
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Orbital:** Unsupported.
    fn set_min_surface_size(&self, min_size: Option<Size>);

    /// Sets a maximum dimensions of the window's surface.
    ///
    /// ```no_run
    /// # use dpi::{LogicalSize, PhysicalSize};
    /// # use winit_core::window::Window;
    /// # fn scope(window: &dyn Window) {
    /// // Specify the size in logical dimensions like this:
    /// window.set_max_surface_size(Some(LogicalSize::new(400.0, 200.0).into()));
    ///
    /// // Or specify the size in physical dimensions like this:
    /// window.set_max_surface_size(Some(PhysicalSize::new(400, 200).into()));
    /// # }
    /// ```
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Orbital:** Unsupported.
    fn set_max_surface_size(&self, max_size: Option<Size>);

    /// Returns surface resize increments if any were set.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Wayland / Orbital:** Always returns [`None`].
    fn surface_resize_increments(&self) -> Option<PhysicalSize<u32>>;

    /// Sets resize increments of the surface.
    ///
    /// This is a niche constraint hint usually employed by terminal emulators and other such apps
    /// that need "blocky" resizes.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Increments are converted to logical size and then macOS rounds them to whole
    ///   numbers.
    /// - **Wayland:** Not implemented.
    /// - **iOS / Android / Web / Orbital:** Unsupported.
    fn set_surface_resize_increments(&self, increments: Option<Size>);

    /// Modifies the title of the window.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android:** Unsupported.
    fn set_title(&self, title: &str);

    /// Change the window transparency state.
    ///
    /// This is just a hint that may not change anything about
    /// the window transparency, however doing a mismatch between
    /// the content of your window and this hint may result in
    /// visual artifacts.
    ///
    /// The default value follows the [`WindowAttributes::with_transparent`].
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** This will reset the window's background color.
    /// - **Web / iOS / Android:** Unsupported.
    /// - **X11:** Can only be set while building the window, with
    ///   [`WindowAttributes::with_transparent`].
    fn set_transparent(&self, transparent: bool);

    /// Change the window blur state.
    ///
    /// If `true`, this will make the transparent window background blurry.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / iOS / X11 / Web / Windows:** Unsupported.
    /// - **Wayland:** Only works with org_kde_kwin_blur_manager protocol.
    fn set_blur(&self, blur: bool);

    /// Modifies the window's visibility.
    ///
    /// If `false`, this will hide the window. If `true`, this will show the window.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / Wayland / Web:** Unsupported.
    fn set_visible(&self, visible: bool);

    /// Gets the window's current visibility state.
    ///
    /// `None` means it couldn't be determined, so it is not recommended to use this to drive your
    /// rendering backend.
    ///
    /// ## Platform-specific
    ///
    /// - **X11:** Not implemented.
    /// - **Wayland / iOS / Android / Web:** Unsupported.
    fn is_visible(&self) -> Option<bool>;

    /// Sets whether the window is resizable or not.
    ///
    /// Note that making the window unresizable doesn't exempt you from handling
    /// [`WindowEvent::SurfaceResized`], as that event can still be triggered by DPI scaling,
    /// entering fullscreen mode, etc. Also, the window could still be resized by calling
    /// [`Window::request_surface_size`].
    ///
    /// ## Platform-specific
    ///
    /// This only has an effect on desktop platforms.
    ///
    /// - **X11:** Due to a bug in XFCE, this has no effect on Xfwm.
    /// - **iOS / Android / Web:** Unsupported.
    ///
    /// [`WindowEvent::SurfaceResized`]: crate::event::WindowEvent::SurfaceResized
    fn set_resizable(&self, resizable: bool);

    /// Gets the window's current resizable state.
    ///
    /// ## Platform-specific
    ///
    /// - **X11:** Not implemented.
    /// - **iOS / Android / Web:** Unsupported.
    fn is_resizable(&self) -> bool;

    /// Sets the enabled window buttons.
    ///
    /// ## Platform-specific
    ///
    /// - **Wayland / X11 / Orbital:** Not implemented.
    /// - **Web / iOS / Android:** Unsupported.
    fn set_enabled_buttons(&self, buttons: WindowButtons);

    /// Gets the enabled window buttons.
    ///
    /// ## Platform-specific
    ///
    /// - **Wayland / X11 / Orbital:** Not implemented. Always returns [`WindowButtons::all`].
    /// - **Web / iOS / Android:** Unsupported. Always returns [`WindowButtons::all`].
    fn enabled_buttons(&self) -> WindowButtons;

    /// Minimize the window, or put it back from the minimized state.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Orbital:** Unsupported.
    /// - **Wayland:** Un-minimize is unsupported.
    fn set_minimized(&self, minimized: bool);

    /// Gets the window's current minimized state.
    ///
    /// `None` will be returned, if the minimized state couldn't be determined.
    ///
    /// ## Note
    ///
    /// - You shouldn't stop rendering for minimized windows, however you could lower the fps.
    ///
    /// ## Platform-specific
    ///
    /// - **Wayland**: always `None`.
    /// - **iOS / Android / Web / Orbital:** Unsupported.
    fn is_minimized(&self) -> Option<bool>;

    /// Sets the window to maximized or back.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web:** Unsupported.
    fn set_maximized(&self, maximized: bool);

    /// Gets the window's current maximized state.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web:** Unsupported.
    fn is_maximized(&self) -> bool;

    /// Set the window's fullscreen state.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** [`Fullscreen::Exclusive`] provides true exclusive mode with a video mode
    ///   change. *Caveat!* macOS doesn't provide task switching (or spaces!) while in exclusive
    ///   fullscreen mode. This mode should be used when a video mode change is desired, but for a
    ///   better user experience, borderless fullscreen might be preferred.
    ///
    ///   [`Fullscreen::Borderless`] provides a borderless fullscreen window on a
    ///   separate space. This is the idiomatic way for fullscreen games to work
    ///   on macOS. See `WindowExtMacOs::set_simple_fullscreen` if
    ///   separate spaces are not preferred.
    ///
    ///   The dock and the menu bar are disabled in exclusive fullscreen mode.
    /// - **Wayland:** Does not support exclusive fullscreen mode and will no-op a request.
    /// - **Windows:** Screen saver is disabled in fullscreen mode.
    /// - **Web:** Passing a [`MonitorHandle`] or [`VideoMode`] that was not created with detailed
    ///   monitor permissions or calling without a [transient activation] does nothing.
    ///
    /// [transient activation]: https://developer.mozilla.org/en-US/docs/Glossary/Transient_activation
    /// [`VideoMode`]: crate::monitor::VideoMode
    fn set_fullscreen(&self, fullscreen: Option<Fullscreen>);

    /// Gets the window's current fullscreen state.
    ///
    /// ## Platform-specific
    ///
    /// - **Android / Orbital:** Will always return `None`.
    /// - **Wayland:** Can return `Borderless(None)` when there are no monitors.
    /// - **Web:** Can only return `None` or `Borderless(None)`.
    fn fullscreen(&self) -> Option<Fullscreen>;

    /// Turn window decorations on or off.
    ///
    /// Enable/disable window decorations provided by the server or Winit.
    /// By default this is enabled. Note that fullscreen windows and windows on
    /// mobile and Web platforms naturally do not have decorations.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web:** No effect.
    fn set_decorations(&self, decorations: bool);

    /// Gets the window's current decorations state.
    ///
    /// Returns `true` when windows are decorated (server-side or by Winit).
    /// Also returns `true` when no decorations are required (mobile, Web).
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web:** Always returns `true`.
    fn is_decorated(&self) -> bool;

    /// Change the window level.
    ///
    /// This is just a hint to the OS, and the system could ignore it.
    ///
    /// See [`WindowLevel`] for details.
    fn set_window_level(&self, level: WindowLevel);

    /// Sets the window icon.
    ///
    /// On Windows, Wayland and X11, this is typically the small icon in the top-left
    /// corner of the titlebar.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / / macOS / Orbital:** Unsupported.
    ///
    /// - **Windows:** Sets `ICON_SMALL`. The base size for a window icon is 16x16, but it's
    ///   recommended to account for screen scaling and pick a multiple of that, i.e. 32x32.
    ///
    /// - **X11:** Has no universal guidelines for icon sizes, so you're at the whims of the WM.
    ///   That said, it's usually in the same ballpark as on Windows.
    ///
    /// - **Wayland:** The compositor needs to implement `xdg_toplevel_icon`.
    fn set_window_icon(&self, window_icon: Option<Icon>);

    /// Set the IME cursor editing area, where the `position` is the top left corner of that area
    /// in surface coordinates and `size` is the size of this area starting from the position. An
    /// example of such area could be a input field in the UI or line in the editor.
    ///
    /// The windowing system could place a candidate box close to that area, but try to not obscure
    /// the specified area, so the user input to it stays visible.
    ///
    /// The candidate box is the window / popup / overlay that allows you to select the desired
    /// characters. The look of this box may differ between input devices, even on the same
    /// platform.
    ///
    /// (Apple's official term is "candidate window", see their [chinese] and [japanese] guides).
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use dpi::{LogicalPosition, PhysicalPosition, LogicalSize, PhysicalSize};
    /// # use winit_core::window::Window;
    /// # fn scope(window: &dyn Window) {
    /// // Specify the position in logical dimensions like this:
    /// window.set_ime_cursor_area(
    ///     LogicalPosition::new(400.0, 200.0).into(),
    ///     LogicalSize::new(100, 100).into(),
    /// );
    ///
    /// // Or specify the position in physical dimensions like this:
    /// window.set_ime_cursor_area(
    ///     PhysicalPosition::new(400, 200).into(),
    ///     PhysicalSize::new(100, 100).into(),
    /// );
    /// # }
    /// ```
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Orbital:** Unsupported.
    ///
    /// [chinese]: https://support.apple.com/guide/chinese-input-method/use-the-candidate-window-cim12992/104/mac/12.0
    /// [japanese]: https://support.apple.com/guide/japanese-input-method/use-the-candidate-window-jpim10262/6.3/mac/12.0
    #[deprecated = "use Window::request_ime_update instead"]
    fn set_ime_cursor_area(&self, position: Position, size: Size) {
        if self.ime_capabilities().map(|caps| caps.cursor_area()).unwrap_or(false) {
            let _ = self.request_ime_update(ImeRequest::Update(
                ImeRequestData::default().with_cursor_area(position, size),
            ));
        }
    }

    /// Sets whether the window should get IME events
    ///
    /// When IME is allowed, the window will receive [`Ime`] events, and during the
    /// preedit phase the window will NOT get [`KeyboardInput`] events. The window
    /// should allow IME when it is expecting text input.
    ///
    /// When IME is not allowed, the window won't receive [`Ime`] events, and will
    /// receive [`KeyboardInput`] events for every keypress instead. Not allowing
    /// IME is useful for games for example.
    ///
    /// IME is **not** allowed by default.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** IME must be enabled to receive text-input where dead-key sequences are
    ///   combined.
    /// - **iOS / Android:** This will show / hide the soft keyboard.
    /// - **Web / Orbital:** Unsupported.
    /// - **X11**: Enabling IME will disable dead keys reporting during compose.
    ///
    /// [`Ime`]: crate::event::WindowEvent::Ime
    /// [`KeyboardInput`]: crate::event::WindowEvent::KeyboardInput
    #[deprecated = "use Window::request_ime_update instead"]
    fn set_ime_allowed(&self, allowed: bool) {
        let action = if allowed {
            let position = LogicalPosition::new(0, 0);
            let size = LogicalSize::new(0, 0);
            let ime_caps = ImeCapabilities::new().with_purpose().with_cursor_area();
            let request_data = ImeRequestData {
                purpose: Some(ImePurpose::Normal),
                // WARNING: there's nothing sensible to use here by default.
                cursor_area: Some((position.into(), size.into())),
                ..ImeRequestData::default()
            };

            // Enable all capabilities to reflect the old behavior.
            ImeRequest::Enable(ImeEnableRequest::new(ime_caps, request_data).unwrap())
        } else {
            ImeRequest::Disable
        };

        let _ = self.request_ime_update(action);
    }

    /// Sets the IME purpose for the window using [`ImePurpose`].
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Windows / X11 / macOS / Orbital:** Unsupported.
    #[deprecated = "use Window::request_ime_update instead"]
    fn set_ime_purpose(&self, purpose: ImePurpose) {
        if self.ime_capabilities().map(|caps| caps.purpose()).unwrap_or(false) {
            let _ = self.request_ime_update(ImeRequest::Update(ImeRequestData {
                purpose: Some(purpose),
                ..ImeRequestData::default()
            }));
        }
    }

    /// Atomically apply request to IME.
    ///
    /// For details consult [`ImeRequest`] and [`ImeCapabilities`].
    ///
    /// Input methods allows the user to compose text without using a keyboard. Requesting one may
    /// be beneficial for touch screen environments or ones where, for example, East Asian scripts
    /// may be entered.
    ///
    /// If the focus within the application changes from one logical text input area to another, the
    /// application should inform the IME of the switch by disabling the IME and enabling it again
    /// in the other area.
    ///
    /// IME is **not** enabled by default.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use dpi::{Position, Size};
    /// # use winit_core::window::{Window, ImePurpose, ImeRequest, ImeCapabilities, ImeRequestData, ImeEnableRequest};
    /// # fn scope(window: &dyn Window, cursor_pos: Position, cursor_size: Size) {
    /// // Clear previous state by switching off IME
    /// window.request_ime_update(ImeRequest::Disable).expect("Disable cannot fail");
    ///
    /// let ime_caps = ImeCapabilities::new().with_cursor_area().with_purpose();
    /// let request_data = ImeRequestData::default()
    ///                          .with_purpose(ImePurpose::Normal)
    ///                          .with_cursor_area(cursor_pos, cursor_size);
    /// let enable_ime = ImeEnableRequest::new(ime_caps, request_data.clone()).unwrap();
    /// window.request_ime_update(ImeRequest::Enable(enable_ime)).expect("Enabling may fail if IME is not supported");
    ///
    /// // Update the current state
    /// window
    ///     .request_ime_update(ImeRequest::Update(request_data.clone()))
    ///     .expect("will fail if it's not enabled or ime is not supported");
    ///
    /// // Update the current state
    /// window
    ///     .request_ime_update(ImeRequest::Update(
    ///        request_data.with_cursor_area(cursor_pos, cursor_size),
    ///     ))
    ///     .expect("Can fail - we didn't submit a cursor position initially");
    ///
    /// // Switch off IME
    /// window.request_ime_update(ImeRequest::Disable).expect("Disable cannot fail");
    /// # }
    /// ```
    fn request_ime_update(&self, request: ImeRequest) -> Result<(), ImeRequestError>;

    /// Return enabled by the client [`ImeCapabilities`] for this window.
    ///
    /// When the IME is not yet enabled it'll return `None`.
    ///
    /// By default IME is disabled, thus will return `None`.
    fn ime_capabilities(&self) -> Option<ImeCapabilities>;

    /// Brings the window to the front and sets input focus. Has no effect if the window is
    /// already in focus, minimized, or not visible.
    ///
    /// This method steals input focus from other applications. Do not use this method unless
    /// you are certain that's what the user wants. Focus stealing can cause an extremely disruptive
    /// user experience.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Wayland / Orbital:** Unsupported.
    fn focus_window(&self);

    /// Gets whether the window has keyboard focus.
    ///
    /// This queries the same state information as [`WindowEvent::Focused`].
    ///
    /// [`WindowEvent::Focused`]: crate::event::WindowEvent::Focused
    fn has_focus(&self) -> bool;

    /// Requests user attention to the window, this has no effect if the application
    /// is already focused. How requesting for user attention manifests is platform dependent,
    /// see [`UserAttentionType`] for details.
    ///
    /// Providing `None` will unset the request for user attention. Unsetting the request for
    /// user attention might not be done automatically by the WM when the window receives input.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Orbital:** Unsupported.
    /// - **macOS:** `None` has no effect.
    /// - **X11:** Requests for user attention must be manually cleared.
    /// - **Wayland:** Requires `xdg_activation_v1` protocol, `None` has no effect.
    fn request_user_attention(&self, request_type: Option<UserAttentionType>);

    /// Set or override the window theme.
    ///
    /// Specify `None` to reset the theme to the system default.
    ///
    /// ## Platform-specific
    ///
    /// - **Wayland:** Sets the theme for the client side decorations. Using `None` will use dbus to
    ///   get the system preference.
    /// - **X11:** Sets `_GTK_THEME_VARIANT` hint to `dark` or `light` and if `None` is used, it
    ///   will default to  [`Theme::Dark`].
    /// - **iOS / Android / Web / Orbital:** Unsupported.
    fn set_theme(&self, theme: Option<Theme>);

    /// Returns the current window theme.
    ///
    /// Returns `None` if it cannot be determined on the current platform.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / x11 / Orbital:** Unsupported.
    /// - **Wayland:** Only returns theme overrides.
    fn theme(&self) -> Option<Theme>;

    /// Prevents the window contents from being captured by other apps.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS**: if `false`, [`NSWindowSharingNone`] is used but doesn't completely prevent all
    ///   apps from reading the window content, for instance, QuickTime.
    /// - **iOS / Android / x11 / Wayland / Web / Orbital:** Unsupported.
    ///
    /// [`NSWindowSharingNone`]: https://developer.apple.com/documentation/appkit/nswindowsharingtype/nswindowsharingnone
    fn set_content_protected(&self, protected: bool);

    /// Gets the current title of the window.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / x11 / Wayland / Web:** Unsupported. Always returns an empty string.
    fn title(&self) -> String;

    /// Modifies the cursor icon of the window.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Orbital:** Unsupported.
    /// - **Web:** Custom cursors have to be loaded and decoded first, until then the previous
    ///   cursor is shown.
    fn set_cursor(&self, cursor: Cursor);

    /// Changes the position of the cursor in window coordinates.
    ///
    /// ```no_run
    /// # use dpi::{LogicalPosition, PhysicalPosition};
    /// # use winit_core::window::Window;
    /// # fn scope(window: &dyn Window) {
    /// // Specify the position in logical dimensions like this:
    /// window.set_cursor_position(LogicalPosition::new(400.0, 200.0).into());
    ///
    /// // Or specify the position in physical dimensions like this:
    /// window.set_cursor_position(PhysicalPosition::new(400, 200).into());
    /// # }
    /// ```
    ///
    /// ## Platform-specific
    ///
    /// - **Wayland**: Cursor must be in [`CursorGrabMode::Locked`].
    /// - **iOS / Android / Web / Orbital:** Always returns an [`RequestError::NotSupported`].
    fn set_cursor_position(&self, position: Position) -> Result<(), RequestError>;

    /// Set grabbing [mode][CursorGrabMode] on the cursor preventing it from leaving the window.
    ///
    /// ## Example
    ///
    /// First try confining the cursor, and if that fails, try locking it instead.
    ///
    /// ```no_run
    /// # use winit_core::window::{CursorGrabMode, Window};
    /// # fn scope(window: &dyn Window) {
    /// window
    ///     .set_cursor_grab(CursorGrabMode::Confined)
    ///     .or_else(|_e| window.set_cursor_grab(CursorGrabMode::Locked))
    ///     .unwrap();
    /// # }
    /// ```
    fn set_cursor_grab(&self, mode: CursorGrabMode) -> Result<(), RequestError>;

    /// Modifies the cursor's visibility.
    ///
    /// If `false`, this will hide the cursor. If `true`, this will show the cursor.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows:** The cursor is only hidden within the confines of the window.
    /// - **X11:** The cursor is only hidden within the confines of the window.
    /// - **Wayland:** The cursor is only hidden within the confines of the window.
    /// - **macOS:** The cursor is hidden as long as the window has input focus, even if the cursor
    ///   is outside of the window.
    /// - **iOS / Android:** Unsupported.
    fn set_cursor_visible(&self, visible: bool);

    /// Moves the window with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed
    /// immediately before this function is called.
    ///
    /// ## Platform-specific
    ///
    /// - **X11:** Un-grabs the cursor.
    /// - **Wayland:** Requires the cursor to be inside the window to be dragged.
    /// - **macOS:** May prevent the button release event to be triggered.
    /// - **iOS / Android / Web:** Always returns an [`RequestError::NotSupported`].
    fn drag_window(&self) -> Result<(), RequestError>;

    /// Resizes the window with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed
    /// immediately before this function is called.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Always returns an [`RequestError::NotSupported`]
    /// - **iOS / Android / Web:** Always returns an [`RequestError::NotSupported`].
    fn drag_resize_window(&self, direction: ResizeDirection) -> Result<(), RequestError>;

    /// Show [window menu] at a specified position in surface coordinates.
    ///
    /// This is the context menu that is normally shown when interacting with
    /// the title bar. This is useful when implementing custom decorations.
    ///
    /// ## Platform-specific
    /// **Android / iOS / macOS / Orbital / Wayland / Web / X11:** Unsupported.
    ///
    /// [window menu]: https://en.wikipedia.org/wiki/Common_menus_in_Microsoft_Windows#System_menu
    fn show_window_menu(&self, position: Position);

    /// Modifies whether the window catches cursor events.
    ///
    /// If `true`, the window will catch the cursor events. If `false`, events are passed through
    /// the window such that any other window behind it receives them. By default hittest is
    /// enabled.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Orbital:** Always returns an [`RequestError::NotSupported`].
    fn set_cursor_hittest(&self, hittest: bool) -> Result<(), RequestError>;

    /// Returns the monitor on which the window currently resides.
    ///
    /// Returns `None` if current monitor can't be detected.
    fn current_monitor(&self) -> Option<MonitorHandle>;

    /// Returns the list of all the monitors available on the system.
    ///
    /// This is the same as [`ActiveEventLoop::available_monitors`], and is provided for
    /// convenience.
    ///
    /// [`ActiveEventLoop::available_monitors`]: crate::event_loop::ActiveEventLoop::available_monitors
    fn available_monitors(&self) -> Box<dyn Iterator<Item = MonitorHandle>>;

    /// Returns the primary monitor of the system.
    ///
    /// Returns `None` if it can't identify any monitor as a primary one.
    ///
    /// This is the same as [`ActiveEventLoop::primary_monitor`], and is provided for convenience.
    ///
    /// ## Platform-specific
    ///
    /// - **Wayland:** Always returns `None`.
    ///
    /// [`ActiveEventLoop::primary_monitor`]: crate::event_loop::ActiveEventLoop::primary_monitor
    fn primary_monitor(&self) -> Option<MonitorHandle>;

    /// Get the raw-window-handle v0.6 display handle.
    fn rwh_06_display_handle(&self) -> &dyn rwh_06::HasDisplayHandle;

    /// Get the raw-window-handle v0.6 window handle.
    fn rwh_06_window_handle(&self) -> &dyn rwh_06::HasWindowHandle;
}

impl_dyn_casting!(Window);

impl PartialEq for dyn Window + '_ {
    fn eq(&self, other: &dyn Window) -> bool {
        self.id().eq(&other.id())
    }
}

impl Eq for dyn Window + '_ {}

impl std::hash::Hash for dyn Window + '_ {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl rwh_06::HasDisplayHandle for dyn Window + '_ {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        self.rwh_06_display_handle().display_handle()
    }
}

impl rwh_06::HasWindowHandle for dyn Window + '_ {
    fn window_handle(&self) -> Result<rwh_06::WindowHandle<'_>, rwh_06::HandleError> {
        self.rwh_06_window_handle().window_handle()
    }
}

/// The behavior of cursor grabbing.
///
/// Use this enum with [`Window::set_cursor_grab`] to grab the cursor.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CursorGrabMode {
    /// No grabbing of the cursor is performed.
    None,

    /// The cursor is confined to the window area.
    ///
    /// There's no guarantee that the cursor will be hidden. You should hide it by yourself if you
    /// want to do so.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Not implemented. Always returns [`RequestError::NotSupported`] for now.
    /// - **iOS / Android / Web:** Always returns an [`RequestError::NotSupported`].
    Confined,

    /// The cursor is locked inside the window area to the certain position.
    ///
    /// There's no guarantee that the cursor will be hidden. You should hide it by yourself if you
    /// want to do so.
    ///
    /// ## Platform-specific
    ///
    /// - **X11:** Not implemented. Always returns [`RequestError::NotSupported`] for now.
    /// - **iOS / Android:** Always returns an [`RequestError::NotSupported`].
    Locked,
}

/// Defines the orientation that a window resize will be performed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ResizeDirection {
    East,
    North,
    NorthEast,
    NorthWest,
    South,
    SouthEast,
    SouthWest,
    West,
}

impl From<ResizeDirection> for CursorIcon {
    fn from(direction: ResizeDirection) -> Self {
        use ResizeDirection::*;
        match direction {
            East => CursorIcon::EResize,
            North => CursorIcon::NResize,
            NorthEast => CursorIcon::NeResize,
            NorthWest => CursorIcon::NwResize,
            South => CursorIcon::SResize,
            SouthEast => CursorIcon::SeResize,
            SouthWest => CursorIcon::SwResize,
            West => CursorIcon::WResize,
        }
    }
}

/// The theme variant to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Theme {
    /// Use the light variant.
    Light,

    /// Use the dark variant.
    Dark,
}

/// ## Platform-specific
///
/// - **X11:** Sets the WM's `XUrgencyHint`. No distinction between [`Critical`] and
///   [`Informational`].
///
/// [`Critical`]: Self::Critical
/// [`Informational`]: Self::Informational
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UserAttentionType {
    /// ## Platform-specific
    ///
    /// - **macOS:** Bounces the dock icon until the application is in focus.
    /// - **Windows:** Flashes both the window and the taskbar button until the application is in
    ///   focus.
    Critical,

    /// ## Platform-specific
    ///
    /// - **macOS:** Bounces the dock icon once.
    /// - **Windows:** Flashes the taskbar button until the application is in focus.
    #[default]
    Informational,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WindowButtons: u32 {
        const CLOSE  = 1 << 0;
        const MINIMIZE  = 1 << 1;
        const MAXIMIZE  = 1 << 2;
    }
}

/// A window level groups windows with respect to their z-position.
///
/// The relative ordering between windows in different window levels is fixed.
/// The z-order of a window within the same window level may change dynamically on user interaction.
///
/// ## Platform-specific
///
/// - **iOS / Android / Web / Wayland:** Unsupported.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WindowLevel {
    /// The window will always be below normal windows.
    ///
    /// This is useful for a widget-based app.
    AlwaysOnBottom,

    /// The default.
    #[default]
    Normal,

    /// The window will always be on top of normal windows.
    AlwaysOnTop,
}

/// Generic IME purposes for use in [`Window::set_ime_purpose`].
///
/// The purpose may improve UX by optimizing the IME for the specific use case,
/// if winit can express the purpose to the platform and the platform reacts accordingly.
///
/// ## Platform-specific
///
/// - **iOS / Android / Web / Windows / X11 / macOS / Orbital:** Unsupported.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ImePurpose {
    /// No special hints for the IME (default).
    Normal,
    /// The IME is used for password input.
    Password,
    /// The IME is used to input into a terminal.
    ///
    /// For example, that could alter OSK on Wayland to show extra buttons.
    Terminal,
}

impl Default for ImePurpose {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ImeSurroundingTextError {
    /// Text exceeds 4000 bytes
    TextTooLong,
    /// Cursor not on a code point boundary, or past the end of text.
    CursorBadPosition,
    /// Anchor not on a code point boundary, or past the end of text.
    AnchorBadPosition,
}

/// Defines the text surrounding the caret
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ImeSurroundingText {
    /// An excerpt of the text present in the text input field, excluding preedit.
    text: String,
    /// The position of the caret, in bytes from the beginning of the string
    cursor: usize,
    /// The position of the other end of selection, in bytes.
    /// With no selection, it should be the same as the cursor.
    anchor: usize,
}

impl ImeSurroundingText {
    /// The maximum size of the text excerpt.
    pub const MAX_TEXT_BYTES: usize = 4000;
    /// Defines the text surroundng the cursor and the selection within it.
    ///
    /// `text`: An excerpt of the text present in the text input field, excluding preedit.
    /// It must be limited to 4000 bytes due to backend constraints.
    /// `cursor`: The position of the caret, in bytes from the beginning of the string.
    /// `anchor: The position of the other end of selection, in bytes.
    /// With no selection, it should be the same as the cursor.
    ///
    /// This may fail if the byte indices don't fall on code point boundaries,
    /// or if the text is too long.
    ///
    /// ## Examples:
    ///
    /// A text field containing `foo|bar` where `|` denotes the caret would correspond to a value
    /// obtained by:
    ///
    /// ```
    /// # use winit_core::window::ImeSurroundingText;
    /// let s = ImeSurroundingText::new("foobar".into(), 3, 3).unwrap();
    /// ```
    ///
    /// Because preedit is excluded from the text string, a text field containing `foo[baz|]bar`
    /// where `|` denotes the caret and [baz|] is the preedit would be created in exactly the same
    /// way.
    pub fn new(
        text: String,
        cursor: usize,
        anchor: usize,
    ) -> Result<Self, ImeSurroundingTextError> {
        let text = if text.len() < 4000 {
            text
        } else {
            return Err(ImeSurroundingTextError::TextTooLong);
        };

        let cursor = if text.is_char_boundary(cursor) && cursor <= text.len() {
            cursor
        } else {
            return Err(ImeSurroundingTextError::CursorBadPosition);
        };

        let anchor = if text.is_char_boundary(anchor) && anchor <= text.len() {
            anchor
        } else {
            return Err(ImeSurroundingTextError::AnchorBadPosition);
        };

        Ok(Self { text, cursor, anchor })
    }

    /// Consumes the object, releasing the text string only.
    /// Use this call in the backend to avoid an extra clone when submitting the surrounding text.
    pub fn into_text(self) -> String {
        self.text
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn anchor(&self) -> usize {
        self.anchor
    }
}

/// Request to send to IME.
#[derive(Debug, PartialEq, Clone)]
pub enum ImeRequest {
    /// Enable the IME with the [`ImeCapabilities`] and [`ImeRequestData`] as initial state. When
    /// the [`ImeRequestData`] is **not** matching capabilities fully, the default values will be
    /// used instead.
    ///
    /// **Requesting to update data matching not enabled capabilities will result in update
    /// being ignored.** The winit backend in such cases is recommended to log a warning. This
    /// appiles to both [`ImeRequest::Enable`] and [`ImeRequest::Update`]. For details on
    /// capabilities refer to [`ImeCapabilities`].
    ///
    /// To update the [`ImeCapabilities`], the IME must be disabled and then re-enabled.
    Enable(ImeEnableRequest),
    /// Update the state of already enabled IME. Issuing this request before [`ImeRequest::Enable`]
    /// will result in error.
    Update(ImeRequestData),
    /// Disable the IME.
    ///
    /// **The disable request can not fail**.
    Disable,
}

/// Initial IME request.
#[derive(Debug, Clone, PartialEq)]
pub struct ImeEnableRequest {
    capabilities: ImeCapabilities,
    request_data: ImeRequestData,
}

impl ImeEnableRequest {
    /// Create request for the [`ImeRequest::Enable`]
    ///
    /// This will return [`None`] if some capability was requested but its initial value was not
    /// set by the user or value was set by the user, but capability not requested.
    pub fn new(capabilities: ImeCapabilities, request_data: ImeRequestData) -> Option<Self> {
        if capabilities.cursor_area() ^ request_data.cursor_area.is_some() {
            return None;
        }

        if capabilities.purpose() ^ request_data.purpose.is_some() {
            return None;
        }

        if capabilities.surrounding_text() ^ request_data.surrounding_text.is_some() {
            return None;
        }
        Some(Self { capabilities, request_data })
    }

    /// [`ImeCapabilities`] to enable.
    pub const fn capabilities(&self) -> &ImeCapabilities {
        &self.capabilities
    }

    /// Request data attached to request.
    pub const fn request_data(&self) -> &ImeRequestData {
        &self.request_data
    }

    /// Destruct [`ImeEnableRequest`]  into its raw parts.
    pub fn into_raw(self) -> (ImeCapabilities, ImeRequestData) {
        (self.capabilities, self.request_data)
    }
}

/// IME capabilities supported by client.
///
/// For example, if the client doesn't support [`ImeCapabilities::cursor_area()`], then not enabling
/// it will make IME hide the popup window instead of placing it arbitrary over the
/// client's window surface.
///
/// When the capability is not enabled or not supported by the IME, trying to update its'
/// corresponding data with [`ImeRequest`] will be ignored.
///
/// New capabilities may be added to this struct in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ImeCapabilities(ImeCapabilitiesFlags);

impl ImeCapabilities {
    /// Returns a new empty set of capabilities.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks `purpose` as supported.
    ///
    /// For more details see [`ImeRequestData::with_purpose`].
    pub const fn with_purpose(self) -> Self {
        Self(self.0.union(ImeCapabilitiesFlags::PURPOSE))
    }

    /// Marks `purpose` as unsupported.
    ///
    /// For more details see [`ImeRequestData::with_purpose`].
    pub const fn without_purpose(self) -> Self {
        Self(self.0.difference(ImeCapabilitiesFlags::PURPOSE))
    }

    /// Returns `true` if `purpose` is supported.
    pub const fn purpose(&self) -> bool {
        self.0.contains(ImeCapabilitiesFlags::PURPOSE)
    }

    /// Marks `cursor_area` as supported.
    ///
    /// For more details see [`ImeRequestData::with_cursor_area`].
    pub const fn with_cursor_area(self) -> Self {
        Self(self.0.union(ImeCapabilitiesFlags::CURSOR_AREA))
    }

    /// Marks `cursor_area` as unsupported.
    ///
    /// For more details see [`ImeRequestData::with_cursor_area`].
    pub const fn without_cursor_area(self) -> Self {
        Self(self.0.difference(ImeCapabilitiesFlags::CURSOR_AREA))
    }

    /// Returns `true` if `cursor_area` is supported.
    pub const fn cursor_area(&self) -> bool {
        self.0.contains(ImeCapabilitiesFlags::CURSOR_AREA)
    }

    /// Marks `surrounding_text` as supported.
    ///
    /// For more details see [`ImeRequestData::with_surrounding_text`].
    pub const fn with_surrounding_text(self) -> Self {
        Self(self.0.union(ImeCapabilitiesFlags::SURROUNDING_TEXT))
    }

    /// Marks `surrounding_text` as unsupported.
    ///
    /// For more details see [`ImeRequestData::with_surrounding_text`].
    pub const fn without_surrounding_text(self) -> Self {
        Self(self.0.difference(ImeCapabilitiesFlags::SURROUNDING_TEXT))
    }

    /// Returns `true` if `surrounding_text` is supported.
    pub const fn surrounding_text(&self) -> bool {
        self.0.contains(ImeCapabilitiesFlags::SURROUNDING_TEXT)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub(crate) struct ImeCapabilitiesFlags : u8 {
        /// Client supports setting IME purpose.
        const PURPOSE = 1 << 0;
        /// Client supports reporting cursor area for IME popup to
        /// appear.
        const CURSOR_AREA = 1 << 1;
        /// Client supports reporting the text around the caret
        const SURROUNDING_TEXT = 1 << 2;
    }
}

/// The [`ImeRequest`] data to communicate to system's IME.
///
/// This applies multiple IME state properties at once.
/// Fields set to `None` are not updated and the previously sent
/// value is reused.
#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ImeRequestData {
    /// Text input purpose.
    ///
    /// To support updating it, enable [`ImeCapabilities::PURPOSE`].
    pub purpose: Option<ImePurpose>,
    /// The IME cursor area which should not be covered by the input method popup.
    ///
    /// To support updating it, enable [`ImeCapabilities::CURSOR_AREA`].
    pub cursor_area: Option<(Position, Size)>,
    /// The text surrounding the caret
    ///
    /// To support updating it, enable [`ImeCapabilities::SURROUNDING_TEXT`].
    pub surrounding_text: Option<ImeSurroundingText>,
}

impl ImeRequestData {
    /// Sets the purpose hint of the current text input.
    pub fn with_purpose(self, purpose: ImePurpose) -> Self {
        Self { purpose: Some(purpose), ..self }
    }

    /// Sets the IME cursor editing area.
    ///
    /// The `position` is the top left corner of that area
    /// in surface coordinates and `size` is the size of this area starting from the position. An
    /// example of such area could be a input field in the UI or line in the editor.
    ///
    /// The windowing system could place a candidate box close to that area, but try to not obscure
    /// the specified area, so the user input to it stays visible.
    ///
    /// The candidate box is the window / popup / overlay that allows you to select the desired
    /// characters. The look of this box may differ between input devices, even on the same
    /// platform.
    ///
    /// (Apple's official term is "candidate window", see their [chinese] and [japanese] guides).
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use dpi::{LogicalPosition, PhysicalPosition, LogicalSize, PhysicalSize};
    /// # use winit_core::window::ImeRequestData;
    /// # fn scope(ime_request_data: ImeRequestData) {
    /// // Specify the position in logical dimensions like this:
    /// let ime_request_data = ime_request_data.with_cursor_area(
    ///     LogicalPosition::new(400.0, 200.0).into(),
    ///     LogicalSize::new(100, 100).into(),
    /// );
    ///
    /// // Or specify the position in physical dimensions like this:
    /// let ime_request_data = ime_request_data.with_cursor_area(
    ///     PhysicalPosition::new(400, 200).into(),
    ///     PhysicalSize::new(100, 100).into(),
    /// );
    /// # }
    /// ```
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Orbital:** Unsupported.
    ///
    /// [chinese]: https://support.apple.com/guide/chinese-input-method/use-the-candidate-window-cim12992/104/mac/12.0
    /// [japanese]: https://support.apple.com/guide/japanese-input-method/use-the-candidate-window-jpim10262/6.3/mac/12.0
    pub fn with_cursor_area(self, position: Position, size: Size) -> Self {
        Self { cursor_area: Some((position, size)), ..self }
    }

    /// Describes the text surrounding the caret.
    ///
    /// The IME can then continue providing suggestions for the continuation of the existing text,
    /// as well as can erase text more accurately, for example glyphs composed of multiple code
    /// points.
    pub fn with_surrounding_text(self, surrounding_text: ImeSurroundingText) -> Self {
        Self { surrounding_text: Some(surrounding_text), ..self }
    }
}

/// Error from sending request to IME with
/// [`Window::request_ime_update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImeRequestError {
    /// IME is not yet enabled.
    NotEnabled,
    /// IME is already enabled.
    AlreadyEnabled,
    /// Not supported.
    NotSupported,
}

impl fmt::Display for ImeRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImeRequestError::NotEnabled => write!(f, "ime is not enabled."),
            ImeRequestError::AlreadyEnabled => write!(f, "ime is already enabled."),
            ImeRequestError::NotSupported => write!(f, "ime is not supported."),
        }
    }
}

impl std::error::Error for ImeRequestError {}

/// An opaque token used to activate the [`Window`].
///
/// [`Window`]: crate::window::Window
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ActivationToken {
    pub(crate) token: String,
}

impl ActivationToken {
    /// Make an [`ActivationToken`] from a string.
    ///
    /// This method should be used to wrap tokens passed by side channels to your application, like
    /// dbus.
    ///
    /// The validity of the token is ensured by the windowing system. Using the invalid token will
    /// only result in the side effect of the operation involving it being ignored (e.g. window
    /// won't get focused automatically), but won't yield any errors.
    ///
    /// To obtain a valid token consult the backend implementation.
    pub fn from_raw(token: String) -> Self {
        Self { token }
    }

    /// Convert the token to its string representation to later pass via IPC.
    pub fn into_raw(self) -> String {
        self.token
    }

    /// Get a reference to a raw token.
    pub fn as_raw(&self) -> &str {
        &self.token
    }
}

#[cfg(test)]
mod tests {

    use dpi::{LogicalPosition, LogicalSize, Position, Size};

    use super::{
        ImeCapabilities, ImeEnableRequest, ImeRequestData, ImeSurroundingText,
        ImeSurroundingTextError,
    };
    use crate::window::ImePurpose;

    #[test]
    fn ime_initial_request_caps_match() {
        let position: Position = LogicalPosition::new(0, 0).into();
        let size: Size = LogicalSize::new(0, 0).into();

        assert!(ImeEnableRequest::new(
            ImeCapabilities::new().with_cursor_area(),
            ImeRequestData::default()
        )
        .is_none());
        assert!(ImeEnableRequest::new(
            ImeCapabilities::new().with_purpose(),
            ImeRequestData::default()
        )
        .is_none());

        assert!(ImeEnableRequest::new(
            ImeCapabilities::new().with_cursor_area(),
            ImeRequestData::default().with_purpose(ImePurpose::Normal)
        )
        .is_none());

        assert!(ImeEnableRequest::new(
            ImeCapabilities::new(),
            ImeRequestData::default()
                .with_purpose(ImePurpose::Normal)
                .with_cursor_area(position, size)
        )
        .is_none());

        assert!(ImeEnableRequest::new(
            ImeCapabilities::new().with_cursor_area(),
            ImeRequestData::default()
                .with_purpose(ImePurpose::Normal)
                .with_cursor_area(position, size)
        )
        .is_none());

        assert!(ImeEnableRequest::new(
            ImeCapabilities::new().with_cursor_area(),
            ImeRequestData::default().with_cursor_area(position, size)
        )
        .is_some());

        assert!(ImeEnableRequest::new(
            ImeCapabilities::new().with_purpose().with_cursor_area(),
            ImeRequestData::default()
                .with_purpose(ImePurpose::Normal)
                .with_cursor_area(position, size)
        )
        .is_some());

        let text: &[u8] = ['a' as u8; 8000].as_slice();
        let text = std::str::from_utf8(text).unwrap();
        assert_eq!(
            ImeSurroundingText::new(text.into(), 0, 0),
            Err(ImeSurroundingTextError::TextTooLong),
        );

        assert_eq!(
            ImeSurroundingText::new("short".into(), 110, 0),
            Err(ImeSurroundingTextError::CursorBadPosition),
        );

        assert_eq!(
            ImeSurroundingText::new("граница".into(), 1, 0),
            Err(ImeSurroundingTextError::CursorBadPosition),
        );
    }
}
