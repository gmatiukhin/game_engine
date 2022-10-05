use game_engine::{
    gfx::GraphicsEngine,
    input::{InputHandler, VirtualKeyCode},
    Game, GameObject, GameState, ResizeMode, WindowSettings,
};

mod camera_controller;
mod controller_2d;
mod controller_3d;

use camera_controller::CameraController;
use controller_2d::Controller2D;
use controller_3d::ModelController;
use controller_3d::PrefabController;

struct GameController {}

impl GameObject for GameController {
    fn update(
        &mut self,
        game_state: &mut GameState,
        _graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
    ) {
        if input_handler.is_key_down(&VirtualKeyCode::Escape) {
            game_state.exit();
        }
    }

    fn end(&mut self) {
        println!("Game ended");
    }
}

fn main() {
    let mut game = Game::new(
        "Test game",
        WindowSettings {
            logical_width: 640,
            logical_height: 360,
            resize_mode: ResizeMode::KeepAspectRatio,
        },
    );

    let prefab_controller = PrefabController {};
    game.add_game_object(prefab_controller);

    let model_controller = ModelController {};
    game.add_game_object(model_controller);

    let camera_controller = CameraController {};
    game.add_game_object(camera_controller);

    let ui = Controller2D::new();
    game.add_game_object(ui);

    let game_controller = GameController {};
    game.add_game_object(game_controller);

    game.run();
}
