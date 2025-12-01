//! Cross-platform shared texture handles for zero-copy GPU composition
//!
//! This module provides platform-agnostic shared texture abstractions that enable
//! zero-copy GPU composition between different rendering contexts (e.g., Bevy + GPUI).
//!
//! ## Platform Support
//!
//! - **Windows**: Uses D3D11/D3D12 shared NT handles via `IDXGIResource::GetSharedHandle()`
//! - **macOS**: Uses IOSurface for Metal texture sharing
//! - **Linux**: Uses DMA-BUF file descriptors for Vulkan texture sharing
//!
//! ## Example
//!
//! ```ignore
//! // Get the shared texture handle from GPUI window
//! let handle = window.get_shared_texture_handle()?;
//!
//! match handle {
//!     SharedTextureHandle::D3D11NTHandle(nt_handle) => {
//!         // Open in D3D12 context
//!     }
//!     SharedTextureHandle::IOSurface(io_surface) => {
//!         // Create Metal texture from IOSurface
//!     }
//!     SharedTextureHandle::DmaBuf(fd, modifier) => {
//!         // Import into Vulkan
//!     }
//! }
//! ```

use crate::{Size, DevicePixels};

/// Cross-platform shared texture handle
///
/// This enum abstracts platform-specific shared texture mechanisms,
/// allowing zero-copy GPU texture sharing across different graphics APIs.
#[derive(Debug, Clone)]
pub enum SharedTextureHandle {
    /// Windows D3D11/D3D12 shared NT handle
    ///
    /// This handle can be used to open the same GPU texture in multiple D3D11 or D3D12 devices.
    /// The handle is reference-counted by the OS and remains valid until all references are released.
    ///
    /// ## Safety
    /// The pointer must be closed with `CloseHandle()` when no longer needed.
    #[cfg(target_os = "windows")]
    D3D11NTHandle {
        /// The NT HANDLE to the shared D3D11 texture
        handle: *mut std::ffi::c_void,
        /// The size of the texture in device pixels
        size: Size<DevicePixels>,
        /// The DXGI format of the texture (typically DXGI_FORMAT_B8G8R8A8_UNORM = 87)
        format: u32,
    },

    /// macOS IOSurface handle
    ///
    /// IOSurface provides zero-copy texture sharing between Metal, OpenGL, and other frameworks.
    /// The IOSurface is reference-counted and automatically released when all references are dropped.
    ///
    /// ## Safety
    /// The pointer points to a CFTypeRef that must be retained/released following Core Foundation rules.
    #[cfg(target_os = "macos")]
    IOSurface {
        /// Raw pointer to IOSurfaceRef (a CFTypeRef)
        io_surface: *mut std::ffi::c_void,
        /// The size of the texture in device pixels
        size: Size<DevicePixels>,
        /// The Metal pixel format (e.g., MTLPixelFormatBGRA8Unorm = 80)
        format: u32,
    },

    /// Linux DMA-BUF file descriptor
    ///
    /// DMA-BUF provides zero-copy buffer sharing in the Linux kernel, commonly used
    /// for sharing Vulkan textures between processes or different GPU contexts.
    ///
    /// ## Safety
    /// The file descriptor must be closed with `close()` when no longer needed.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    DmaBuf {
        /// The DMA-BUF file descriptor
        fd: i32,
        /// The DRM format modifier (for tiling/compression info)
        modifier: u64,
        /// The size of the texture in device pixels
        size: Size<DevicePixels>,
        /// The Vulkan format (e.g., VK_FORMAT_B8G8R8A8_UNORM = 44)
        format: u32,
        /// Stride in bytes
        stride: u32,
    },
}

impl SharedTextureHandle {
    /// Get the size of the shared texture
    pub fn size(&self) -> Size<DevicePixels> {
        match self {
            #[cfg(target_os = "windows")]
            SharedTextureHandle::D3D11NTHandle { size, .. } => *size,
            #[cfg(target_os = "macos")]
            SharedTextureHandle::IOSurface { size, .. } => *size,
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            SharedTextureHandle::DmaBuf { size, .. } => *size,
        }
    }

    /// Get a human-readable description of the handle type
    pub fn type_name(&self) -> &'static str {
        match self {
            #[cfg(target_os = "windows")]
            SharedTextureHandle::D3D11NTHandle { .. } => "D3D11 NT Handle",
            #[cfg(target_os = "macos")]
            SharedTextureHandle::IOSurface { .. } => "IOSurface",
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            SharedTextureHandle::DmaBuf { .. } => "DMA-BUF",
        }
    }
}

// Platform-specific handle validation and utilities

#[cfg(target_os = "windows")]
impl SharedTextureHandle {
    /// Check if the NT handle is valid (non-null)
    pub fn is_valid(&self) -> bool {
        match self {
            SharedTextureHandle::D3D11NTHandle { handle, .. } => !handle.is_null(),
        }
    }
}

#[cfg(target_os = "macos")]
impl SharedTextureHandle {
    /// Check if the IOSurface pointer is valid (non-null)
    pub fn is_valid(&self) -> bool {
        match self {
            SharedTextureHandle::IOSurface { io_surface, .. } => !io_surface.is_null(),
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
impl SharedTextureHandle {
    /// Check if the DMA-BUF file descriptor is valid (>= 0)
    pub fn is_valid(&self) -> bool {
        match self {
            SharedTextureHandle::DmaBuf { fd, .. } => *fd >= 0,
        }
    }
}

/// Information needed to resize a renderer in external window mode
#[derive(Debug, Clone, Copy)]
pub struct ResizeInfo {
    /// The new physical size in device pixels
    pub physical_size: Size<DevicePixels>,
    /// Whether to recreate the shared texture
    pub recreate_shared_texture: bool,
}

impl ResizeInfo {
    /// Create a new resize info with the given physical size
    pub fn new(physical_size: Size<DevicePixels>) -> Self {
        Self {
            physical_size,
            recreate_shared_texture: true,
        }
    }

    /// Create a resize info that only updates viewport without recreating textures
    pub fn viewport_only(physical_size: Size<DevicePixels>) -> Self {
        Self {
            physical_size,
            recreate_shared_texture: false,
        }
    }
}
