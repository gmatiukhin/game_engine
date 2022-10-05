use game_engine::{
    cgmath::{InnerSpace, Point2, Vector2},
    gfx::{
        gfx_2d::{FontParameters, Sprite, TextParameters},
        texture::Color,
        GraphicsEngine,
    },
    input::{InputHandler, VirtualKeyCode},
    GameObject, GameState,
};

pub struct Controller2D {
    sprite: Sprite,
    position: Point2<f32>,
}

impl Controller2D {
    pub fn new() -> Self {
        let mut sprite = Sprite::new(20, 20, Color::TRANSPARENT);
        for y in 0..sprite.height() {
            for x in 0..sprite.width() {
                sprite.draw_pixel(
                    (x as i32, y as i32).into(),
                    Color::new(
                        (x * 10) as u8,
                        (y * 10) as u8,
                        0,
                        if x < sprite.width() / 2 { 200 } else { 255 },
                    ),
                );
            }
        }

        Self {
            sprite,
            position: Point2::new(0.0, 0.0),
        }
    }
}

impl GameObject for Controller2D {
    fn start(&mut self, _game_state: &mut GameState, graphics_engine: &mut GraphicsEngine) {
        let renderer_2d = &mut graphics_engine.renderer_2d;

        renderer_2d
            .background()
            .clear(Color::new(26, 178, 255, 255));
    }

    fn update(
        &mut self,
        game_state: &mut GameState,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
    ) {
        let gui = &mut graphics_engine.renderer_2d;

        let surface = gui.foreground();
        surface.clear(Color::TRANSPARENT);

        surface.draw_text(
            &TextParameters {
                text: "Hello world".to_string(),
                color: Color::BLACK,
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

        direction *= speed * game_state.dt();
        self.position += direction;

        surface.draw_sprite(
            &self.sprite,
            (self.position.x as i32, self.position.y as i32).into(),
        );
    }
}
