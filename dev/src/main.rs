use game_engine::cgmath::Point2;
use game_engine::gfx::gfx_2d::components_2d::Surface2D;
#[allow(unused_imports)]
use game_engine::{
    cgmath::{Deg, InnerSpace, One, Point3, Quaternion, Rad, Vector3},
    gfx::{
        gfx_2d::{
            components_2d::{GUIPanel, GUIPanelContent, GUITransform},
            text::{FontParameters, TextParameters},
        },
        gfx_3d::{
            components_3d::{Mesh, Model, PrefabInstance, Vertex},
            Renderer3D,
        },
        texture::{Color, Image, Material, Shader, Texture},
        GraphicsEngine,
    },
    image::load_from_memory,
    input::{InputHandler, MouseButton, VirtualKeyCode},
    Game, GameObject,
};
use std::f32::consts::FRAC_PI_2;

struct ObjectController {
    model: Model,
    movable_instance: Option<PrefabInstance>,
    immovable_instance: Option<PrefabInstance>,
}

impl ObjectController {
    fn new() -> Self {
        let vertices = vec![
            Vertex {
                position: [0.0, 1.0, 0.0],
                texture_coordinates: [0.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0],
                texture_coordinates: [0.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                texture_coordinates: [1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                texture_coordinates: [1.0, 0.0],
            },
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let mesh = Mesh { vertices, indices };

        let buffer = std::fs::read("./dev/res/textures/stone_bricks.jpg").unwrap();
        let image = load_from_memory(&buffer).unwrap();

        let model = Model::new(
            "Pentagon",
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

impl GameObject for ObjectController {
    fn start(&mut self, graphics_engine: &mut GraphicsEngine) {
        let renderer = &mut graphics_engine.renderer_3d;

        renderer.add_as_prefab(&self.model);
        self.immovable_instance = renderer.instantiate_prefab(
            &self.model.name,
            &(0.0, 0.0, 0.0).into(),
            &Quaternion::one(),
        );
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

struct UI {}

impl GameObject for UI {
    fn start(&mut self, graphics_engine: &mut GraphicsEngine) {
        let gui = &mut graphics_engine.renderer_2d;
        let panel_with_text = GUIPanel {
            name: "Test text".to_string(),
            active: true,
            position: GUITransform::Relative(0.1, 0.1),
            dimensions: GUITransform::Relative(0.8, 0.8),
            content: GUIPanelContent::Text(TextParameters {
                text: "Hello, World!".to_string(),
                color: Color::GREEN,
                scale: 40.0,
                font: FontParameters::Default,
            }),
        };

        let _colored_panel = GUIPanel {
            name: "Test color".to_string(),
            active: true,
            position: GUITransform::Relative(0.01, 0.01),
            dimensions: GUITransform::Relative(0.3, 0.7),
            content: GUIPanelContent::Panels(Color::BLACK, vec![panel_with_text]),
        };

        let mut surface = Surface2D::new(16, 9, Color::WHITE);
        for y in 0..9 {
            for x in 0..16 {
                let color = if (x + y % 2) % 2 == 0 {
                    Color::BLACK
                } else {
                    Color::WHITE
                };
                surface.draw_pixel(Point2::new(x, y), color);
            }
        }

        let graphics_panel = GUIPanel {
            name: "Graphics panel".to_string(),
            active: true,
            position: GUITransform::Relative(0.0, 0.0),
            dimensions: GUITransform::Relative(1.0, 1.0),
            content: GUIPanelContent::Surface2D(surface),
        };

        gui.add_top_level_panels(vec![graphics_panel]);
    }

    fn update(
        &mut self,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
        _dt: f32,
    ) {
        let gui = &mut graphics_engine.renderer_2d;
        // let color_panel = gui.get_panel("Test color");
        //
        // if let Some(color_panel) = color_panel {
        //     if input_handler.is_key_down(&VirtualKeyCode::C) {
        //         color_panel.active = !color_panel.active;
        //     }
        // }
        //
        // let text_panel = gui.get_panel("Test text");
        //
        // if let Some(text_panel) = text_panel {
        //     if input_handler.is_key_down(&VirtualKeyCode::T) {
        //         text_panel.active = !text_panel.active;
        //     }
        //     if input_handler.is_key_down(&VirtualKeyCode::K) {
        //         if let GUIPanelContent::Text(param) = &mut text_panel.content {
        //             param.text = "Something new".to_string();
        //         }
        //     }
        // }

        let surface = gui.get_panel("Graphics panel");
        if let Some(GUIPanel {
            content: GUIPanelContent::Surface2D(surface),
            ..
        }) = surface
        {
            if input_handler.is_key_down(&VirtualKeyCode::H) {
                surface.draw_line((0, 0).into(), (5, 0).into(), Color::GREEN);
            }

            if input_handler.is_key_down(&VirtualKeyCode::V) {
                surface.draw_line((7, 0).into(), (7, 5).into(), Color::GREEN);
            }

            if input_handler.is_key_down(&VirtualKeyCode::I) {
                surface.draw_line((5, 5).into(), (0, 0).into(), Color::RED);
            }

            if input_handler.is_key_down(&VirtualKeyCode::J) {
                surface.draw_line((0, 1).into(), (6, 4).into(), Color::BLUE)
            }
        }
    }
}

fn main() {
    env_logger::init();

    let mut game = Game::new("Test game", 1280, 720, true);

    // let game_object = ObjectController::new();
    // game.add_game_object(game_object);
    //
    // let camera_controller = CameraController {};
    // game.add_game_object(camera_controller);

    let ui = UI {};
    game.add_game_object(ui);

    game.run();
}
