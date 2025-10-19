use collections::FxHashMap;
use etagere::BucketedAtlasAllocator;
use parking_lot::Mutex;
use windows::Win32::Graphics::{
    Direct3D11::{
        D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_CPU_ACCESS_WRITE, D3D11_MAP_WRITE,
        D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
        ID3D11Device, ID3D11DeviceContext, ID3D11ShaderResourceView, ID3D11Texture2D,
    },
    Dxgi::Common::*,
};

use crate::{
    AtlasKey, AtlasTextureId, AtlasTextureKind, AtlasTile, Bounds, DevicePixels, ExternalTextureId,
    PlatformAtlas, Point, Size, platform::AtlasTextureList,
};

pub(crate) struct DirectXAtlas(Mutex<DirectXAtlasState>);

/// External texture entry with double buffering and CPU-mappable staging texture
struct ExternalTextureEntry {
    /// Front texture (currently being rendered)
    front_texture: ID3D11Texture2D,
    front_view: ID3D11ShaderResourceView,
    /// Back texture (currently being written to)
    back_texture: ID3D11Texture2D,
    back_view: ID3D11ShaderResourceView,
    /// Staging texture for CPU writes (D3D11_USAGE_STAGING with CPU_ACCESS_WRITE)
    staging_texture: ID3D11Texture2D,
    /// Size of the texture
    size: Size<DevicePixels>,
    /// Pixel format
    format: DXGI_FORMAT,
    /// Bytes per pixel
    bytes_per_pixel: u32,
    /// Whether buffers need to be swapped
    needs_swap: bool,
    /// Whether staging texture is currently mapped
    is_mapped: bool,
}

struct DirectXAtlasState {
    device: ID3D11Device,
    device_context: ID3D11DeviceContext,
    monochrome_textures: AtlasTextureList<DirectXAtlasTexture>,
    polychrome_textures: AtlasTextureList<DirectXAtlasTexture>,
    tiles_by_key: FxHashMap<AtlasKey, AtlasTile>,
    external_textures: FxHashMap<ExternalTextureId, ExternalTextureEntry>,
    next_external_texture_id: u64,
}

struct DirectXAtlasTexture {
    id: AtlasTextureId,
    bytes_per_pixel: u32,
    allocator: BucketedAtlasAllocator,
    texture: ID3D11Texture2D,
    view: [Option<ID3D11ShaderResourceView>; 1],
    live_atlas_keys: u32,
}

impl DirectXAtlas {
    pub(crate) fn new(device: &ID3D11Device, device_context: &ID3D11DeviceContext) -> Self {
        DirectXAtlas(Mutex::new(DirectXAtlasState {
            device: device.clone(),
            device_context: device_context.clone(),
            monochrome_textures: Default::default(),
            polychrome_textures: Default::default(),
            tiles_by_key: Default::default(),
            external_textures: Default::default(),
            next_external_texture_id: 1,
        }))
    }

    pub(crate) fn get_texture_view(
        &self,
        id: AtlasTextureId,
    ) -> [Option<ID3D11ShaderResourceView>; 1] {
        let lock = self.0.lock();
        let tex = lock.texture(id);
        tex.view.clone()
    }

    pub(crate) fn handle_device_lost(
        &self,
        device: &ID3D11Device,
        device_context: &ID3D11DeviceContext,
    ) {
        let mut lock = self.0.lock();
        lock.device = device.clone();
        lock.device_context = device_context.clone();
        lock.monochrome_textures = AtlasTextureList::default();
        lock.polychrome_textures = AtlasTextureList::default();
        lock.tiles_by_key.clear();
        lock.external_textures.clear();
    }

