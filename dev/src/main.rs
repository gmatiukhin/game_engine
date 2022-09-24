#[allow(unused_imports)]
use game_engine::{
    cgmath::{Deg, InnerSpace, One, Point2, Point3, Quaternion, Rad, Vector2, Vector3},
    gfx::{
        gfx_2d::{
            components_2d::{DrawMode, Sprite},
            text::{FontParameters, TextParameters},
        },
        gfx_3d::{
            components_3d::{Mesh, Model, PrefabInstance, Vertex},
            Renderer3D,
        },
        texture::{Image, Material, PixelColor, Shader, Texture},
        GraphicsEngine,
    },
    image::{load_from_memory, Rgba, RgbaImage},
    input::{InputHandler, MouseButton, VirtualKeyCode},
    Game, GameObject, ResizeMode, WindowSettings,
};
use std::f32::consts::FRAC_PI_2;

struct PrefabController {
    model: Model,
    movable_instance: Option<PrefabInstance>,
    immovable_instance: Option<PrefabInstance>,
}

impl PrefabController {
    fn new() -> Self {
        let vertices = vec![
            Vertex {
                position: (0.0, 1.0, 0.0).into(),
                tex_coords: (0.0, 0.0).into(),
            },
            Vertex {
                position: (0.0, 0.0, 0.0).into(),
                tex_coords: (0.0, 1.0).into(),
            },
            Vertex {
                position: (1.0, 0.0, 0.0).into(),
                tex_coords: (1.0, 1.0).into(),
            },
            Vertex {
                position: (1.0, 1.0, 0.0).into(),
                tex_coords: (1.0, 0.0).into(),
            },
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let mesh = Mesh { vertices, indices };

        let buffer = std::fs::read("./dev/res/textures/stone_bricks.jpg").unwrap();
        let image = load_from_memory(&buffer).unwrap();

        let model = Model::new(
            "Square Prefab",
            mesh,
            Some(Material::Textured(Image {
                name: "Stone Bricks".to_string(),
                file: image,
            })),
            None,
        );

        Self {
            model,
            movable_instance: None,
            immovable_instance: None,
        }
    }
}

impl GameObject for PrefabController {
    fn start(&mut self, graphics_engine: &mut GraphicsEngine) {
        let renderer = &mut graphics_engine.renderer_3d;

        renderer.add_as_prefab(&self.model);
        // self.immovable_instance = renderer.instantiate_prefab(
        //     &self.model.name,
        //     &(0.0, 0.0, 0.0).into(),
        //     &Quaternion::one(),
        // );

        self.movable_instance = renderer.instantiate_prefab(
            &self.model.name,
            &(0.0, 0.0, 1.0).into(),
            &Quaternion::one(),
        );
    }

