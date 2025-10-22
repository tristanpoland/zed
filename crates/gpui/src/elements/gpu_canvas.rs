use crate::{
    App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    ObjectFit, Pixels, Style, StyleRefinement, Styled, Window,
};
use refineable::Refineable;
use std::sync::Arc;

/// Platform-specific GPU texture handle for zero-copy rendering.
/// This represents a shared GPU texture that can be rendered without CPU copies.
#[derive(Clone, Debug)]
pub enum GpuTextureHandle {
    #[cfg(target_os = "windows")]
    Windows {
        /// NT handle to the shared DX12/DX11 texture
        nt_handle: isize,
        /// Width of the texture
        width: u32,
        /// Height of the texture
        height: u32,
    },
    #[cfg(target_os = "macos")]
    Metal {
        /// IOSurface handle for Metal/OpenGL sharing
        io_surface: metal::IOSurface,
    },
    #[cfg(target_os = "linux")]
    Vulkan {
        /// DMA-BUF file descriptor for Vulkan/OpenGL sharing
        dma_buf_fd: i32,
        /// Width of the texture
        width: u32,
        /// Height of the texture
        height: u32,
    },
}

unsafe impl Send for GpuTextureHandle {}
unsafe impl Sync for GpuTextureHandle {}

/// Double-buffered GPU texture source for flicker-free rendering.
/// One buffer is written by the producer while the other is read by GPUI.
#[derive(Clone)]
pub struct GpuCanvasSource {
    /// Current active buffer index (0 or 1)
    active_buffer: Arc<std::sync::atomic::AtomicUsize>,
    /// The two shared GPU texture handles
    buffers: [GpuTextureHandle; 2],
}

impl GpuCanvasSource {
    /// Create a new double-buffered GPU canvas source.
    pub fn new(buffer0: GpuTextureHandle, buffer1: GpuTextureHandle) -> Self {
        Self {
            active_buffer: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            buffers: [buffer0, buffer1],
        }
    }

    /// Get the currently active buffer for reading.
    pub fn active_buffer(&self) -> &GpuTextureHandle {
        let index = self.active_buffer.load(std::sync::atomic::Ordering::Acquire);
        &self.buffers[index % 2]
    }

    /// Swap to the other buffer (call this from the producer thread after rendering).
    pub fn swap_buffers(&self) {
        self.active_buffer
            .fetch_xor(1, std::sync::atomic::Ordering::Release);
    }
}

/// A GPU canvas element for zero-copy rendering of external GPU content.
///
/// This element displays GPU textures shared from another rendering context
/// (e.g., DX12, Vulkan, Metal) without any CPU copies. It uses double-buffering
/// to avoid tearing and allows the producer to render independently.
///
/// # Example
/// ```ignore
/// gpu_canvas(source.clone())
///     .object_fit(ObjectFit::Cover)
///     .w_full()
///     .h_full()
/// ```
pub struct GpuCanvas {
    source: GpuCanvasSource,
    object_fit: ObjectFit,
    style: StyleRefinement,
}

/// Create a new GPU canvas element with the given texture source.
pub fn gpu_canvas(source: GpuCanvasSource) -> GpuCanvas {
    GpuCanvas {
        source,
        object_fit: ObjectFit::Contain,
        style: Default::default(),
    }
}

impl GpuCanvas {
    /// Set how the GPU texture should fit within the element bounds.
    pub fn object_fit(mut self, object_fit: ObjectFit) -> Self {
        self.object_fit = object_fit;
        self
    }
}

impl Element for GpuCanvas {
    type RequestLayoutState = ();
    type PrepaintState = GpuTextureHandle;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        let layout_id = window.request_layout(style, [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        self.source.active_buffer().clone()
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        window.paint_gpu_texture(bounds, prepaint.clone(), self.object_fit);
    }
}

impl IntoElement for GpuCanvas {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Styled for GpuCanvas {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