    /// Register a new external GPU texture for rendering with CPU-mappable memory
    pub fn register_external_texture(
        &self,
        size: Size<DevicePixels>,
        format: DXGI_FORMAT,
    ) -> anyhow::Result<ExternalTextureId> {
        let mut lock = self.0.lock();

        let bytes_per_pixel = match format {
            DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_B8G8R8A8_UNORM => 4,
            DXGI_FORMAT_R8_UNORM => 1,
            _ => anyhow::bail!("Unsupported texture format"),
        };

        // Create front texture (GPU-only, used for rendering)
        let front_desc = D3D11_TEXTURE2D_DESC {
            Width: size.width.0 as u32,
            Height: size.height.0 as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };

        let mut front_texture: Option<ID3D11Texture2D> = None;
        unsafe {
            lock.device
                .CreateTexture2D(&front_desc, None, Some(&mut front_texture))?;
        }
        let front_texture = front_texture.unwrap();

        let mut front_view = None;
        unsafe {
            lock.device
                .CreateShaderResourceView(&front_texture, None, Some(&mut front_view))?;
        }
        let front_view = front_view.unwrap();

        // Create back texture (identical to front)
        let mut back_texture: Option<ID3D11Texture2D> = None;
        unsafe {
            lock.device
                .CreateTexture2D(&front_desc, None, Some(&mut back_texture))?;
        }
        let back_texture = back_texture.unwrap();

        let mut back_view = None;
        unsafe {
            lock.device
                .CreateShaderResourceView(&back_texture, None, Some(&mut back_view))?;
        }
        let back_view = back_view.unwrap();

        // Create staging texture (CPU-mappable for direct writes)
        let staging_desc = D3D11_TEXTURE2D_DESC {
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..front_desc
        };

        let mut staging_texture: Option<ID3D11Texture2D> = None;
        unsafe {
            lock.device
                .CreateTexture2D(&staging_desc, None, Some(&mut staging_texture))?;
        }
        let staging_texture = staging_texture.unwrap();

        let id = ExternalTextureId(lock.next_external_texture_id);
        lock.next_external_texture_id += 1;

        lock.external_textures.insert(id, ExternalTextureEntry {
            front_texture,
            front_view,
            back_texture,
            back_view,
            staging_texture,
            size,
            format,
            bytes_per_pixel,
            needs_swap: false,
            is_mapped: false,
        });

        Ok(id)
    }

    /// Map an external texture for CPU writes, returns a mutable slice
    ///
    /// SAFETY: Caller must ensure the returned slice is not used after unmap is called
    pub unsafe fn map_external_texture(&self, id: ExternalTextureId) -> anyhow::Result<&mut [u8]> {
        let mut lock = self.0.lock();

        // Get texture info first
        let (staging_texture, size, bytes_per_pixel) = {
            let entry = lock.external_textures.get(&id)
                .ok_or_else(|| anyhow::anyhow!("External texture not found"))?;

            if entry.is_mapped {
                anyhow::bail!("Texture already mapped");
            }

            (entry.staging_texture.clone(), entry.size, entry.bytes_per_pixel)
        };

        let size = (size.width.0 * size.height.0) as usize * bytes_per_pixel as usize;

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            lock.device_context.Map(
                &staging_texture,
                0,
                D3D11_MAP_WRITE,
                0,
                Some(&mut mapped),
            )?;
        }

        // Now mark as mapped
        if let Some(entry) = lock.external_textures.get_mut(&id) {
            entry.is_mapped = true;
        }

        // SAFETY: The pointer is valid for the size of the texture, and the caller
        // guarantees it won't be used after unmap
        Ok(unsafe { std::slice::from_raw_parts_mut(mapped.pData as *mut u8, size) })
    }

    /// Unmap an external texture after CPU writes are complete
    pub fn unmap_external_texture(&self, id: ExternalTextureId) -> anyhow::Result<()> {
        let mut lock = self.0.lock();

        // Get texture references first
        let (staging_texture, back_texture, is_mapped) = {
            let entry = lock.external_textures.get(&id)
                .ok_or_else(|| anyhow::anyhow!("External texture not found"))?;

            if !entry.is_mapped {
                anyhow::bail!("Texture not mapped");
            }

            (entry.staging_texture.clone(), entry.back_texture.clone(), entry.is_mapped)
        };

        unsafe {
            lock.device_context.Unmap(&staging_texture, 0);
        }

        // Copy staging texture to back texture
        unsafe {
            lock.device_context.CopyResource(&back_texture, &staging_texture);
        }

        // Mark as unmapped and needs swap
        if let Some(entry) = lock.external_textures.get_mut(&id) {
            entry.is_mapped = false;
            entry.needs_swap = true;
        }

        Ok(())
    }

    /// Swap front/back buffers for an external texture
    pub fn swap_external_texture_buffers(&self, id: ExternalTextureId) -> anyhow::Result<()> {
        let mut lock = self.0.lock();
        let entry = lock.external_textures.get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("External texture not found"))?;

        if entry.needs_swap {
            std::mem::swap(&mut entry.front_texture, &mut entry.back_texture);
            std::mem::swap(&mut entry.front_view, &mut entry.back_view);
            entry.needs_swap = false;
        }
        Ok(())
    }

    /// Get texture view for rendering
    pub fn get_external_texture_view(&self, id: ExternalTextureId) -> anyhow::Result<[Option<ID3D11ShaderResourceView>; 1]> {
        let lock = self.0.lock();
        let entry = lock.external_textures.get(&id)
            .ok_or_else(|| anyhow::anyhow!("External texture not found"))?;
        Ok([Some(entry.front_view.clone())])
    }

    /// Unregister an external texture
    pub fn unregister_external_texture(&self, id: ExternalTextureId) {
        let mut lock = self.0.lock();
        lock.external_textures.remove(&id);
        // D3D11 resources are automatically released when dropped
    }
}

