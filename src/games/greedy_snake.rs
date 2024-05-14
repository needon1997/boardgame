use std::time::Duration;

use bevy::{
    prelude::*, render::camera::CameraPlugin, time::common_conditions::on_timer,
    window::WindowResolution,
};
use rand::random;

const SNAKE_HEAD_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
const FOOD_COLOR: Color = Color::rgb(1.0, 0.0, 1.0);
const SNAKE_SEGMENT_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);
const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}
impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(Component)]
struct SnakeHead {
    direction: Option<Direction>,
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Default, Resource)]
struct LastTailPosition(Option<Position>);

#[derive(Default, Resource)]
struct SnakeSegments(Vec<Entity>);

fn spawn_segment(commands: &mut Commands, position: Position) -> Entity {
    commands
        .spawn((
            SnakeSegment,
            position,
            Size::square(0.65),
            SpriteBundle {
                sprite: Sprite {
                    color: SNAKE_SEGMENT_COLOR,
                    ..default()
                },
                ..default()
            },
        ))
        .id()
}

fn spawn_snake(mut commands: Commands, mut snake: ResMut<SnakeSegments>) {
    *snake = SnakeSegments(vec![
        commands
            .spawn((
                SnakeHead { direction: None },
                SnakeSegment,
                Position { x: 3, y: 3 },
                Size::square(0.8),
                SpriteBundle {
                    sprite: Sprite {
                        color: SNAKE_HEAD_COLOR,
                        ..default()
                    },
                    transform: Transform {
                        scale: Vec3::new(10.0, 10.0, 10.0),
                        ..default()
                    },
                    ..default()
                },
            ))
            .id(),
        spawn_segment(&mut commands, Position { x: 3, y: 2 }),
    ]);
}

fn snake_movement(
    snake: Res<SnakeSegments>, mut tail: ResMut<LastTailPosition>,
    mut positions: Query<&mut Position, With<SnakeSegment>>,
    mut heads: Query<(Entity, &SnakeHead)>,
) {
    let mut tail_pos = None;
    for head in heads.iter_mut() {
        for id in snake.0.iter() {
            let mut pos = positions.get_mut(*id).unwrap();

            if *id == head.0 {
                if let Some(dir) = head.1.direction {
                    tail_pos = Some(*pos);
                    match dir {
                        Direction::Left => pos.x -= 1,
                        Direction::Right => pos.x += 1,
                        Direction::Down => pos.y -= 1,
                        Direction::Up => pos.y += 1,
                    }
                }
            } else if let Some(tail) = tail_pos {
                let tmp = *pos;
                *pos = tail;
                tail_pos = Some(tmp);
            }
        }
        tail.0 = tail_pos;
    }
}

fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>, mut heads: Query<&mut SnakeHead>,
) {
    for mut head in heads.iter_mut() {
        let dir = if keyboard_input.pressed(KeyCode::ArrowLeft) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::ArrowRight) {
            Direction::Right
        } else if keyboard_input.pressed(KeyCode::ArrowDown) {
            Direction::Down
        } else if keyboard_input.pressed(KeyCode::ArrowUp) {
            Direction::Up
        } else {
            return;
        };

        match head.direction {
            Some(d) if d == dir.opposite() => return,
            _ => head.direction = Some(dir),
        }
    }
}

#[derive(Event)]
struct GrowthEvent;

fn snake_eating(
    mut commands: Commands, mut growth_tx: EventWriter<GrowthEvent>,
    food_pos: Query<(Entity, &Position), With<Food>>,
    head_pos: Query<&Position, With<SnakeHead>>,
) {
    for food in food_pos.iter() {
        for head in head_pos.iter() {
            if food.1 == head {
                commands.entity(food.0).despawn();
                growth_tx.send(GrowthEvent);
            }
        }
    }
}

fn snake_growth(
    mut commands: Commands, last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>, mut growth_reader: EventReader<GrowthEvent>,
) {
    for _ in growth_reader.read() {
        segments
            .0
            .push(spawn_segment(&mut commands, last_tail_position.0.unwrap()));
    }
}

#[derive(Component)]
struct Food;

fn food_spawner(mut commands: Commands) {
    commands.spawn((
        Food,
        Position {
            x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
        },
        Size::square(0.8),
        SpriteBundle {
            sprite: Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            transform: Transform {
                scale: Vec3::new(10.0, 10.0, 10.0),
                ..default()
            },
            ..default()
        },
    ));
}

fn size_scaling(windows: Query<&Window>, mut q: Query<(&Size, &mut Transform)>) {
    for window in windows.iter() {
        for (sprite_size, mut transform) in q.iter_mut() {
            transform.scale = Vec3::new(
                sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
                sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
                1.0,
            );
        }
        break;
    }
}

fn position_translation(
    windows: Query<&Window>, mut q: Query<(&Position, &mut Transform)>,
) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    for window in windows.iter() {
        for (pos, mut transform) in q.iter_mut() {
            transform.translation = Vec3::new(
                convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
                convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
                0.0,
            );
        }
        break;
    }
}

pub fn greedy_snake_run() {
    App::new()
        .add_event::<GrowthEvent>()
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution:
                    WindowResolution::new(500., 500.).with_scale_factor_override(1.0),
                title: "Snake Game".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CameraPlugin)
        .add_systems(Startup, spawn_snake)
        .add_systems(
            Update,
            food_spawner.run_if(on_timer(Duration::from_secs_f32(1.))),
        )
        .add_systems(
            Update,
            (
                snake_movement_input,
                snake_movement.run_if(on_timer(Duration::from_secs_f32(1.))),
                snake_eating.after(snake_movement),
                snake_growth.after(snake_eating),
            ),
        )
        .add_systems(PostUpdate, position_translation)
        .add_systems(PostUpdate, size_scaling)
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .run();
}
