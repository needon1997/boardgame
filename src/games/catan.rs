use bevy::scene::ron::de;
use bevy::transform::commands;
use bevy::window::WindowResolution;
use bevy::DefaultPlugins;
use bevy::{prelude::*, transform};
use bevy_vector_shapes::prelude::*;

use crate::catan::element::TileKind;

use super::common;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Tile {
    tile_kind: TileKind,
    coordinate: Vec3,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum CatanState {
    #[default]
    Wait,
    Menu,
    BuidSettlement,
    BuildCity,
    BuildRoad,
    Trade,
    SelectRobber,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct CatanPlayerDraw {
    transform: Transform,
    text_id: Option<Entity>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct CatanPlayer {
    name: String,
    color: Color,
    points: usize,
    resources_count: usize,
    cards_count: usize,
    draw: CatanPlayerDraw,
}

#[derive(Resource)]
struct Catan {
    tiles: Vec<Vec<Tile>>,
    points: Vec<Vec<Vec3>>,
    players: Vec<CatanPlayer>,
    radius: Option<f32>,
}

impl Catan {
    fn new(tiles_data: Vec<Vec<TileKind>>, players: Vec<CatanPlayer>) -> Self {
        let mut tiles = Vec::new();
        let mut points = Vec::new();

        for i in 0..tiles_data.len() {
            let mut row = Vec::new();
            for j in 0..tiles_data[0].len() {
                row.push(Tile {
                    tile_kind: tiles_data[i][j],
                    ..Default::default()
                });
            }
            tiles.push(row);
        }
        for i in 0..(tiles_data.len() + 1) {
            let mut row = Vec::new();
            for j in 0..(2 * tiles_data[0].len() + 1) {
                row.push(Default::default());
            }
            points.push(row);
        }

        Self {
            tiles,
            points,
            radius: None,
            players,
        }
    }

    fn update_radius(&mut self, radius: f32) {
        match self.radius {
            Some(r) if r == radius => {},
            _ => {
                self.radius = Some(radius);
                let x_offset = radius;
                let y_offset =
                    ((radius * radius - (radius / 2. * radius / 2.)) as f32).sqrt();
                for i in 0..self.tiles.len() {
                    for j in 0..self.tiles[i].len() {
                        if i % 2 == 0 {
                            self.tiles[i][j].coordinate = Vec3::new(
                                x_offset + 3. * x_offset * (i / 2) as f32,
                                y_offset + 2. * y_offset * j as f32,
                                1.,
                            );
                        } else {
                            self.tiles[i][j].coordinate = Vec3::new(
                                2.5 * x_offset as f32 + 3. * x_offset * (i / 2) as f32,
                                2. * y_offset + 2. * y_offset * j as f32,
                                1.,
                            );
                        }
                    }
                }

                for i in 0..self.points.len() {
                    for j in 0..self.points[i].len() {
                        if i % 2 == 0 && j % 2 == 0 {
                            self.points[i][j] = Vec3::new(
                                x_offset / 2. + 3. * x_offset * (i / 2) as f32,
                                y_offset * j as f32,
                                2.,
                            );
                        } else if i % 2 == 0 && j % 2 == 1 {
                            self.points[i][j] = Vec3::new(
                                3. * x_offset * (i / 2) as f32,
                                y_offset * j as f32,
                                2.,
                            );
                        } else if i % 2 == 1 && j % 2 == 0 {
                            self.points[i][j] = Vec3::new(
                                x_offset / 2. + x_offset + 3. * x_offset * (i / 2) as f32,
                                y_offset * j as f32,
                                2.,
                            );
                        } else if i % 2 == 1 && j % 2 == 1 {
                            self.points[i][j] = Vec3::new(
                                2. * x_offset + 3. * x_offset * (i / 2) as f32,
                                y_offset * j as f32,
                                2.,
                            );
                        }
                    }
                }
            },
        }
    }
}

fn draw_roads(painter: &mut ShapePainter, catan: &ResMut<Catan>) {
    let config = painter.config().clone();

    for i in 0..catan.points.len() {
        for j in 0..catan.points[i].len() {
            painter.set_config(config.clone());
            painter.translate(Vec3::new(catan.points[i][j].x, catan.points[i][j].y, 0.2));
            painter.color = Color::rgb(1.0, 0.0, 1.0);
            painter.circle(catan.radius.unwrap() * 0.2);
            painter.color = Color::rgb(0.0, 1.0, 1.0);
            painter.circle(catan.radius.unwrap() * 0.16);
        }
    }
    painter.set_config(config);
}

fn draw_points(painter: &mut ShapePainter, catan: &ResMut<Catan>) {
    let config = painter.config().clone();

    for i in 0..catan.points.len() {
        for j in 0..catan.points[i].len() {
            painter.set_config(config.clone());
            painter.translate(Vec3::new(catan.points[i][j].x, catan.points[i][j].y, 0.2));
            painter.color = Color::rgb(1.0, 0.0, 1.0);
            painter.circle(catan.radius.unwrap() * 0.2);
            painter.color = Color::rgb(0.0, 1.0, 1.0);
            painter.circle(catan.radius.unwrap() * 0.16);
        }
    }
    painter.set_config(config);
}

fn draw_tiles(painter: &mut ShapePainter, catan: &ResMut<Catan>) {
    let config = painter.config().clone();

    for i in 0..catan.tiles.len() {
        for j in 0..catan.tiles[i].len() {
            painter.set_config(config.clone());
            painter.translate(Vec3::new(
                catan.tiles[i][j].coordinate.x as f32,
                catan.tiles[i][j].coordinate.y as f32,
                0.1,
            ));
            painter.color = Color::rgb(0.0, 1.0, 1.0);
            painter.ngon(6., catan.radius.unwrap());
            painter.color = Color::rgb(0.0, 0.0, 1.0);
            painter.ngon(6., catan.radius.unwrap() * 0.96);
        }
    }
    painter.set_config(config);
}

fn draw_board(
    mut painter: ShapePainter, mut catan: ResMut<Catan>, windows: Query<&Window>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let board_size = window.width().min(window.height()) * 0.7;
        let board_translate = Vec3 {
            x: window.width() / 2. - board_size / 2.,
            y: window.height() / 2. - board_size / 2.,
            z: 0.0,
        };
        let element_translate = Vec3 {
            x: -board_size / 2.0,
            y: -board_size / 2.0,
            z: 0.0,
        };
        painter.translate(board_translate);
        painter
            .rect(Vec2 {
                x: board_size,
                y: board_size,
            })
            .with_children(|child_painter| {
                child_painter.translate(element_translate);

                let radius = board_size / (catan.tiles.len() * 2 + 1) as f32;
                catan.update_radius(radius);
                draw_tiles(child_painter, &catan);
                draw_points(child_painter, &catan);
            });
    }
}

#[derive(Component)]
struct PlayerText;

fn update_player_text(
    mut texts: Query<(Entity, &mut Transform), With<PlayerText>>, catan: Res<Catan>,
) {
    for (entity, mut transform) in texts.iter_mut() {
        for player in catan.players.iter() {
            if entity == player.draw.text_id.unwrap() {
                *transform = player.draw.transform;
                transform.translation.z = 0.3;
            }
        }
    }
}
fn draw_player_text(mut commands: Commands, mut catan: ResMut<Catan>) {
    for player in catan.players.iter_mut() {
        println!(
            "catan.players[i].name: {} transform: {:?}",
            player.name, player.draw.transform
        );
        player.draw.text_id.replace(
            commands
                .spawn((
                    PlayerText,
                    Text2dBundle {
                        text: Text::from_sections([
                            TextSection::new(
                                format!("{}\n", player.name),
                                TextStyle {
                                    font: Default::default(),
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ),
                            TextSection::new(
                                format!(
                                    "resource:{} \t point:{} \t cards:{}",
                                    player.resources_count,
                                    player.points,
                                    player.cards_count,
                                ),
                                TextStyle {
                                    font: Default::default(),
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ),
                        ])
                        .with_justify(JustifyText::Center),
                        transform: player.draw.transform,
                        ..default()
                    },
                ))
                .id(),
        );
    }
}

fn draw_player_board(
    mut painter: ShapePainter, mut catan: ResMut<Catan>, windows: Query<&Window>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let player_board_x_size = window.width().min(window.height()) * 0.3;
        let player_board_y_size = window.height();

        let board_translate = Vec3 {
            x: -(window.width() / 2. - player_board_x_size / 2.),
            y: window.height() / 2. - player_board_y_size / 2.,
            z: 0.0,
        };
        let element_translate = Vec3 {
            x: 0.,
            y: -player_board_y_size / 2. + player_board_y_size * 0.05,
            z: 0.0,
        };

        painter.translate(board_translate);
        painter
            .rect(Vec2 {
                x: player_board_x_size,
                y: player_board_y_size,
            })
            .with_children(|child_painter| {
                child_painter.translate(element_translate);
                let config = child_painter.config().clone();

                for i in 0..catan.players.len() {
                    child_painter.set_config(config.clone());
                    child_painter.color = catan.players[i].color;
                    child_painter.translate(Vec3 {
                        x: 0.0,
                        y: player_board_y_size * i as f32 * 0.1,
                        z: 0.1,
                    });
                    child_painter.rect(Vec2 {
                        x: player_board_x_size * 0.9,
                        y: player_board_y_size * 0.09,
                    });
                    catan.players[i].draw.transform = child_painter.transform;
                }
            });
    }
}

enum Operation {
    BuildSettlement,
    BuildCity,
    BuildRoad,
    Trade,
    BuyCard,
    UseCard,
    EndTurn,
}

struct OperationEntry {
    operation: Operation,
    image: Handle<Image>,
}

#[derive(Resource)]
struct OperationMenu([OperationEntry; 7]);

fn draw_operation(
    mut painter: ShapePainter, catan: ResMut<Catan>, windows: Query<&Window>,
    menu: Res<OperationMenu>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let operation_board_x_size = window.width().min(window.height()) * 0.7;
        let operation_board_y_size = window.height() * 0.1;

        let board_translate = Vec3 {
            x: window.width() / 2. - operation_board_x_size / 2.,
            y: -window.height() * 0.45,
            z: 0.0,
        };

        painter.translate(board_translate);
        painter
            .rect(Vec2 {
                x: operation_board_x_size,
                y: operation_board_y_size,
            })
            .with_children(|spawn_children| {
                let config = spawn_children.config().clone();
                let operation_size = operation_board_x_size / menu.0.len() as f32;
                let xoffset = -operation_board_x_size / 2. + operation_size * 0.5;
                for i in 0..menu.0.len() {
                    let size = Vec2 {
                        x: operation_size * 0.95,
                        y: operation_board_y_size * 0.95,
                    };
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(Vec3 {
                        x: xoffset + operation_size * i as f32,
                        y: 0.0,
                        z: 0.1,
                    });
                    spawn_children.image(menu.0[i].image.clone(), size);
                }
            });
    }
}

fn load_menu_img(asset_server: Res<AssetServer>, mut menu: ResMut<OperationMenu>) {
    const ASSET_PATH: [&str; 7] = [
        "catan/settlement.png",
        "catan/city.png",
        "catan/settlement.png",
        "catan/settlement.png",
        "catan/settlement.png",
        "catan/settlement.png",
        "catan/settlement.png",
    ];
    for i in 0..menu.0.len() {
        menu.0[i].image = asset_server.load(ASSET_PATH[i]);
    }
}

fn limit_frame(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    settings.limiter = bevy_framepace::Limiter::from_framerate(10.0);
}

pub fn catan_run() {
    App::new()
        .init_state::<CatanState>()
        .insert_resource(OperationMenu([
            OperationEntry {
                operation: Operation::BuildSettlement,
                image: Default::default(),
            },
            OperationEntry {
                operation: Operation::BuildCity,
                image: Default::default(),
            },
            OperationEntry {
                operation: Operation::BuildRoad,
                image: Default::default(),
            },
            OperationEntry {
                operation: Operation::Trade,
                image: Default::default(),
            },
            OperationEntry {
                operation: Operation::BuyCard,
                image: Default::default(),
            },
            OperationEntry {
                operation: Operation::UseCard,
                image: Default::default(),
            },
            OperationEntry {
                operation: Operation::EndTurn,
                image: Default::default(),
            },
        ]))
        .insert_resource(Catan::new(
            {
                let mut vec = Vec::new();
                for _ in 0..9 {
                    vec.push([TileKind::Brick; 9].to_vec());
                }
                vec
            },
            vec![
                CatanPlayer {
                    name: "Player 1".to_string(),
                    color: Color::rgb(1.0, 0.0, 0.0),
                    points: 0,
                    resources_count: 0,
                    ..default()
                },
                CatanPlayer {
                    name: "Player 2".to_string(),
                    color: Color::rgb(1.0, 0.0, 0.0),
                    points: 0,
                    resources_count: 0,
                    ..default()
                },
            ],
        ))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution:
                    WindowResolution::new(1000., 1000.).with_scale_factor_override(1.0),
                title: "Catan".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(common::CameraPlugin)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(Shape2dPlugin::default())
        .add_systems(Startup, limit_frame)
        .add_systems(Startup, load_menu_img)
        .add_systems(Startup, draw_player_text)
        .add_systems(
            Update,
            (
                draw_board,
                draw_player_board,
                draw_operation,
                update_player_text,
            ),
        )
        .run();
}
