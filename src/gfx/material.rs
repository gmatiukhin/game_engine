pub struct Texture {
    pub(crate) _texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
}

impl Texture {
    pub(crate) const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub(crate) const TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<
        'static,
    > = wgpu::BindGroupLayoutDescriptor {
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
    };

    pub(crate) fn depth_texture(
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
            format: Self::DEPTH_TEXTURE_FORMAT,
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

    pub(crate) fn default_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self::from_color(device, queue, &wgpu::Color::WHITE)
    }

    pub(crate) fn from_color(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color: &wgpu::Color,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Texture for color: {:?}", color)),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        let data = [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ];

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: Default::default(),
            },
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4),
                rows_per_image: std::num::NonZeroU32::new(1),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("Texture view for color texture: {:?}", color)),
            ..wgpu::TextureViewDescriptor::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Sampler for color texture: {:?}", color)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }

    pub(crate) fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &image::DynamicImage,
        label: &str,
    ) -> Self {
        let image = image.to_rgba8();
        let dimensions = image.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Texture for image: {}", label)),
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
            &image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(dimensions.1),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("Texture view for image: {}", label)),
            ..wgpu::TextureViewDescriptor::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Sampler for image: {}", label)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }

    pub(crate) fn from_text(device: &wgpu::Device, queue: &wgpu::Queue, text: &String, max_width: f32) -> Self {
        use ab_glyph::Font;

        let font = ab_glyph::FontRef::try_from_slice(include_bytes!(
            "../../res/fonts/pixel/Pixel NES.otf"
        ))
            .unwrap();
        let scale = 40.0;
        let scaled_font = font.as_scaled(ab_glyph::PxScale::from(scale));
        let glyphs = crate::text::layout_paragraph(scaled_font, (0.0, 0.0).into(), max_width, &text);
        // println!("{:#?}", glyphs);
        // std::process::exit(0);
        let (data, width, height) = crate::text::parse(&font, glyphs, scale);

        let mut image: image::RgbaImage = image::ImageBuffer::new(width as u32, height as u32);

        for (i, v) in data.iter().enumerate() {
            let x = i as u32 % image.width();
            let x = if x >= image.width() { image.width() - 1} else { x };
            let y = (i as u32 - x) / image.width();
            let y = if y >= image.height() { image.height() - 1} else { y };
                image.put_pixel(
                    x, y,
                    image::Rgba([
                        (v * 255.0) as u8,
                        (v * 255.0) as u8,
                        (v * 255.0) as u8,
                        255,
                    ]));
        };

        let texture_size = wgpu::Extent3d {
            width: width,
            height: height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture for text"),
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
            &image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * width),
                rows_per_image: std::num::NonZeroU32::new(height),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Texture for text"),
            ..wgpu::TextureViewDescriptor::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture for text"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }
}

pub enum Material {
    Textured(Image),
    FlatColor(wgpu::Color),
}

impl Material {
    pub(crate) fn texture(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Texture {
        match self {
            Material::Textured(img) => Texture::from_image(device, queue, &img.file, &img.name),
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
