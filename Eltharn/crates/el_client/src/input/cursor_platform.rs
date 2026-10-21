#[cfg(not(target_arch = "wasm32"))]
use winit::{dpi::PhysicalPosition, window::Window};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn prepare_release(window: &Window, position: PhysicalPosition<f64>) -> bool {
    #[cfg(target_os = "linux")]
    {
        use raw_window_handle::{
            HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
        };
        use wayland_client::{Connection, Proxy, protocol::wl_surface::WlSurface};

        let (Ok(display), Ok(surface)) = (window.display_handle(), window.window_handle()) else {
            return false;
        };
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(surface)) =
            (display.as_raw(), surface.as_raw())
        else {
            return false;
        };
        if !window.has_focus() {
            return false;
        }
        if window.set_cursor_position(position).is_err() {
            return false;
        }
        let committed = unsafe {
            let backend = wayland_backend::client::Backend::from_foreign_display(
                display.display.as_ptr().cast(),
            );
            let connection = Connection::from_backend(backend);
            let Ok(id) = wayland_backend::client::ObjectId::from_ptr(
                WlSurface::interface(),
                surface.surface.as_ptr().cast(),
            ) else {
                return false;
            };
            let Ok(surface) = WlSurface::from_id(&connection, id) else {
                return false;
            };
            surface.commit();
            connection.flush().is_ok()
        };
        return committed;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (window, position);
        false
    }
}
