use game_engine::{
    cgmath::{Deg, InnerSpace, Point3, Rad, Vector3},
    gfx::GraphicsEngine,
    input::{InputHandler, MouseButton, VirtualKeyCode},
    GameObject, GameState,
};
use std::f32::consts::FRAC_PI_2;

pub struct CameraController {}

impl CameraController {
    const SPEED: f32 = 4.0;
    const ZOOM_SPEED: f32 = 16.0;
    const SENSITIVITY: f32 = 0.4;

    const MIN_FOVY_DEG: Deg<f32> = Deg(10.0);
    const MAX_FOVY_DEG: Deg<f32> = Deg(90.0);
    const DEG_PER_ZOOM: Deg<f32> = Deg(15.0);
}

impl GameObject for CameraController {
    fn start(&mut self, _game_state: &mut GameState, graphics_engine: &mut GraphicsEngine) {
        let renderer = &mut graphics_engine.renderer_3d;
        renderer.camera().position = Point3::new(0.0, 0.0, 2.0);
    }

    fn update(
        &mut self,
        game_state: &mut GameState,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
    ) {
        let dt = game_state.dt();
        let renderer = &mut graphics_engine.renderer_3d;
        let camera = renderer.camera();

        let mut translation = Vector3::new(0.0 as f32, 0.0, 0.0);
        if input_handler.is_key_held(&VirtualKeyCode::D) {
            translation.x += 1.0;
        }
        if input_handler.is_key_held(&VirtualKeyCode::A) {
            translation.x += -1.0;
        };

        if input_handler.is_key_held(&VirtualKeyCode::E) {
            translation.y += 1.0;
        }
        if input_handler.is_key_held(&VirtualKeyCode::Q) {
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
