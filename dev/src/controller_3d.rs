use game_engine::{
    cgmath::{One, Quaternion},
    gfx::{
        gfx_3d::components_3d::{Mesh, Model, PrefabInstance, Vertex},
        texture::{Image, Material},
        GraphicsEngine,
    },
    image::load_from_memory,
    input::{InputHandler, VirtualKeyCode},
    GameObject,
};

pub struct PrefabController {
    model: Model,
    movable_instance: Option<PrefabInstance>,
    immovable_instance: Option<PrefabInstance>,
}

impl PrefabController {
    pub fn new() -> Self {
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
