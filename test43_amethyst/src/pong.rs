use amethyst::{
    prelude::*,
    ecs::prelude::World,
    assets::{
        AssetStorage, 
        Handle, 
        Loader
    },
    core::{
        timing::Time, 
        transform::Transform
    },
    renderer::{
        Camera, 
        ImageFormat, 
        SpriteRender, 
        SpriteSheet, 
        SpriteSheetFormat, 
        Texture
    },
    ui::{
        Anchor, 
        TtfFormat, 
        UiText, 
        UiTransform
    }
};
use crate::{
    Ball, 
    Paddle, 
    Side,
    systems::ScoreText, 
    constants::{
        ARENA_HEIGHT, 
        ARENA_WIDTH
    }
};


/// Функция загрузки атласа
fn load_sprite_sheet(world: &mut World) -> Handle<SpriteSheet> {
    // Загружаем спрайт атлас необходимый для рендеринга графики
    // `sprite_sheet` - это атлас
    // `texture_handle` - это клонируемый референс на текстуру

    // Получаем "ресурс" загрузки ресурсов
    let loader = world.read_resource::<Loader>();

    // Получаем текстуру
    let texture_handle = {
        // Получаем хранилище ассетов текстур
        let texture_storage = world.read_resource::<AssetStorage<Texture>>();
        // Получаем текстуру и сохраняем в хранилище
        loader.load(
            "texture/pong_spritesheet.png",
            ImageFormat::default(),
            (),
            &texture_storage,
        )
    };

    // Получаем хранилище для развертки атласа
    let sprite_sheet_store = world.read_resource::<AssetStorage<SpriteSheet>>();
    // Получаем развертку с сохранением в хранилище
    loader.load(
        "texture/pong_spritesheet.ron", // Файл развертки
        SpriteSheetFormat(texture_handle), // Формат на основании текстуры
        (),
        &sprite_sheet_store,
    )
}

/// Инициализация камеры
fn initialise_camera(world: &mut World) {
    // Настраиваем камеру так, чтобы наш экран покрывал все иговое поле и (0,0) был нижним левым углом
    let mut transform = Transform::default();
    transform.set_translation_xyz(ARENA_WIDTH * 0.5, ARENA_HEIGHT * 0.5, 1.0);

    world
        .create_entity()
        .with(Camera::standard_2d(ARENA_WIDTH, ARENA_HEIGHT))
        .with(transform)
        .build();
}

/// Инициализируем ракетки слева и справа
fn initialise_paddles(world: &mut World, sprite_sheet_handle: Handle<SpriteSheet>) {
    use crate::constants::{
        PADDLE_HEIGHT, 
        PADDLE_VELOCITY, 
        PADDLE_WIDTH
    };

    let mut left_transform = Transform::default();
    let mut right_transform = Transform::default();

    // Correctly position the paddles.
    let y = ARENA_HEIGHT / 2.0;
    left_transform.set_translation_xyz(PADDLE_WIDTH * 0.5, y, 0.0);
    right_transform.set_translation_xyz(ARENA_WIDTH - PADDLE_WIDTH * 0.5, y, 0.0);

    // Assign the sprites for the paddles
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet_handle,
        sprite_number: 0, // paddle is the first sprite in the sprite_sheet
    };

    // Create a left plank entity.
    world
        .create_entity()
        .with(sprite_render.clone())
        .with(Paddle {
            velocity: PADDLE_VELOCITY,
            side: Side::Left,
            width: PADDLE_WIDTH,
            height: PADDLE_HEIGHT,
        })
        .with(left_transform)
        .build();

    // Create right plank entity.
    world
        .create_entity()
        .with(sprite_render)
        .with(Paddle {
            velocity: PADDLE_VELOCITY,
            side: Side::Right,
            width: PADDLE_WIDTH,
            height: PADDLE_HEIGHT,
        })
        .with(right_transform)
        .build();
}

/// Initialises one ball in the middle-ish of the arena.
fn initialise_ball(world: &mut World, sprite_sheet_handle: Handle<SpriteSheet>) {
    use crate::constants::{
        BALL_RADIUS, 
        BALL_VELOCITY_X, 
        BALL_VELOCITY_Y
    };

    // Create the translation.
    let mut local_transform = Transform::default();
    local_transform.set_translation_xyz(ARENA_WIDTH / 2.0, ARENA_HEIGHT / 2.0, 0.0);

    // Assign the sprite for the ball
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet_handle,
        sprite_number: 1, // ball is the second sprite on the sprite_sheet
    };

    world
        .create_entity()
        .with(sprite_render)
        .with(Ball {
            radius: BALL_RADIUS,
            velocity: [BALL_VELOCITY_X, BALL_VELOCITY_Y],
        })
        .with(local_transform)
        .build();
}

fn initialise_score(world: &mut World) {
    let font = world.read_resource::<Loader>().load(
        "font/square.ttf",
        TtfFormat,
        (),
        &world.read_resource(),
    );
    let p1_transform = UiTransform::new(
        "P1".to_string(),
        Anchor::TopMiddle,
        Anchor::Middle,
        -50.,
        -50.,
        1.,
        200.,
        50.,
    );

    let p2_transform = UiTransform::new(
        "P2".to_string(),
        Anchor::TopMiddle,
        Anchor::Middle,
        50.,
        -50.,
        1.,
        200.,
        50.,
    );

    let p1_score = world
        .create_entity()
        .with(p1_transform)
        .with(UiText::new(
            font.clone(),
            "0".to_string(),
            [1.0, 1.0, 1.0, 1.0],
            50.,
        ))
        .build();
    let p2_score = world
        .create_entity()
        .with(p2_transform)
        .with(UiText::new(
            font,
            "0".to_string(),
            [1.0, 1.0, 1.0, 1.0],
            50.,
        ))
        .build();
    world.insert(ScoreText { p1_score, p2_score });
}

///////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct Pong {
    ball_spawn_timer: Option<f32>,
    sprite_sheet_handle: Option<Handle<SpriteSheet>>,
}

// Реализуем простое игровое состояние
impl SimpleState for Pong {
    // Старт игрового состояния
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        use crate::audio::initialise_audio;

        // Разворачиваем структуру для получения мира
        let StateData{ world, .. } = data;

        // Устанавливаем время ожидания в 3 секунды до старта мячика
        self.ball_spawn_timer.replace(3.0);

        // Прогружаем страйты необходимые для рендеринга
        // `spritesheet` - это лаяут спрайтов на картинке (атлас)
        // `texture` - это картинка
        self.sprite_sheet_handle.replace(load_sprite_sheet(world));

        // Инициализируем спрайты
        initialise_paddles(world, self.sprite_sheet_handle.clone().unwrap());
        // Камера
        initialise_camera(world);
        // Аудио
        initialise_audio(world);
        // Счет
        initialise_score(world);
    }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        // Получаем оставшееся время обнуляя Option
        if let Some(mut timer) = self.ball_spawn_timer.take() {
            // Если время не истекло, тогда отнимаем время прошедшее с прошлого кадра
            {
                let time = data.world.fetch::<Time>();
                timer -= time.delta_seconds();
            }

            if timer <= 0.0 {
                // Время истекло - создаем мяч
                initialise_ball(data.world, self.sprite_sheet_handle.clone().unwrap());
            } else {
                // Если время не истекло - возвращаем значение в Option
                self.ball_spawn_timer.replace(timer);
            }
        }

        // Состояние менять не надо
        Trans::None
    }
}