    fn update(
        &mut self,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
        dt: f32,
    ) {
        let renderer = &mut graphics_engine.renderer_3d;
        if let Some(movable_instance) = &mut self.movable_instance {
            if input_handler.is_key_held(&VirtualKeyCode::Up) {
                movable_instance.position.y += 1.0 * dt;
            }

            if input_handler.is_key_held(&VirtualKeyCode::Down) {
                movable_instance.position.y += -1.0 * dt;
            }

            if input_handler.is_key_down(&VirtualKeyCode::R) {
                renderer.delete_prefab_instance(movable_instance);
            }

            renderer.update_prefab_instance(&movable_instance);
        }

        if let Some(immovable_instance) = &mut self.immovable_instance {
            if input_handler.is_key_down(&VirtualKeyCode::Q) {
                renderer.delete_prefab_instance(&immovable_instance);
            }

            renderer.update_prefab_instance(&immovable_instance);
        }
    }
}

struct ModelController {}

impl GameObject for ModelController {
    fn start(&mut self, graphics_engine: &mut GraphicsEngine) {
        let vertices = vec![
            Vertex {
                position: (0.0, 1.0, 0.0).into(),
                tex_coords: (0.0, 0.0).into(),
            },
            Vertex {
                position: (0.0, 0.0, 0.0).into(),
                tex_coords: (0.0, 1.0).into(),
            },
            Vertex {
                position: (1.0, 0.0, 0.0).into(),
                tex_coords: (1.0, 1.0).into(),
            },
            Vertex {
                position: (1.0, 1.0, 0.0).into(),
                tex_coords: (1.0, 0.0).into(),
            },
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let mesh = Mesh { vertices, indices };

        let buffer = std::fs::read("./dev/res/textures/stone_bricks.jpg").unwrap();
        let image = load_from_memory(&buffer).unwrap();

        let model = Model::new(
            "Square model",
            mesh,
            Some(Material::Textured(Image {
                name: "Stone Bricks".to_string(),
                file: image,
            })),
            None,
        );

        let renderer = &mut graphics_engine.renderer_3d;

        renderer.add_model(model);
    }

    fn update(
        &mut self,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
        dt: f32,
    ) {
        let renderer = &mut graphics_engine.renderer_3d;
        if let Some(Model { mesh, .. }) = renderer.get_model("Square model") {
            if input_handler.is_key_held(&VirtualKeyCode::I) {
                mesh.vertices[0].position[1] += 1.0 * dt;
            }

            if input_handler.is_key_held(&VirtualKeyCode::J) {
                mesh.vertices[0].position[1] += -1.0 * dt;
            }
        }

        if input_handler.is_key_down(&VirtualKeyCode::K) {
            renderer.remove_model("Square model");
        }
    }
}

struct CameraController {}

impl CameraController {
    const SPEED: f32 = 4.0;
    const ZOOM_SPEED: f32 = 16.0;
    const SENSITIVITY: f32 = 0.4;

    const MIN_FOVY_DEG: Deg<f32> = Deg(10.0);
    const MAX_FOVY_DEG: Deg<f32> = Deg(90.0);
    const DEG_PER_ZOOM: Deg<f32> = Deg(15.0);
}

impl GameObject for CameraController {
    fn start(&mut self, graphics_engine: &mut GraphicsEngine) {
        let renderer = &mut graphics_engine.renderer_3d;
        renderer.camera().position = Point3::new(0.0, 0.0, 2.0);
    }

    fn update(
        &mut self,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
        dt: f32,
    ) {
        let renderer = &mut graphics_engine.renderer_3d;
        let camera = renderer.camera();

        let mut translation = Vector3::new(0.0 as f32, 0.0, 0.0);
        if input_handler.is_key_held(&VirtualKeyCode::D) {
            translation.x += 1.0;
        }
        if input_handler.is_key_held(&VirtualKeyCode::A) {
            translation.x += -1.0;
        };

        if input_handler.is_key_held(&VirtualKeyCode::Space) {
            translation.y += 1.0;
        }
        if input_handler.is_key_held(&VirtualKeyCode::LShift) {
            translation.y += -1.0;
        }

        if input_handler.is_key_held(&VirtualKeyCode::W) {
            translation.z += 1.0;
        }
        if input_handler.is_key_held(&VirtualKeyCode::S) {
            translation.z += -1.0;
        }

        translation = translation.normalize() * Self::SPEED * dt;

        if translation.x.is_nan() {
            translation = Vector3::new(0.0, 0.0, 0.0);
        }

        camera.position += camera.view_direction() * translation.z;
        camera.position += camera.right_direction() * translation.x;
        camera.position += camera.up_direction() * translation.y;

        if input_handler.is_mouse_button_held(&MouseButton::Left) {
            let cursor_delta = input_handler.cursor_delta();

            camera.yaw += Rad(cursor_delta.x) * Self::SENSITIVITY * dt;
            camera.pitch += Rad(cursor_delta.y) * Self::SENSITIVITY * dt;

            if camera.pitch < -Rad(FRAC_PI_2) {
                camera.pitch = -Rad(FRAC_PI_2);
            } else if camera.pitch > Rad(FRAC_PI_2) {
                camera.pitch = Rad(FRAC_PI_2);
            }
        }

        let fovy_delta_deg =
            Self::DEG_PER_ZOOM * input_handler.scroll_delta() * Self::ZOOM_SPEED * dt;
        let fovy_delta_rad = -Rad::from(fovy_delta_deg);

        let mut fovy = camera.fovy + fovy_delta_rad;

        if fovy < Rad::from(Self::MIN_FOVY_DEG) {
            fovy = Rad::from(Self::MIN_FOVY_DEG);
        } else if fovy > Rad::from(Self::MAX_FOVY_DEG) {
            fovy = Rad::from(Self::MAX_FOVY_DEG);
        }
        camera.fovy = fovy;
    }
}

struct GFX2DController {
    sprite: Sprite,
    position: Point2<f32>,
}

impl GFX2DController {
    fn new() -> Self {
        let mut sprite = Sprite::new(20, 20, PixelColor::TRANSPARENT);
        for y in 0..sprite.height() {
            for x in 0..sprite.width() {
                sprite.draw_pixel(
                    (x as i32, y as i32).into(),
                    PixelColor::new((x * 10) as u8, (y * 10) as u8, 0, 200),
                );
            }
        }

        Self {
            sprite,
            position: Point2::new(0.0, 0.0),
        }
    }
}

impl GameObject for GFX2DController {
    fn start(&mut self, graphics_engine: &mut GraphicsEngine) {
        let renderer_2d = &mut graphics_engine.renderer_2d;

        renderer_2d
            .background()
            .clear(PixelColor::new(26, 178, 255, 255));
    }

