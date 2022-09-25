#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Constants
impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    // Constant color values
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    pub const RED: Self = Self {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    pub const GREEN: Self = Self {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };

    pub const BLUE: Self = Self {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };

    pub const YELLOW: Self = Self {
        r: 255,
        g: 255,
        b: 0,
        a: 255,
    };

    pub const MAGENTA: Self = Self {
        r: 255,
        g: 0,
        b: 255,
        a: 255,
    };

    pub const CYAN: Self = Self {
        r: 0,
        g: 255,
        b: 255,
        a: 255,
    };

    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
}

impl Color {
    pub fn premultiply(&self) -> Self {
        let r = (self.r as f32 * self.a as f32 / 255.0) as u8;
        let g = (self.g as f32 * self.a as f32 / 255.0) as u8;
        let b = (self.b as f32 * self.a as f32 / 255.0) as u8;
        Self { r, g, b, a: self.a }
    }

    /// Blends dst over src
    pub fn blend(dst: &Self, src: &Self) -> Self {
        let inv_a = 255 - src.a;

        let r = src.r + ((dst.r as u16 * inv_a as u16) / 255) as u8;
        let g = src.g + ((dst.g as u16 * inv_a as u16) / 255) as u8;
        let b = src.b + ((dst.b as u16 * inv_a as u16) / 255) as u8;
        let a = src.a + ((dst.a as u16 * inv_a as u16) / 255) as u8;

        Self { r, g, b, a }
    }
}

impl Into<wgpu::Color> for Color {
    fn into(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64 / 255.0,
            g: self.g as f64 / 255.0,
            b: self.b as f64 / 255.0,
            a: self.a as f64 / 255.0,
        }
    }
}

impl From<wgpu::Color> for Color {
    fn from(color: wgpu::Color) -> Self {
        Self {
            r: (color.r * 255.0) as u8,
            g: (color.g * 255.0) as u8,
            b: (color.b * 255.0) as u8,
            a: (color.a * 255.0) as u8,
        }
    }
}

impl Into<image::Rgba<u8>> for Color {
    fn into(self) -> image::Rgba<u8> {
        image::Rgba([self.r, self.g, self.b, self.a])
    }
}

impl From<image::Rgba<u8>> for Color {
    fn from(rgba8: image::Rgba<u8>) -> Self {
        Self {
            r: rgba8[0],
            g: rgba8[1],
            b: rgba8[2],
            a: rgba8[3],
        }
    }
}

pub(in crate::gfx) const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat =
    wgpu::TextureFormat::Depth32Float;

pub struct Texture {
    pub(crate) _texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
}

impl Texture {
    pub(in crate::gfx) fn depth_texture(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("depth_texture_view"),
            ..wgpu::TextureViewDescriptor::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("depth_texture_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: Some(wgpu::CompareFunction::Less),
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }

    pub(in crate::gfx) fn default_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self::from_color(device, queue, &Color::WHITE)
    }

    pub(crate) fn from_color(device: &wgpu::Device, queue: &wgpu::Queue, color: &Color) -> Self {
        let data = [color.r, color.g, color.b, color.a];

        Self::from_bytes_rgba(
            &device,
            &queue,
            &data,
            1,
            1,
            false,
            Some(&format!("{:?}", color)),
        )
    }

    pub(crate) fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &image::DynamicImage,
        label: &str,
        pixelated: bool,
    ) -> Self {
        let image = image.to_rgba8();
        let dimensions = image.dimensions();

        Self::from_bytes_rgba(
            &device,
            &queue,
            &image,
            dimensions.0,
            dimensions.1,
            pixelated,
            Some(label),
        )
    }

    pub(crate) fn from_bytes_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        width: u32,
        height: u32,
        pixelated: bool,
        label: Option<&str>,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: Default::default(),
            },
            &bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * width),
                rows_per_image: std::num::NonZeroU32::new(height),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mag_min_filter = if pixelated {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        };

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: mag_min_filter,
            min_filter: mag_min_filter,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }

    pub(crate) fn texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    pub(crate) fn texture_bind_group(device: &wgpu::Device, texture: &Self) -> wgpu::BindGroup {
        let texture_bind_group_layout = Self::texture_bind_group_layout(&device);

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    }
}

pub enum Material {
    Textured(Image),
    FlatColor(Color),
}

impl Material {
    pub(crate) fn texture(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Texture {
        match self {
            Material::Textured(img) => {
                Texture::from_image(device, queue, &img.file, &img.name, false)
            }
            Material::FlatColor(color) => Texture::from_color(device, queue, &color),
        }
    }
}

pub struct Shader {
    pub name: String,
    pub contents: String,
}

pub struct Image {
    pub name: String,
    pub file: image::DynamicImage,
}
