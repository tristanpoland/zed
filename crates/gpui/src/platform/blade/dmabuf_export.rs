//! DMA-BUF export support for Linux via Vulkan external memory extensions
//!
//! This module provides FULL DMA-BUF export functionality by accessing blade-graphics's
//! internal Vulkan device handles and creating exportable images.

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use anyhow::{anyhow, Result};

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use crate::{DevicePixels, SharedTextureHandle, Size};

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use std::sync::Arc;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use ash::vk;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub fn export_texture_as_dmabuf(
    gpu_context: &Arc<blade_graphics::Context>,
    texture: blade_graphics::Texture,
    size: Size<DevicePixels>,
    format: blade_graphics::TextureFormat,
) -> Result<Option<SharedTextureHandle>> {
    unsafe {
        // Use blade's new export methods
        let vk_device = gpu_context.vk_device();
        let vk_image = texture.vk_image();
        let vk_memory = gpu_context.vk_texture_memory(texture)
            .ok_or_else(|| anyhow!("Texture has no memory"))?;

        let ext_memory_fd = gpu_context.vk_external_memory_fd()
            .ok_or_else(|| anyhow!("External memory FD extension not available"))?;

        // Export the memory as DMA-BUF FD
        let get_fd_info = vk::MemoryGetFdInfoKHR::default()
            .memory(vk_memory)
            .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

        let fd = ext_memory_fd.get_memory_fd(&get_fd_info)
            .map_err(|e| anyhow!("Failed to export memory as DMA-BUF FD: {:?}", e))?;

        log::info!(
            "✅ Successfully exported DMA-BUF: fd={}, image={:?}, size={}x{}, format={:?}",
            fd, vk_image, size.width.0, size.height.0, format
        );

        // Calculate stride (assuming 4 bytes per pixel for BGRA/RGBA formats)
        let stride = size.width.0 as u32 * 4;

        // Return the DMA-BUF handle
        Ok(Some(SharedTextureHandle::DmaBuf {
            fd,
            modifier: 0,
            size,
            format: format as u32,
            stride,
        }))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
pub fn export_texture_as_dmabuf(
    _gpu_context: &std::sync::Arc<blade_graphics::Context>,
    _texture: blade_graphics::Texture,
    _size: crate::Size<crate::DevicePixels>,
    _format: blade_graphics::TextureFormat,
) -> anyhow::Result<Option<crate::SharedTextureHandle>> {
    Ok(None)
}