    fn update(
        &mut self,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
        dt: f32,
    ) {
        let gui = &mut graphics_engine.renderer_2d;

        let surface = gui.foreground();
        surface.clear(PixelColor::TRANSPARENT);

        surface.draw_text(
            &TextParameters {
                text: "Hello world".to_string(),
                color: PixelColor::BLACK,
                scale: 40.0,
                font: FontParameters::Default,
            },
            (0, 0).into(),
            400,
            200,
        );

        // Move sprite up, down, left, right using arrow keys
        let mut direction = Vector2::new(
            input_handler.is_key_held(&VirtualKeyCode::Right) as i32 as f32
                - input_handler.is_key_held(&VirtualKeyCode::Left) as i32 as f32,
            input_handler.is_key_held(&VirtualKeyCode::Down) as i32 as f32
                - input_handler.is_key_held(&VirtualKeyCode::Up) as i32 as f32,
        )
        .normalize();

        if direction.x.is_nan() || direction.y.is_nan() {
            direction = Vector2::new(0.0, 0.0);
        }

        let mut speed: f32 = 50.0;
        if input_handler.is_key_held(&VirtualKeyCode::LShift) {
            speed *= 2.0;
        }

        direction *= speed * dt;
        self.position += direction;

        surface.draw_sprite(
            &self.sprite,
            (self.position.x as i32, self.position.y as i32).into(),
        );
    }
}

struct GameController {}

impl GameObject for GameController {
    fn update(
        &mut self,
        _graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
        _dt: f32,
    ) {
        if input_handler.is_key_down(&VirtualKeyCode::Escape) {
            Game::exit();
        }
    }

    fn end(&mut self) {
        println!("Game ended");
    }
}

fn main() {
    env_logger::init();

    let mut game = Game::new(
        "Test game",
        WindowSettings {
            window_width: 1280,
            window_height: 720,
            resize_mode: ResizeMode::Resize,
        },
    );

    let prefab_controller = PrefabController::new();
    game.add_game_object(prefab_controller);

    let model_controller = ModelController {};
    game.add_game_object(model_controller);

    let camera_controller = CameraController {};
    game.add_game_object(camera_controller);

    let ui = GFX2DController::new();
    game.add_game_object(ui);

    let game_controller = GameController {};
    game.add_game_object(game_controller);

    game.run();
}