impl PlatformAtlas for DirectXAtlas {
    fn get_or_insert_with<'a>(
        &self,
        key: &AtlasKey,
        build: &mut dyn FnMut() -> anyhow::Result<
            Option<(Size<DevicePixels>, std::borrow::Cow<'a, [u8]>)>,
        >,
    ) -> anyhow::Result<Option<AtlasTile>> {
        let mut lock = self.0.lock();
        if let Some(tile) = lock.tiles_by_key.get(key) {
            Ok(Some(tile.clone()))
        } else {
            let Some((size, bytes)) = build()? else {
                return Ok(None);
            };
            let tile = lock
                .allocate(size, key.texture_kind())
                .ok_or_else(|| anyhow::anyhow!("failed to allocate"))?;
            let texture = lock.texture(tile.texture_id);
            texture.upload(&lock.device_context, tile.bounds, &bytes);
            lock.tiles_by_key.insert(key.clone(), tile.clone());
            Ok(Some(tile))
        }
    }

    fn remove(&self, key: &AtlasKey) {
        let mut lock = self.0.lock();

        let Some(id) = lock.tiles_by_key.remove(key).map(|tile| tile.texture_id) else {
            return;
        };

        let textures = match id.kind {
            AtlasTextureKind::Monochrome => &mut lock.monochrome_textures,
            AtlasTextureKind::Polychrome => &mut lock.polychrome_textures,
        };

        let Some(texture_slot) = textures.textures.get_mut(id.index as usize) else {
            return;
        };

        if let Some(mut texture) = texture_slot.take() {
            texture.decrement_ref_count();
            if texture.is_unreferenced() {
                textures.free_list.push(texture.id.index as usize);
                lock.tiles_by_key.remove(key);
            } else {
                *texture_slot = Some(texture);
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DirectXAtlasState {
    fn allocate(
        &mut self,
        size: Size<DevicePixels>,
        texture_kind: AtlasTextureKind,
    ) -> Option<AtlasTile> {
        {
            let textures = match texture_kind {
                AtlasTextureKind::Monochrome => &mut self.monochrome_textures,
                AtlasTextureKind::Polychrome => &mut self.polychrome_textures,
            };

            if let Some(tile) = textures
                .iter_mut()
                .rev()
                .find_map(|texture| texture.allocate(size))
            {
                return Some(tile);
            }
        }

        let texture = self.push_texture(size, texture_kind)?;
        texture.allocate(size)
    }

    fn push_texture(
        &mut self,
        min_size: Size<DevicePixels>,
        kind: AtlasTextureKind,
    ) -> Option<&mut DirectXAtlasTexture> {
        const DEFAULT_ATLAS_SIZE: Size<DevicePixels> = Size {
            width: DevicePixels(1024),
            height: DevicePixels(1024),
        };
        // Max texture size for DirectX. See:
        // https://learn.microsoft.com/en-us/windows/win32/direct3d11/overviews-direct3d-11-resources-limits
        const MAX_ATLAS_SIZE: Size<DevicePixels> = Size {
            width: DevicePixels(16384),
            height: DevicePixels(16384),
        };
        let size = min_size.min(&MAX_ATLAS_SIZE).max(&DEFAULT_ATLAS_SIZE);
        let pixel_format;
        let bind_flag;
        let bytes_per_pixel;
        match kind {
            AtlasTextureKind::Monochrome => {
                pixel_format = DXGI_FORMAT_R8_UNORM;
                bind_flag = D3D11_BIND_SHADER_RESOURCE;
                bytes_per_pixel = 1;
            }
            AtlasTextureKind::Polychrome => {
                pixel_format = DXGI_FORMAT_B8G8R8A8_UNORM;
                bind_flag = D3D11_BIND_SHADER_RESOURCE;
                bytes_per_pixel = 4;
            }
        }
        let texture_desc = D3D11_TEXTURE2D_DESC {
            Width: size.width.0 as u32,
            Height: size.height.0 as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: pixel_format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: bind_flag.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut texture: Option<ID3D11Texture2D> = None;
        unsafe {
            // This only returns None if the device is lost, which we will recreate later.
            // So it's ok to return None here.
            self.device
                .CreateTexture2D(&texture_desc, None, Some(&mut texture))
                .ok()?;
        }
        let texture = texture.unwrap();

        let texture_list = match kind {
            AtlasTextureKind::Monochrome => &mut self.monochrome_textures,
            AtlasTextureKind::Polychrome => &mut self.polychrome_textures,
        };
        let index = texture_list.free_list.pop();
        let view = unsafe {
            let mut view = None;
            self.device
                .CreateShaderResourceView(&texture, None, Some(&mut view))
                .ok()?;
            [view]
        };
        let atlas_texture = DirectXAtlasTexture {
            id: AtlasTextureId {
                index: index.unwrap_or(texture_list.textures.len()) as u32,
                kind,
            },
            bytes_per_pixel,
            allocator: etagere::BucketedAtlasAllocator::new(size.into()),
            texture,
            view,
            live_atlas_keys: 0,
        };
        if let Some(ix) = index {
            texture_list.textures[ix] = Some(atlas_texture);
            texture_list.textures.get_mut(ix).unwrap().as_mut()
        } else {
            texture_list.textures.push(Some(atlas_texture));
            texture_list.textures.last_mut().unwrap().as_mut()
        }
    }

    fn texture(&self, id: AtlasTextureId) -> &DirectXAtlasTexture {
        let textures = match id.kind {
            crate::AtlasTextureKind::Monochrome => &self.monochrome_textures,
            crate::AtlasTextureKind::Polychrome => &self.polychrome_textures,
        };
        textures[id.index as usize].as_ref().unwrap()
    }
}

impl DirectXAtlasTexture {
    fn allocate(&mut self, size: Size<DevicePixels>) -> Option<AtlasTile> {
        let allocation = self.allocator.allocate(size.into())?;
        let tile = AtlasTile {
            texture_id: self.id,
            tile_id: allocation.id.into(),
            bounds: Bounds {
                origin: allocation.rectangle.min.into(),
                size,
            },
            padding: 0,
        };
        self.live_atlas_keys += 1;
        Some(tile)
    }

    fn upload(
        &self,
        device_context: &ID3D11DeviceContext,
        bounds: Bounds<DevicePixels>,
        bytes: &[u8],
    ) {
        unsafe {
            device_context.UpdateSubresource(
                &self.texture,
                0,
                Some(&D3D11_BOX {
                    left: bounds.left().0 as u32,
                    top: bounds.top().0 as u32,
                    front: 0,
                    right: bounds.right().0 as u32,
                    bottom: bounds.bottom().0 as u32,
                    back: 1,
                }),
                bytes.as_ptr() as _,
                bounds.size.width.to_bytes(self.bytes_per_pixel as u8),
                0,
            );
        }
    }

    fn decrement_ref_count(&mut self) {
        self.live_atlas_keys -= 1;
    }

    fn is_unreferenced(&mut self) -> bool {
        self.live_atlas_keys == 0
    }
}

impl From<Size<DevicePixels>> for etagere::Size {
    fn from(size: Size<DevicePixels>) -> Self {
        etagere::Size::new(size.width.into(), size.height.into())
    }
}

impl From<etagere::Point> for Point<DevicePixels> {
    fn from(value: etagere::Point) -> Self {
        Point {
            x: DevicePixels::from(value.x),
            y: DevicePixels::from(value.y),
        }
    }
}
