use game_engine::{
    cgmath::{Deg, One, Quaternion, Rotation3},
    gfx::{
        gfx_3d::{InstanceTransform, Mesh, Model, Vertex},
        texture::{Image, Material},
        GraphicsEngine,
    },
    image::load_from_memory,
    input::{InputHandler, VirtualKeyCode},
    GameObject,
};

pub struct PrefabController {}

impl GameObject for PrefabController {
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
            "Square Prefab",
            mesh,
            Some(Material::Textured(Image {
                name: "Stone Bricks".to_string(),
                file: image,
            })),
            None,
        );

        let renderer = &mut graphics_engine.renderer_3d;

        renderer.add_prefab(model);
        if let Some(prefab) = renderer.get_prefab("Square Prefab") {
            prefab.transforms.insert(
                "Instance".to_string(),
                InstanceTransform {
                    position: (0.0, 0.0, 1.0).into(),
                    rotation: Quaternion::one(),
                },
            );
        }
    }

    fn update(
        &mut self,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
        dt: f32,
    ) {
        let renderer = &mut graphics_engine.renderer_3d;

        if let Some(prefab) = renderer.get_prefab("Square Prefab") {
            if let Some(instance) = prefab.transforms.get_mut("Instance") {
                if input_handler.is_key_held(&VirtualKeyCode::O) {
                    instance.position.y += 1.0 * dt;
                }
                if input_handler.is_key_held(&VirtualKeyCode::L) {
                    instance.position.y -= 1.0 * dt;
                }

                if input_handler.is_key_held(&VirtualKeyCode::X) {
                    let mouse_x_delta = input_handler.cursor_delta().x;
                    instance.rotation = instance.rotation
                        * Quaternion::from_angle_x(Deg(mouse_x_delta * dt * 100.0));
                }
                if input_handler.is_key_held(&VirtualKeyCode::Y) {
                    let mouse_x_delta = input_handler.cursor_delta().x;
                    instance.rotation = instance.rotation
                        * Quaternion::from_angle_y(Deg(mouse_x_delta * dt * 100.0));
                }
                if input_handler.is_key_held(&VirtualKeyCode::Z) {
                    let mouse_x_delta = input_handler.cursor_delta().x;
                    instance.rotation = instance.rotation
                        * Quaternion::from_angle_z(Deg(mouse_x_delta * dt * 100.0));
                }
            }

            if input_handler.is_key_down(&VirtualKeyCode::R) {
                prefab.transforms.remove("Instance");
            }
        }
    }
}

pub struct ModelController {}

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
