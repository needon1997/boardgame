use bevy::asset::{AssetMetaCheck, LoadState};
use bevy::ecs::world;
use bevy::prelude::*;
use bevy::render::view::visibility;
use bevy::ui::update;
use bevy::window::WindowResolution;
use bevy::DefaultPlugins;
use bevy_consumable_event::{
    ConsumableEventApp, ConsumableEventReader, ConsumableEventWriter,
};
use bevy_vector_shapes::prelude::*;
#[cfg(target_family = "wasm")]
use bevy_web_asset::WebAssetPlugin;
use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
    ops::Deref,
};

use boardgame_common::{
    catan::element::{
        CatanCommon, DevCard, DevelopmentCard, GameAct, GameMsg, GameStart, PlayerCommon,
        SelectRobber, TileKind, Trade, TradeRequest, TradeResponse, TradeTarget,
    },
    element::{Coordinate, Line},
    network::{new_client, ClientMsg, NetworkClientEvent},
};

use crate::common::{CameraPlugin, NetworkClt, Platform, WindowResizePlugin};

const BOARD_LAYER: f32 = 1.0;
const TRADEBPARD_LAYER: f32 = 2.0;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum CatanState {
    #[default]
    Wait,
    InitSettlement,
    InitRoad,
    Menu,
    BuidSettlement,
    BuildCity,
    BuildRoad,
    Trade,
    UseDevelopmentCard,
    SelectRobber,
    Stealing,
    DropResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct CatanPlayerDraw {
    transform: Transform,
    text_id: Option<Entity>,
}

#[derive(Debug, Default)]
struct CatanPlayer {
    inner: PlayerCommon,
    draw: CatanPlayerDraw,
}

#[derive(Debug)]
struct CatanResourceProperty {
    tranform: Transform,
    size: Vec2,
}

#[derive(Debug, Default)]
struct DiceProperty {
    tranform: Transform,
    size: Vec2,
    nubmer: u8,
}

#[derive(Resource)]
struct Catan {
    resources: Vec<CatanResourceProperty>,
    operations: Vec<OperationEntry>,
    dices: [DiceProperty; 2],
    inner: CatanCommon,
    tiles: Vec<Vec<Vec3>>,
    points: Vec<Vec<Vec3>>,
    players: Vec<CatanPlayer>,
    radius: f32,
    me: usize,
    current_turn: usize,
    stealing_candidate: HashSet<usize>,
    selected_yop: Option<TileKind>,
    road_building: Option<Line>,
    used_card: bool,
    drop_cnt: usize,
}

impl Catan {
    fn new(start: GameStart) -> Self {
        let mut draw_tiles = Vec::new();
        let mut logic_points = Vec::new();
        let mut draw_points = Vec::new();

        for _ in 0..start.tile.len() {
            let mut draw_row = Vec::new();

            for _ in 0..start.tile[0].len() {
                draw_row.push(Vec3::default());
            }
            draw_tiles.push(draw_row);
        }
        for _ in 0..(start.tile.len() + 1) {
            let mut logic_row = Vec::new();
            let mut draw_row = Vec::new();
            for _ in 0..(2 * start.tile[0].len() + 1) {
                logic_row.push(Default::default());
                draw_row.push(Vec3::default());
            }
            logic_points.push(logic_row);
            draw_points.push(draw_row);
        }

        let inner = CatanCommon::new(
            start.tile,
            logic_points,
            HashMap::new(),
            start.harbor,
            start.dice_map,
            start.robber,
        );

        Self {
            resources: Default::default(),
            operations: Default::default(),
            dices: Default::default(),
            inner,
            tiles: draw_tiles,
            points: draw_points,
            radius: 0.0,
            players: start
                .players
                .iter()
                .map(|player_common| CatanPlayer {
                    inner: player_common.clone(),
                    draw: CatanPlayerDraw::default(),
                })
                .collect(),
            me: start.you,
            current_turn: 0,
            drop_cnt: 0,
            stealing_candidate: HashSet::new(),
            selected_yop: None,
            road_building: None,
            used_card: false,
        }
    }
}

fn check_build_city(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut catan: ResMut<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            for i in 0..catan.points.len() {
                for j in 0..catan.points[i].len() {
                    if x > catan.points[i][j].x - catan.radius * 0.2
                        && x < catan.points[i][j].x + catan.radius * 0.2
                        && y > catan.points[i][j].y - catan.radius * 0.2
                        && y < catan.points[i][j].y + catan.radius * 0.2
                    {
                        let coordinate = Coordinate { x: i, y: j };
                        let me = catan.me;
                        if catan.inner.point_valid(coordinate)
                            && catan.inner.point(coordinate).owner().is_some()
                            && catan.inner.point(coordinate).owner().unwrap() == me
                        {
                            let me = catan.me;
                            catan.players[me].inner.resources
                                [TileKind::Stone as usize] -= 3;
                            catan.players[me].inner.resources
                                [TileKind::Grain as usize] -= 2;
                            action_writer.send(GameAct::BuildCity(coordinate).into());
                            next_state.set(CatanState::Menu);
                        }
                    }
                }
            }
        }
    }
}

fn check_build_settlement(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut catan: ResMut<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            for i in 0..catan.points.len() {
                for j in 0..catan.points[i].len() {
                    if x > catan.points[i][j].x - catan.radius * 0.2
                        && x < catan.points[i][j].x + catan.radius * 0.2
                        && y > catan.points[i][j].y - catan.radius * 0.2
                        && y < catan.points[i][j].y + catan.radius * 0.2
                    {
                        let coordinate = Coordinate { x: i, y: j };
                        let me = catan.me;
                        if catan.inner.point_valid(coordinate)
                            && catan.inner.point(coordinate).owner().is_none()
                            && catan.players[me].inner.can_build_settlement()
                            && catan.players[me].inner.have_roads_to(coordinate)
                            && !catan.inner.point_get_points(coordinate).iter().any(
                                |&p| {
                                    if let Some(p) = p {
                                        catan.inner.point(p).is_owned()
                                    } else {
                                        false
                                    }
                                },
                            )
                        {
                            let me = catan.me;
                            catan.players[me].inner.resources[TileKind::Wood as usize] -=
                                1;
                            catan.players[me].inner.resources
                                [TileKind::Grain as usize] -= 1;
                            catan.players[me].inner.resources[TileKind::Wool as usize] -=
                                1;
                            catan.players[me].inner.resources
                                [TileKind::Brick as usize] -= 1;
                            action_writer
                                .send(GameAct::BuildSettlement(coordinate).into());
                            next_state.set(CatanState::Menu);
                        }
                    }
                }
            }
        }
    }
}

fn check_init_settlement(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut catan: ResMut<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            for i in 0..catan.points.len() {
                for j in 0..catan.points[i].len() {
                    if x > catan.points[i][j].x - catan.radius * 0.2
                        && x < catan.points[i][j].x + catan.radius * 0.2
                        && y > catan.points[i][j].y - catan.radius * 0.2
                        && y < catan.points[i][j].y + catan.radius * 0.2
                    {
                        let coordinate = Coordinate { x: i, y: j };
                        let me = catan.me;
                        if catan.inner.point_valid(coordinate)
                            && catan.inner.point(coordinate).owner().is_none()
                            && !catan.inner.point_get_points(coordinate).iter().any(
                                |&p| {
                                    if let Some(p) = p {
                                        catan.inner.point(p).is_owned()
                                    } else {
                                        false
                                    }
                                },
                            )
                        {
                            action_writer
                                .send(GameAct::BuildSettlement(coordinate).into());
                            catan.inner.add_settlement(me, coordinate);
                            next_state.set(CatanState::InitRoad);
                        }
                    }
                }
            }
        }
    }
}

fn check_init_road(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    roads: Query<(Entity, &BuildableCatanRoad)>, mut catan: ResMut<Catan>,
    mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);

            for (_, road) in roads.iter() {
                let point = (catan.points[road.line.start.x][road.line.start.y]
                    + catan.points[road.line.end.x][road.line.end.y])
                    / 2.0;
                if x > point.x - catan.radius * 0.25
                    && x < point.x + catan.radius * 0.25
                    && y > point.y - catan.radius * 0.25
                    && y < point.y + catan.radius * 0.25
                {
                    action_writer
                        .send(GameAct::BuildRoad(road.line.start, road.line.end).into());
                    let me = catan.me;
                    catan.inner.add_road(me, road.line);
                    catan.players[me].inner.add_road(road.line);
                    next_state.set(CatanState::Wait);
                    break;
                }
            }
        }
    }
}

fn check_build_road(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut catan: ResMut<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            let mut buildable_road = HashMap::new();

            for (road, player) in catan.inner.roads() {
                if *player == catan.me {
                    for candidate in catan.inner.point_get_points(road.start) {
                        if let Some(candidate) = candidate {
                            let road = Line::new(road.start, candidate);
                            if catan.inner.roads().get(&road).is_none() {
                                buildable_road.insert(road, ());
                            }
                        }
                    }
                    for candidate in catan.inner.point_get_points(road.end) {
                        if let Some(candidate) = candidate {
                            let road = Line::new(road.end, candidate);
                            if catan.inner.roads().get(&road).is_none() {
                                buildable_road.insert(road, ());
                            }
                        }
                    }
                }
            }

            for (road, _) in buildable_road {
                if x > (catan.points[road.start.x][road.start.y]
                    + catan.points[road.end.x][road.end.y])
                    .x
                    / 2.
                    - catan.radius * 0.2
                    && x < (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .x
                        / 2.
                        + catan.radius * 0.2
                    && y > (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .y
                        / 2.
                        - catan.radius * 0.2
                    && y < (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .y
                        / 2.
                        + catan.radius * 0.2
                {
                    let me = catan.me;
                    catan.players[me].inner.resources[TileKind::Brick as usize] -= 1;
                    catan.players[me].inner.resources[TileKind::Wood as usize] -= 1;
                    action_writer.send(GameAct::BuildRoad(road.start, road.end).into());
                    next_state.set(CatanState::Menu);
                    break;
                }
            }
        }
    }
}

fn check_road_building_build_road(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut catan: ResMut<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut next_card_state: ResMut<NextState<UseCardState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            let mut buildable_road = HashMap::new();

            for (road, player) in catan.inner.roads() {
                if *player == catan.me {
                    for candidate in catan.inner.point_get_points(road.start) {
                        if let Some(candidate) = candidate {
                            let road = Line::new(road.start, candidate);
                            if catan.inner.roads().get(&road).is_none() {
                                buildable_road.insert(road, ());
                            }
                        }
                    }
                    for candidate in catan.inner.point_get_points(road.end) {
                        if let Some(candidate) = candidate {
                            let road = Line::new(road.end, candidate);
                            if catan.inner.roads().get(&road).is_none() {
                                buildable_road.insert(road, ());
                            }
                        }
                    }
                }
            }

            for (road, _) in buildable_road {
                if x > (catan.points[road.start.x][road.start.y]
                    + catan.points[road.end.x][road.end.y])
                    .x
                    / 2.
                    - catan.radius * 0.2
                    && x < (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .x
                        / 2.
                        + catan.radius * 0.2
                    && y > (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .y
                        / 2.
                        - catan.radius * 0.2
                    && y < (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .y
                        / 2.
                        + catan.radius * 0.2
                    && Some(road) != catan.road_building
                {
                    if catan.road_building.is_none() {
                        catan.road_building = Some(road);
                    } else {
                        let road1 = catan.road_building.take().unwrap();
                        action_writer.send(
                            GameAct::UseDevelopmentCard((
                                DevCard::RoadBuilding,
                                DevelopmentCard::RoadBuilding([road1, road]),
                            ))
                            .into(),
                        );
                        next_card_state.set(UseCardState::SelectCard);
                        next_state.set(CatanState::Menu);
                    }
                    break;
                }
            }
        }
    }
}

fn check_steal_target(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    catan: Res<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            let offset = -windows.iter().next().unwrap().width() * 0.35;
            let icon_size = windows.iter().next().unwrap().width() * 0.1;
            for (i, player) in catan.stealing_candidate.iter().enumerate() {
                if x > offset + icon_size * i as f32
                    && x < offset + icon_size + icon_size * i as f32
                    && y < icon_size / 2.
                    && y > -icon_size / 2.
                {
                    action_writer.send(
                        GameAct::SelectRobber((Some(*player), catan.inner.robber()))
                            .into(),
                    );
                    next_state.set(CatanState::Menu);
                    break;
                }
            }
        }
    }
}

fn draw_steal_target(
    windows: Query<&Window>, mut painter: ShapePainter, catan: Res<Catan>,
    img_store: Res<ImageStore>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        painter.translate(Vec3::new(0., 0., TRADEBPARD_LAYER));
        painter.rect(Vec2 {
            x: window.width() * 0.7,
            y: window.height() * 0.7,
        });
        let icon_size = window.width() * 0.1;
        for (i, player) in catan.stealing_candidate.iter().enumerate() {
            painter.reset();
            painter.translate(Vec3::new(
                -window.width() * 0.35 + icon_size / 2.0 + icon_size * i as f32,
                0.0,
                TRADEBPARD_LAYER + 0.1,
            ));
            painter.image(
                img_store.settlement_img[*player].clone(),
                Vec2::new(icon_size, icon_size),
            );
        }
    }
}

fn check_select_robber(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut catan: ResMut<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            for i in 0..catan.tiles.len() {
                for j in 0..catan.tiles[i].len() {
                    if x > catan.tiles[i][j].x - catan.radius * 0.9
                        && x < catan.tiles[i][j].x + catan.radius * 0.9
                        && y > catan.tiles[i][j].y - catan.radius * 0.9
                        && y < catan.tiles[i][j].y + catan.radius * 0.9
                    {
                        let coordinate = Coordinate { x: i, y: j };
                        if catan.inner.tile(coordinate).kind() != TileKind::Empty
                            && catan.inner.tile(coordinate).kind() != TileKind::Dessert
                            && catan.inner.robber() != coordinate
                        {
                            catan.inner.set_robber(coordinate);

                            catan.stealing_candidate.clear();
                            for point in catan.inner.tile_get_points(coordinate) {
                                match catan.inner.point(point).owner() {
                                    Some(owner) => {
                                        if owner != catan.me {
                                            catan.stealing_candidate.insert(owner);
                                        }
                                    },
                                    None => continue,
                                }
                            }

                            if !catan.stealing_candidate.is_empty() {
                                next_state.set(CatanState::Stealing);
                            } else {
                                action_writer.send(
                                    GameAct::SelectRobber((None, coordinate)).into(),
                                );
                                next_state.set(CatanState::Menu);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn check_knight_steal_target(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    catan: Res<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut next_card_state: ResMut<NextState<UseCardState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            for (i, player) in catan.stealing_candidate.iter().enumerate() {
                if x > -20.
                    && x < 20.
                    && y > 60. - 40. * i as f32 - 20.
                    && y < 60. - 40. * i as f32 + 20.
                {
                    action_writer.send(
                        GameAct::UseDevelopmentCard((
                            DevCard::Knight,
                            DevelopmentCard::Knight(SelectRobber {
                                player: catan.me,
                                target: Some(*player),
                                coord: catan.inner.robber(),
                            }),
                        ))
                        .into(),
                    );
                    next_card_state.set(UseCardState::SelectCard);
                    next_state.set(CatanState::Menu);
                    break;
                }
            }
        }
    }
}

fn check_knight_select_robber(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut catan: ResMut<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut next_card_state: ResMut<NextState<UseCardState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            print!("check_knight_select_robber x: {} y: {}", x, y);
            for i in 0..catan.tiles.len() {
                for j in 0..catan.tiles[i].len() {
                    if x > catan.tiles[i][j].x - catan.radius * 0.9
                        && x < catan.tiles[i][j].x + catan.radius * 0.9
                        && y > catan.tiles[i][j].y - catan.radius * 0.9
                        && y < catan.tiles[i][j].y + catan.radius * 0.9
                    {
                        let coordinate = Coordinate { x: i, y: j };
                        if catan.inner.tile(coordinate).kind() != TileKind::Empty
                            && catan.inner.tile(coordinate).kind() != TileKind::Dessert
                            && catan.inner.robber() != coordinate
                        {
                            catan.inner.set_robber(coordinate);

                            catan.stealing_candidate.clear();
                            for point in catan.inner.tile_get_points(coordinate) {
                                match catan.inner.point(point).owner() {
                                    Some(owner) => {
                                        if owner != catan.me {
                                            catan.stealing_candidate.insert(owner);
                                        }
                                    },
                                    None => continue,
                                }
                            }

                            if !catan.stealing_candidate.is_empty() {
                                next_card_state.set(UseCardState::KnightStealing);
                            } else {
                                action_writer.send(
                                    GameAct::UseDevelopmentCard((
                                        DevCard::Knight,
                                        DevelopmentCard::Knight(SelectRobber {
                                            player: catan.me,
                                            target: None,
                                            coord: coordinate,
                                        }),
                                    ))
                                    .into(),
                                );
                                next_card_state.set(UseCardState::SelectCard);
                                next_state.set(CatanState::Menu);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Component)]
struct CatanTile {
    x: usize,
    y: usize,
}

impl From<Coordinate> for CatanTile {
    fn from(coord: Coordinate) -> Self {
        CatanTile {
            x: coord.x,
            y: coord.y,
        }
    }
}

#[derive(Component)]
struct CatanPoint {
    x: usize,
    y: usize,
}

impl From<Coordinate> for CatanPoint {
    fn from(coord: Coordinate) -> Self {
        CatanPoint {
            x: coord.x,
            y: coord.y,
        }
    }
}

#[derive(Component)]
struct CatanRobber;

#[derive(Component)]
struct CatanRoad {
    line: Line,
}

#[derive(Component)]
struct BuildableCatanRoad {
    line: Line,
}

fn update_buildable_roads_spawn(
    mut roads: Query<(&mut Transform, &mut Sprite, &BuildableCatanRoad)>,
    catan: Res<Catan>,
) {
    for (mut transform, mut sprite, road) in roads.iter_mut() {
        transform.translation = (catan.points[road.line.start.x][road.line.start.y]
            + catan.points[road.line.end.x][road.line.end.y])
            / 2.0;
        sprite.custom_size = Some(Vec2::new(catan.radius * 0.5, catan.radius));
        transform.rotation = Quat::from_rotation_z(road_get_rotate(
            catan.points[road.line.start.x][road.line.start.y],
            catan.points[road.line.end.x][road.line.end.y],
        ));
        sprite.color = Color::rgba(1.0, 1.0, 1.0, 0.5);
    }
}

fn despawn_buildable_roads(
    mut commands: Commands, roads: Query<(Entity, &BuildableCatanRoad)>,
) {
    info!("despawn_buildable_roads entity: ");
    for (entity, _) in roads.iter() {
        info!("despawn_buildable_roads entity: {:?}", entity);
        commands.entity(entity).despawn();
    }
}

fn spawn_initable_roads(
    mut commands: Commands, catan: Res<Catan>, image_store: Res<ImageStore>,
) {
    for i in 0..catan.points.len() {
        for j in 0..catan.points[i].len() {
            let point = Coordinate { x: i, y: j };
            if let Some(player) = catan.inner.point(point).owner() {
                if player == catan.me {
                    for candidate in catan.inner.point_get_points(point) {
                        if let Some(candidate) = candidate {
                            let road = Line::new(point, candidate);
                            if catan.inner.roads().get(&road).is_none() {
                                commands.spawn((
                                    BuildableCatanRoad { line: road },
                                    SpriteBundle {
                                        texture: image_store.road_img[catan.me].clone(),
                                        sprite: Sprite {
                                            custom_size: Some(Vec2::new(0.0, 0.0)),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn spawn_buildable_roads(
    mut commands: Commands, catan: Res<Catan>, image_store: Res<ImageStore>,
) {
    for (road, player) in catan.inner.roads() {
        if *player == catan.me {
            for candidate in catan.inner.point_get_points(road.start) {
                if let Some(candidate) = candidate {
                    let road = Line::new(road.start, candidate);
                    if catan.inner.roads().get(&road).is_none()
                        && catan.inner.point_valid(candidate)
                        && Some(road) != catan.road_building
                    {
                        commands.spawn((
                            BuildableCatanRoad { line: road },
                            SpriteBundle {
                                texture: image_store.road_img[catan.me].clone(),
                                sprite: Sprite {
                                    custom_size: Some(Vec2::new(0.0, 0.0)),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ));
                    }
                }
            }
            for candidate in catan.inner.point_get_points(road.end) {
                if let Some(candidate) = candidate {
                    let road = Line::new(road.end, candidate);
                    if catan.inner.roads().get(&road).is_none()
                        && catan.inner.point_valid(candidate)
                        && Some(road) != catan.road_building
                    {
                        commands.spawn((
                            BuildableCatanRoad { line: road },
                            SpriteBundle {
                                texture: image_store.road_img[catan.me].clone(),
                                sprite: Sprite {
                                    custom_size: Some(Vec2::new(0.0, 0.0)),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ));
                    }
                }
            }
        }
    }
}

fn update_catan_road_spawn(
    mut roads: Query<(&mut Transform, &mut Sprite, &CatanRoad)>, catan: Res<Catan>,
) {
    for (mut transform, mut sprite, road) in roads.iter_mut() {
        transform.translation = (catan.points[road.line.start.x][road.line.start.y]
            + catan.points[road.line.end.x][road.line.end.y])
            / 2.0;
        sprite.custom_size = Some(Vec2::new(catan.radius * 0.5, catan.radius));
        transform.rotation = Quat::from_rotation_z(road_get_rotate(
            catan.points[road.line.start.x][road.line.start.y],
            catan.points[road.line.end.x][road.line.end.y],
        ));
    }
}

fn road_get_rotate(start: Vec3, end: Vec3) -> f32 {
    let x = end.x - start.x;
    let y = end.y - start.y;
    if x == 0. {
        PI / 2.
    } else {
        (y / x).atan() + PI / 2.
    }
}

fn spawn_catan_road(commands: &mut Commands, road: Line, img: Handle<Image>) {
    commands.spawn((
        CatanRoad { line: road },
        SpriteBundle {
            texture: img,
            sprite: Sprite {
                custom_size: Some(Vec2::new(0.0, 0.0)),
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

#[derive(Component)]
struct CatanHarbor {
    location: Line,
}

impl From<Line> for CatanHarbor {
    fn from(line: Line) -> Self {
        CatanHarbor { location: line }
    }
}

fn update_catan_harbor_spawn(
    mut harbors: Query<(&mut Transform, &mut Sprite, &CatanHarbor)>, catan: Res<Catan>,
) {
    for (mut transform, mut sprite, tile) in harbors.iter_mut() {
        transform.translation = (catan.points[tile.location.start.x]
            [tile.location.start.y]
            + catan.points[tile.location.end.x][tile.location.end.y])
            / 2.0;
        sprite.custom_size = Some(Vec2::new(catan.radius * 0.3, catan.radius * 0.3));
    }
}

fn spawn_catan_harbor(
    commands: &mut Commands, catan: &Catan, img_store: &ResMut<ImageStore>,
) {
    for (line, kind) in catan.inner.harbors() {
        commands.spawn((
            CatanHarbor::from(*line),
            SpriteBundle {
                texture: img_store.resource_img[kind].clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::new(0.0, 0.0)),
                    ..Default::default()
                },
                ..Default::default()
            },
        ));
    }
}

fn update_catan_robber_spawn(
    mut robber: Query<(&mut Transform, &mut Sprite, &CatanRobber)>, catan: Res<Catan>,
) {
    for (mut transform, mut sprite, _) in robber.iter_mut() {
        transform.translation = catan.tiles[catan.inner.robber().x]
            [catan.inner.robber().y]
            + Vec3::new(0.0, 0.0, 0.1);
        sprite.custom_size = Some(Vec2::new(catan.radius * 2.0, catan.radius * 2.0));
    }
}

fn update_catan_tiles_spawn(
    mut tiles: Query<(&mut Transform, &mut Sprite, &CatanTile)>, catan: Res<Catan>,
    state: Res<State<CatanState>>, card_state: Res<State<UseCardState>>,
) {
    for (mut transform, mut sprite, tile) in tiles.iter_mut() {
        transform.translation = catan.tiles[tile.x][tile.y];
        sprite.custom_size = Some(Vec2::new(catan.radius * 1.8, catan.radius * 1.8));

        let coord = Coordinate {
            x: tile.x,
            y: tile.y,
        };

        if (state.eq(&CatanState::SelectRobber) || card_state.eq(&UseCardState::Knight))
            && catan.inner.tile(coord).kind().is_resource()
        {
            sprite.color = Color::rgba(1.0, 1.0, 1.0, 0.5);
        } else {
            sprite.color = Color::rgba(1.0, 1.0, 1.0, 1.0);
        }
    }
}

fn update_catan_points_spawn(
    mut tiles: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Handle<Image>,
        &mut Visibility,
        &CatanPoint,
    )>,
    catan: Res<Catan>, state: Res<State<CatanState>>, img_store: ResMut<ImageStore>,
) {
    for (mut transform, mut sprite, mut img, mut visibility, point) in tiles.iter_mut() {
        *visibility = Visibility::Hidden;
        transform.translation = catan.points[point.x][point.y];
        sprite.custom_size = Some(Vec2::new(catan.radius, catan.radius));
        sprite.color = Color::rgba(1.0, 1.0, 1.0, 1.0);

        let coord = Coordinate {
            x: point.x,
            y: point.y,
        };
        *img = Handle::default();
        if catan.inner.point_valid(coord) && catan.inner.point(coord).owner().is_some() {
            if !catan.inner.point(coord).is_city() {
                *img = img_store.settlement_img
                    [catan.inner.point(coord).owner().unwrap()]
                .clone();
            } else {
                *img =
                    img_store.city_img[catan.inner.point(coord).owner().unwrap()].clone();
            }
            *visibility = Visibility::Visible;
        }
        if state.eq(&CatanState::BuidSettlement)
            && catan.inner.point_valid(coord)
            && catan.inner.point(coord).owner().is_none()
            && catan.players[catan.me].inner.can_build_settlement()
            && catan.players[catan.me].inner.have_roads_to(coord)
            && !catan.inner.point_get_points(coord).iter().any(|&p| {
                if let Some(p) = p {
                    catan.inner.point(p).is_owned()
                } else {
                    false
                }
            })
        {
            *img = img_store.settlement_img[catan.me].clone();
            *visibility = Visibility::Visible;
            sprite.color = Color::rgba(1.0, 1.0, 1.0, 0.5);
        } else if state.eq(&CatanState::BuildCity)
            && catan.inner.point(coord).owner().is_some()
            && catan.inner.point(coord).owner().unwrap() == catan.me
            && !catan.inner.point(coord).is_city()
            && catan.players[catan.me].inner.can_build_city()
        {
            *img = img_store.city_img[catan.me].clone();
            *visibility = Visibility::Visible;
            sprite.color = Color::rgba(1.0, 1.0, 1.0, 0.5);
        } else if state.eq(&CatanState::InitSettlement)
            && catan.inner.point_valid(coord)
            && catan.inner.point(coord).owner().is_none()
            && !catan.inner.point_get_points(coord).iter().any(|&p| {
                if let Some(p) = p {
                    catan.inner.point(p).is_owned()
                } else {
                    false
                }
            })
        {
            *img = img_store.settlement_img[catan.me].clone();
            *visibility = Visibility::Visible;
            sprite.color = Color::rgba(1.0, 1.0, 1.0, 0.5);
        }
    }
}

fn update_catan_tiles_transform(mut catan: ResMut<Catan>, windows: Query<&Window>) {
    for window in windows.iter() {
        let board_size = window.width().min(window.height() * 0.7);
        let translate = Vec3 {
            x: 0.0,
            y: window.height() / 2. - board_size / 2.0,
            z: 0.0,
        } + Vec3 {
            x: -board_size / 2.0,
            y: -board_size / 2.0,
            z: 0.0,
        };
        let radius = board_size
            / if catan.tiles.len() % 2 == 0 {
                (catan.tiles.len() * 3 / 2) as f32 + 0.25
            } else {
                (catan.tiles.len() * 3 / 2) as f32 + 1.
            }
            .max(0.866 * 2. * catan.tiles[0].len() as f32);
        let x_offset = radius;
        let y_offset = ((radius * radius - (radius / 2. * radius / 2.)) as f32).sqrt();
        for i in 0..catan.tiles.len() {
            for j in 0..catan.tiles[i].len() {
                if i % 2 == 0 {
                    catan.tiles[i][j] = Vec3::new(
                        x_offset + 3. * x_offset * (i / 2) as f32,
                        y_offset + 2. * y_offset * j as f32,
                        1.,
                    ) + translate;
                } else {
                    catan.tiles[i][j] = Vec3::new(
                        2.5 * x_offset as f32 + 3. * x_offset * (i / 2) as f32,
                        2. * y_offset + 2. * y_offset * j as f32,
                        1.,
                    ) + translate;
                }
            }
        }

        for i in 0..catan.points.len() {
            for j in 0..catan.points[i].len() {
                if i % 2 == 0 && j % 2 == 0 {
                    catan.points[i][j] = Vec3::new(
                        x_offset / 2. + 3. * x_offset * (i / 2) as f32,
                        y_offset * j as f32,
                        1.2,
                    ) + translate;
                } else if i % 2 == 0 && j % 2 == 1 {
                    catan.points[i][j] = Vec3::new(
                        3. * x_offset * (i / 2) as f32,
                        y_offset * j as f32,
                        1.2,
                    ) + translate;
                } else if i % 2 == 1 && j % 2 == 0 {
                    catan.points[i][j] = Vec3::new(
                        x_offset / 2. + x_offset + 3. * x_offset * (i / 2) as f32,
                        y_offset * j as f32,
                        1.2,
                    ) + translate;
                } else if i % 2 == 1 && j % 2 == 1 {
                    catan.points[i][j] = Vec3::new(
                        2. * x_offset + 3. * x_offset * (i / 2) as f32,
                        y_offset * j as f32,
                        1.2,
                    ) + translate;
                }
            }
        }
        catan.radius = radius;
    }
}

fn spawn_catan_tiles(
    commands: &mut Commands, catan: &mut Catan, img_store: &ResMut<ImageStore>,
) {
    for (i, row) in catan.tiles.iter().enumerate() {
        for (j, _) in row.iter().enumerate() {
            let coord = Coordinate { x: i, y: j };
            let kind = catan.inner.tile(coord).kind();
            if kind != TileKind::Empty {
                commands
                    .spawn((
                        CatanTile::from(coord),
                        SpriteBundle {
                            texture: img_store.resource_img
                                [&catan.inner.tile(coord).kind()]
                                .clone(),
                            sprite: Sprite {
                                custom_size: Some(Vec2::new(0.0, 0.0)),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ))
                    .with_children(|parent| match catan.inner.tile(coord).number() {
                        Some(number) => {
                            parent.spawn(SpriteBundle {
                                texture: img_store.number_img[number].clone(),
                                sprite: Sprite {
                                    custom_size: Some(Vec2::new(0.0, 0.0)),
                                    ..Default::default()
                                },
                                transform: Transform::from_translation(Vec3::new(
                                    0.0, 0.0, 0.1,
                                )),
                                ..Default::default()
                            });
                        },
                        _ => {},
                    });
            }
        }
    }

    for (i, row) in catan.points.iter().enumerate() {
        for (j, _) in row.iter().enumerate() {
            let coord = Coordinate { x: i, y: j };
            commands.spawn((
                CatanPoint::from(coord),
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(0.0, 0.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ));
        }
    }
    commands.spawn((
        CatanRobber,
        SpriteBundle {
            texture: img_store.robber_img.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(0.0, 0.0)),
                ..Default::default()
            },
            ..Default::default()
        },
    ));
    spawn_catan_harbor(commands, catan, img_store);
}

fn intialize_game(mut next_state: ResMut<NextState<CatanLoadState>>) {
    next_state.set(CatanLoadState::Loaded);
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
#[repr(C)]
enum Operation {
    #[default]
    BuildSettlement,
    BuildCity,
    BuildRoad,
    Trade,
    BuyCard,
    UseCard,
    EndTurn,
}

impl From<usize> for Operation {
    fn from(value: usize) -> Self {
        match value {
            0 => Operation::BuildSettlement,
            1 => Operation::BuildCity,
            2 => Operation::BuildRoad,
            3 => Operation::Trade,
            4 => Operation::BuyCard,
            5 => Operation::UseCard,
            6 => Operation::EndTurn,
            _ => unreachable!(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
struct OperationEntry {
    tranform: Transform,
    size: Vec2,
}

fn limit_frame(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    settings.limiter = bevy_framepace::Limiter::from_framerate(10.0);
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, States)]
enum UseCardState {
    #[default]
    SelectCard,
    Knight,
    KnightStealing,
    RoadBuilding,
    YearOfPlenty,
    Monopoly,
}

fn check_year_of_plenty_click(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut catan: ResMut<Catan>, mut next_state: ResMut<NextState<CatanState>>,
    mut next_card_state: ResMut<NextState<UseCardState>>,
    mut action_writer: ConsumableEventWriter<GameAction>, img_store: Res<ImageStore>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            let card_board_x_size = windows.iter().next().unwrap().width();
            let card_board_y_size = windows.iter().next().unwrap().height() * 0.2;
            let card_size = card_board_y_size * 0.9;
            let xoffset = -card_board_x_size / 2. + card_size / 2.;
            let mut i = 0;
            for (kind, _) in img_store.resource_img.iter() {
                if *kind != TileKind::Dessert && *kind != TileKind::Empty {
                    if x > xoffset + card_size * i as f32 - card_size / 2.
                        && x < xoffset + card_size * i as f32 + card_size / 2.
                        && y > -card_board_y_size / 2.
                        && y < card_board_y_size / 2.
                    {
                        if catan.selected_yop.is_none() {
                            catan.selected_yop = Some(*kind);
                        } else if catan.selected_yop.unwrap() != *kind {
                            {
                                action_writer.send(
                                    GameAct::UseDevelopmentCard((
                                        DevCard::YearOfPlenty,
                                        DevelopmentCard::YearOfPlenty(
                                            catan.selected_yop.take().unwrap(),
                                            *kind,
                                        ),
                                    ))
                                    .into(),
                                );
                                next_card_state.set(UseCardState::SelectCard);
                                next_state.set(CatanState::Menu);
                            }
                        }
                        break;
                    }
                    i += 1;
                }
            }
        }
    }
}

fn draw_year_of_plenty(
    mut painter: ShapePainter, catan: Res<Catan>, windows: Query<&Window>,
    img_store: Res<ImageStore>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let card_board_x_size = window.width();
        let card_board_y_size = window.height() * 0.2;

        let board_translate = Vec3 {
            x: 0.0,
            y: 0.0,
            z: TRADEBPARD_LAYER,
        };

        painter.translate(board_translate);
        painter.with_children(|spawn_children| {
            let config = spawn_children.config().clone();
            let card_size = card_board_y_size * 0.9;
            let xoffset = -card_board_x_size / 2. + card_size / 2.;
            let mut i = 0;
            for (kind, img) in img_store.resource_img.iter() {
                if *kind != TileKind::Dessert && *kind != TileKind::Empty {
                    let size = Vec2 {
                        x: card_size,
                        y: card_size,
                    };
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(Vec3 {
                        x: xoffset + card_size * i as f32,
                        y: 0.0,
                        z: 0.1,
                    });
                    if catan.selected_yop.is_some()
                        && catan.selected_yop.unwrap() == *kind
                    {
                        spawn_children.color = Color::rgb(1.0, 1.0, 1.0);
                        spawn_children.rect(Vec2 {
                            x: card_size,
                            y: card_size,
                        });
                        spawn_children.translate(Vec3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.1,
                        });
                    }
                    spawn_children.image(img.clone(), size);
                    spawn_children.translate(Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.1,
                    });
                    spawn_children.image(
                        img_store.number_img
                            [catan.players[catan.me].inner.resources[*kind as usize]]
                            .clone(),
                        Vec2::new(size.x * 0.5, size.y * 0.5),
                    );
                    i += 1;
                }
            }
        });
    }
}

fn check_monopoly_click(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut next_state: ResMut<NextState<CatanState>>,
    mut next_card_state: ResMut<NextState<UseCardState>>,
    mut action_writer: ConsumableEventWriter<GameAction>, img_store: Res<ImageStore>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            let card_board_x_size = windows.iter().next().unwrap().width();
            let card_board_y_size = windows.iter().next().unwrap().height() * 0.2;
            let card_size = card_board_y_size * 0.9;
            let xoffset = -card_board_x_size / 2. + card_size / 2.;
            let mut i = 0;
            for (kind, _) in img_store.resource_img.iter() {
                if *kind != TileKind::Dessert && *kind != TileKind::Empty {
                    if x > xoffset + card_size * i as f32 - card_size / 2.
                        && x < xoffset + card_size * i as f32 + card_size / 2.
                        && y > -card_board_y_size / 2.
                        && y < card_board_y_size / 2.
                    {
                        action_writer.send(
                            GameAct::UseDevelopmentCard((
                                DevCard::Monopoly,
                                DevelopmentCard::Monopoly(*kind),
                            ))
                            .into(),
                        );
                        next_card_state.set(UseCardState::SelectCard);
                        next_state.set(CatanState::Menu);
                        break;
                    }
                    i += 1;
                }
            }
        }
    }
}

fn draw_monopoly(
    mut painter: ShapePainter, catan: ResMut<Catan>, windows: Query<&Window>,
    img_store: Res<ImageStore>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let card_board_x_size = window.width();
        let card_board_y_size = window.height() * 0.2;

        let board_translate = Vec3 {
            x: 0.0,
            y: 0.0,
            z: TRADEBPARD_LAYER,
        };

        painter.translate(board_translate);
        painter.with_children(|spawn_children| {
            let config = spawn_children.config().clone();
            let card_size = card_board_y_size * 0.9;
            let xoffset = -card_board_x_size / 2. + card_size / 2.;
            let mut i = 0;
            for (kind, img) in img_store.resource_img.iter() {
                if *kind != TileKind::Dessert && *kind != TileKind::Empty {
                    let size = Vec2 {
                        x: card_size,
                        y: card_size,
                    };
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(Vec3 {
                        x: xoffset + card_size * i as f32,
                        y: 0.0,
                        z: 0.1,
                    });
                    spawn_children.image(img.clone(), size);
                    spawn_children.translate(Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.1,
                    });
                    spawn_children.image(
                        img_store.number_img
                            [catan.players[catan.me].inner.resources[*kind as usize]]
                            .clone(),
                        Vec2::new(size.x * 0.5, size.y * 0.5),
                    );
                    i += 1;
                }
            }
        });
    }
}

fn check_development_card_click(
    mut commands: Commands, windows: Query<&Window>, image_store: Res<ImageStore>,
    mouse_button_input: Res<ButtonInput<MouseButton>>, mut catan: ResMut<Catan>,
    mut next_state: ResMut<NextState<CatanState>>,
    mut next_card_state: ResMut<NextState<UseCardState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);
            let card_board_x_size = windows.iter().next().unwrap().width();
            let card_board_y_size = windows.iter().next().unwrap().height() * 0.2;
            let card_size = card_board_y_size;
            let xoffset = -card_board_x_size / 2. + card_size / 2.;
            for i in 0..catan.players[catan.me].inner.cards.len() {
                if x > xoffset + card_size * i as f32 - card_size / 2.
                    && x < xoffset + card_size * i as f32 + card_size / 2.
                    && y > -card_board_y_size / 2.
                    && y < card_board_y_size / 2.
                    && catan.players[catan.me].inner.cards[i] > 0
                {
                    if i == DevCard::Knight as usize {
                        next_card_state.set(UseCardState::Knight);
                        catan.used_card = true;
                        break;
                    } else if i == DevCard::RoadBuilding as usize {
                        next_card_state.set(UseCardState::RoadBuilding);
                        (&mut commands, &catan, &image_store, false);
                        catan.used_card = true;
                        break;
                    } else if i == DevCard::YearOfPlenty as usize {
                        next_card_state.set(UseCardState::YearOfPlenty);
                        catan.used_card = true;
                        break;
                    } else if i == DevCard::Monopoly as usize {
                        next_card_state.set(UseCardState::Monopoly);
                        catan.used_card = true;
                        break;
                    } else if i == DevCard::VictoryPoint as usize {
                        action_writer.send(
                            GameAct::UseDevelopmentCard((
                                DevCard::VictoryPoint,
                                DevelopmentCard::VictoryPoint,
                            ))
                            .into(),
                        );
                        next_state.set(CatanState::Menu);
                        catan.used_card = true;
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Component)]
struct CatanDevelopmentCard {
    kind: DevCard,
}

#[derive(Component)]
struct CatanDevelopmentCardCnt;

#[derive(Component)]
struct CatanUseDevelopmentCard;

fn update_development_card_cnt(
    windows: Query<&Window>,
    mut dev_cards: Query<(&mut Sprite, &CatanDevelopmentCardCnt)>,
) {
    for window in windows.iter() {
        let card_board_y_size = window.height() * 0.2;
        for (mut sprite, _) in dev_cards.iter_mut() {
            sprite.custom_size =
                Some(Vec2::new(card_board_y_size / 2.0, card_board_y_size / 2.0));
        }
    }
}

fn update_development_card(
    windows: Query<&Window>,
    mut dev_cards: Query<(&mut Transform, &mut Sprite, &CatanDevelopmentCard)>,
) {
    for window in windows.iter() {
        let card_board_x_size = window.width();
        let card_board_y_size = window.height() * 0.2;

        let board_translate = Vec3 {
            x: 0.0,
            y: 0.0,
            z: TRADEBPARD_LAYER,
        };

        let xoffset = -card_board_x_size / 2. + card_board_y_size / 2.;
        for (mut transform, mut sprite, dev_card) in dev_cards.iter_mut() {
            transform.translation = Vec3 {
                x: xoffset + card_board_y_size * (dev_card.kind as u8) as f32,
                y: 0.0,
                z: 0.1,
            } + board_translate;
            sprite.custom_size = Some(Vec2::new(card_board_y_size, card_board_y_size));
        }
    }
}

fn update_use_development_card(
    windows: Query<&Window>,
    mut dev_cards: Query<(&mut Transform, &mut Sprite, &CatanUseDevelopmentCard)>,
) {
    for window in windows.iter() {
        let card_board_x_size = window.width();
        let card_board_y_size = window.height() * 0.2;

        let board_translate = Vec3 {
            x: 0.0,
            y: 0.0,
            z: TRADEBPARD_LAYER,
        };
        let (mut transform, mut sprite, _) = dev_cards.single_mut();
        transform.translation = board_translate;
        sprite.custom_size = Some(Vec2::new(card_board_x_size, card_board_y_size));
    }
}

fn despawn_use_development_card(
    mut commands: Commands, roads: Query<(Entity, &CatanUseDevelopmentCard)>,
) {
    for (entity, _) in roads.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_use_development_card(
    mut commands: Commands, catan: Res<Catan>, img_store: Res<ImageStore>,
) {
    commands
        .spawn((
            CatanUseDevelopmentCard,
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.0, 0.0, 0.0, 0.5),
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .with_children(|command| {
            for i in 0..catan.players[catan.me].inner.cards.len() {
                let kind = DevCard::try_from(i as u8).unwrap();
                command
                    .spawn((
                        CatanDevelopmentCard { kind },
                        SpriteBundle {
                            texture: img_store.card_img[i].clone(),
                            ..Default::default()
                        },
                    ))
                    .with_children(|command| {
                        command.spawn((
                            CatanDevelopmentCardCnt,
                            SpriteBundle {
                                texture: img_store.number_img
                                    [catan.players[catan.me].inner.cards[i]]
                                    .clone(),
                                transform: Transform::from_translation(Vec3::new(
                                    0.0, 0.0, 0.1,
                                )),
                                ..Default::default()
                            },
                        ));
                    });
            }
        });
}

fn draw_development_card(
    mut painter: ShapePainter, catan: ResMut<Catan>, windows: Query<&Window>,
    img_store: Res<ImageStore>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let card_board_x_size = window.width();
        let card_board_y_size = window.height() * 0.2;

        let board_translate = Vec3 {
            x: 0.0,
            y: 0.0,
            z: TRADEBPARD_LAYER,
        };

        painter.translate(board_translate);
        painter.with_children(|spawn_children| {
            let config = spawn_children.config().clone();
            let card_size = card_board_y_size * 0.9;
            let xoffset = -card_board_x_size / 2. + card_size / 2.;
            for (i, card) in img_store.card_img.iter().enumerate() {
                let size = Vec2 {
                    x: card_size,
                    y: card_size,
                };
                spawn_children.set_config(config.clone());
                spawn_children.translate(Vec3 {
                    x: xoffset + card_size * i as f32,
                    y: 0.0,
                    z: 0.1,
                });
                spawn_children.image(card.clone(), size);
                spawn_children.translate(Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.1,
                });
                spawn_children.image(
                    img_store.number_img[catan.players[catan.me].inner.cards[i]].clone(),
                    Vec2::new(size.x * 0.5, size.y * 0.5),
                );
            }
        });
    }
}

#[derive(Component)]
struct CatanResource {
    kind: TileKind,
}

#[derive(Component)]
struct CatanResourceCnt {
    kind: TileKind,
}

fn update_catan_resources_spawn(
    mut resources: Query<(&mut Transform, &mut Sprite, &CatanResource)>,
    catan: Res<Catan>,
) {
    for (mut transform, mut sprite, res) in resources.iter_mut() {
        *transform = catan.resources[res.kind as usize].tranform;
        sprite.custom_size = Some(catan.resources[res.kind as usize].size);
    }
}

fn update_catan_resources_cnt_spawn(
    mut resources: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Handle<Image>,
        &CatanResourceCnt,
    )>,
    catan: Res<Catan>, img_store: Res<ImageStore>,
) {
    for (mut transform, mut sprite, mut img, res_cnt) in resources.iter_mut() {
        *img = img_store.number_img
            [catan.players[catan.me].inner.resources[res_cnt.kind as usize]]
            .clone();
        *transform = catan.resources[res_cnt.kind as usize].tranform;
        sprite.custom_size = Some(catan.resources[res_cnt.kind as usize].size);
    }
}

fn update_catan_resources_transform(mut catan: ResMut<Catan>, windows: Query<&Window>) {
    for window in windows.iter() {
        let reousece_size = window.width().min(window.height()) * 0.1;

        let board_translate = Vec3 {
            x: 0.0,
            y: -window.height() * 0.35,
            z: 0.0,
        };

        let xoffset = -reousece_size * 2.5 + reousece_size * 0.5;
        let mut i = 0;
        for (j, resource) in catan.resources.iter_mut().enumerate() {
            let kind = TileKind::try_from(j as u8).unwrap();
            if kind.is_resource() {
                resource.tranform = Transform {
                    translation: Vec3 {
                        x: xoffset + reousece_size * i as f32,
                        y: 0.0,
                        z: 0.1,
                    } + board_translate,
                    ..Default::default()
                };
                resource.size = Vec2::new(reousece_size, reousece_size);
                i += 1;
            }
        }
    }
}

fn spawn_catan_resource(commands: &mut Commands, tile: TileKind, image: Handle<Image>) {
    commands.spawn((
        CatanResource { kind: tile },
        SpriteBundle {
            texture: image,
            sprite: Sprite {
                custom_size: Some(Vec2::new(0.0, 0.0)),
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

fn spawn_catan_resource_cnt(
    commands: &mut Commands, tile: TileKind, image: Handle<Image>,
) {
    commands.spawn((
        CatanResourceCnt { kind: tile },
        SpriteBundle {
            texture: image,
            sprite: Sprite {
                custom_size: Some(Vec2::new(0.0, 0.0)),
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

fn spawn_catan_resources(
    commands: &mut Commands, catan: &mut Catan, img_store: &ResMut<ImageStore>,
) {
    for j in 0..catan.players[catan.me].inner.resources.len() {
        let kind = &TileKind::try_from(j as u8).unwrap();
        catan.resources.push(CatanResourceProperty {
            tranform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            size: Vec2::new(0.0, 0.0),
        });
        if kind.is_resource() {
            spawn_catan_resource(commands, *kind, img_store.resource_img[kind].clone());
            spawn_catan_resource_cnt(commands, *kind, img_store.number_img[0].clone());
        }
    }
}

fn check_menu_click(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    state: Res<State<CatanState>>, mut next_state: ResMut<NextState<CatanState>>,
    mut next_trade_state: ResMut<NextState<TradeState>>,
    mut next_card_state: ResMut<NextState<UseCardState>>, catan: Res<Catan>,
    mut trade: ResMut<TradeBoard>, mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);

            for i in 0..catan.operations.len() {
                let entry = &catan.operations[i];
                if x > entry.tranform.translation.x - entry.size.x / 2.
                    && x < entry.tranform.translation.x + entry.size.x / 2.
                    && y > entry.tranform.translation.y - entry.size.y / 2.
                    && y < entry.tranform.translation.y + entry.size.y / 2.
                {
                    match Operation::from(i) {
                        Operation::BuildSettlement => {
                            if catan.players[catan.me].inner.can_build_settlement() {
                                if state.eq(&CatanState::BuidSettlement) {
                                    next_state.set(CatanState::Menu);
                                } else {
                                    next_state.set(CatanState::BuidSettlement);
                                }
                            }
                        },
                        Operation::BuildCity => {
                            if catan.players[catan.me].inner.can_build_city() {
                                if state.eq(&CatanState::BuildCity) {
                                    next_state.set(CatanState::Menu);
                                } else {
                                    next_state.set(CatanState::BuildCity);
                                }
                            }
                        },
                        Operation::BuildRoad => {
                            if catan.players[catan.me].inner.can_build_road() {
                                if state.eq(&CatanState::BuildRoad) {
                                    next_state.set(CatanState::Menu);
                                } else {
                                    next_state.set(CatanState::BuildRoad);
                                }
                            }
                        },
                        Operation::Trade => {
                            if catan.players[catan.me].inner.can_trade() {
                                if state.eq(&CatanState::Trade) {
                                    next_state.set(CatanState::Menu);
                                } else {
                                    next_state.set(CatanState::Trade);
                                    trade.clear();
                                    next_trade_state.set(TradeState::Offering);
                                }
                            }
                        },
                        Operation::BuyCard => {
                            let me = catan.me;
                            if catan.players[me].inner.can_buy_development_card() {
                                action_writer.send(GameAct::BuyDevelopmentCard.into());
                            }
                        },
                        Operation::UseCard => {
                            if catan.players[catan.me].inner.can_use_development_card()
                                && !catan.used_card
                            {
                                if state.eq(&CatanState::UseDevelopmentCard) {
                                    next_state.set(CatanState::Menu);
                                } else {
                                    next_card_state.set(UseCardState::SelectCard);
                                    next_state.set(CatanState::UseDevelopmentCard);
                                }
                            }
                        },
                        Operation::EndTurn => {
                            next_state.set(CatanState::Wait);
                            action_writer.send(GameAct::EndTurn.into());
                        },
                    }
                    break;
                }
            }
        }
    }
}

fn update_operation_spawn(
    mut operations: Query<(&mut Transform, &mut Sprite, &Operation)>, catan: Res<Catan>,
) {
    for (mut transform, mut sprite, operation) in operations.iter_mut() {
        *transform = catan.operations[*operation as usize].tranform;
        sprite.custom_size = Some(catan.operations[*operation as usize].size);
    }
}

fn update_operation_transform(mut catan: ResMut<Catan>, windows: Query<&Window>) {
    for window in windows.iter() {
        let operation_size = window.width().min(window.height()) * 0.1;

        let board_translate = Vec3 {
            x: 0.0,
            y: -window.height() * 0.45,
            z: 0.0,
        };

        let xoffset = -operation_size * 3.5 + operation_size * 0.5;
        for (i, operation) in catan.operations.iter_mut().enumerate() {
            operation.tranform = Transform {
                translation: Vec3 {
                    x: xoffset + operation_size * i as f32,
                    y: 0.0,
                    z: 0.1,
                } + board_translate,
                ..Default::default()
            };
            operation.size = Vec2::new(operation_size, operation_size);
        }
    }
}

fn spawn_operation(
    commands: &mut Commands, operation: Operation, img_store: &ResMut<ImageStore>,
) {
    commands.spawn((
        operation,
        SpriteBundle {
            texture: img_store.operation_img[&operation].clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(0.0, 0.0)),
                ..Default::default()
            },
            visibility: Visibility::Hidden,
            ..Default::default()
        },
    ));
}

fn spawn_operations(
    commands: &mut Commands, catan: &mut Catan, img_store: &ResMut<ImageStore>,
) {
    for i in 0..7 {
        spawn_operation(commands, Operation::from(i), img_store);
        catan.operations.push(OperationEntry {
            tranform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            size: Vec2::new(0.0, 0.0),
        });
    }
}

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
enum Dice {
    Dice1,
    Dice2,
}

fn update_dices_spawn(
    mut operations: Query<(&mut Transform, &mut Sprite, &Dice)>, catan: Res<Catan>,
) {
    for (mut transform, mut sprite, dice) in operations.iter_mut() {
        *transform = catan.dices[*dice as usize].tranform;
        sprite.custom_size = Some(catan.dices[*dice as usize].size);
    }
}

fn update_dices_transform(mut catan: ResMut<Catan>, windows: Query<&Window>) {
    for window in windows.iter() {
        let board_translate = Vec3 {
            x: 0.0,
            y: -window.height() * 0.25,
            z: 0.0,
        };
        let dice_size = window.width().min(window.height()) * 0.1;
        catan.dices[Dice::Dice1 as usize].tranform =
            Transform::from_translation(board_translate - Vec3::new(dice_size, 0.0, 0.0));
        catan.dices[Dice::Dice1 as usize].size = Vec2::new(dice_size, dice_size);
        catan.dices[Dice::Dice2 as usize].tranform =
            Transform::from_translation(board_translate + Vec3::new(dice_size, 0.0, 0.0));

        catan.dices[Dice::Dice2 as usize].size = Vec2::new(dice_size, dice_size);
    }
}

fn spawn_dices(commands: &mut Commands, _: &mut Catan, img_store: &ResMut<ImageStore>) {
    commands.spawn((
        Dice::Dice1,
        SpriteBundle {
            texture: img_store.dice_img[1].clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(0.0, 0.0)),
                ..Default::default()
            },
            ..Default::default()
        },
    ));
    commands.spawn((
        Dice::Dice2,
        SpriteBundle {
            texture: img_store.dice_img[1].clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(0.0, 0.0)),
                ..Default::default()
            },
            visibility: Visibility::Hidden,
            ..Default::default()
        },
    ));
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum TradeState {
    #[default]
    Offering,
    WaitingResponse,
    Confirming,
    Accepting,
    WaitingConfirm,
}

#[derive(Default)]
struct TradeBoardDraw {
    offer: HashMap<TileKind, Vec3>,
    want: HashMap<TileKind, Vec3>,
    button_size: f32,
    trade_size: f32,
}

#[derive(Default)]
struct TradeBoardResource {
    offer: [u8; TileKind::Max as usize],
    want: [u8; TileKind::Max as usize],
    response: HashMap<usize, TradeResponse>,
}

#[derive(Resource, Default)]
struct TradeBoard {
    draw: TradeBoardDraw,
    resource: TradeBoardResource,
}

impl TradeBoard {
    fn clear(&mut self) {
        self.resource = TradeBoardResource::default();
        self.draw = TradeBoardDraw::default();
    }
}

fn check_trade_confirm_click(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    players: Query<&TradeTargetPlayerMarker>,
    mut next_trade_state: ResMut<NextState<TradeState>>, trade: Res<TradeBoard>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);

            for player in players.iter() {
                if x > -trade.draw.trade_size / 2.0
                    && x < trade.draw.trade_size / 2.0
                    && y > trade.draw.trade_size * (2.0 - (player.id as u8) as f32)
                    && y < trade.draw.trade_size * (1.0 - (player.id as u8) as f32)
                    && trade.resource.response.get(&(player.id as usize)).is_some()
                    && trade.resource.response[&(player.id as usize)]
                        == TradeResponse::Accept
                {
                    action_writer.send(GameAct::TradeConfirm(Some(player.player)).into());
                    next_trade_state.set(TradeState::WaitingConfirm);
                    return;
                }
            }
        }
    }
}

fn check_trade_offering_click(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut trade: ResMut<TradeBoard>, catan: ResMut<Catan>,
    mut next_trade_state: ResMut<NextState<TradeState>>,
    mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);

            for (kind, translate) in trade.draw.offer.iter() {
                if x > translate.x + trade.draw.trade_size - trade.draw.button_size
                    && x < translate.x + trade.draw.trade_size + trade.draw.button_size
                    && y > translate.y + trade.draw.trade_size / 4.0
                        - trade.draw.button_size
                    && y < translate.y
                        + trade.draw.trade_size / 4.0
                        + trade.draw.button_size
                {
                    let k = kind.clone();

                    if catan.players[catan.me].inner.resources[k as usize]
                        > trade.resource.offer[k as usize] as usize
                    {
                        trade.resource.offer[k as usize] += 1;
                        trade.resource.offer[k as usize] =
                            trade.resource.offer[k as usize].min(20);
                    }
                    return;
                }
                if x > translate.x + trade.draw.trade_size - trade.draw.button_size
                    && x < translate.x + trade.draw.trade_size + trade.draw.button_size
                    && y > translate.y
                        - trade.draw.trade_size / 4.0
                        - trade.draw.button_size
                    && y < translate.y - trade.draw.trade_size / 4.0
                        + trade.draw.button_size
                {
                    let k = kind.clone();
                    trade.resource.offer[k as usize] =
                        trade.resource.offer[k as usize].saturating_sub(1);
                    return;
                }
            }

            for (kind, translate) in trade.draw.want.iter() {
                if x > translate.x - trade.draw.trade_size - trade.draw.button_size
                    && x < translate.x - trade.draw.trade_size + trade.draw.button_size
                    && y > translate.y + trade.draw.trade_size / 4.0
                        - trade.draw.button_size
                    && y < translate.y
                        + trade.draw.trade_size / 4.0
                        + trade.draw.button_size
                {
                    let k = kind.clone();
                    trade.resource.want[k as usize] += 1;
                    trade.resource.want[k as usize] =
                        trade.resource.want[k as usize].min(20);
                    return;
                }
                if x > translate.x - trade.draw.trade_size - trade.draw.button_size
                    && x < translate.x - trade.draw.trade_size + trade.draw.button_size
                    && y > translate.y
                        - trade.draw.trade_size / 4.0
                        - trade.draw.button_size
                    && y < translate.y - trade.draw.trade_size / 4.0
                        + trade.draw.button_size
                {
                    let k = kind.clone();
                    trade.resource.want[k as usize] =
                        trade.resource.want[k as usize].saturating_sub(1);
                    return;
                }
            }

            if x > -trade.draw.trade_size / 2.0
                && x < trade.draw.trade_size / 2.0
                && y > trade.draw.trade_size
                && y < trade.draw.trade_size * 2.0
            {
                action_writer.send(
                    GameAct::TradeRequest(TradeRequest::new(
                        trade
                            .resource
                            .offer
                            .iter()
                            .enumerate()
                            .map(|(i, count)| {
                                (TileKind::try_from(i as u8).unwrap(), *count as usize)
                            })
                            .collect(),
                        trade
                            .resource
                            .want
                            .iter()
                            .enumerate()
                            .map(|(i, count)| {
                                (TileKind::try_from(i as u8).unwrap(), *count as usize)
                            })
                            .collect(),
                        TradeTarget::Bank,
                    ))
                    .into(),
                );
                next_trade_state.set(TradeState::WaitingResponse);
                return;
            }

            if x > -trade.draw.trade_size / 2.0
                && x < trade.draw.trade_size / 2.0
                && y > 0.0
                && y < trade.draw.trade_size
            {
                action_writer.send(
                    GameAct::TradeRequest(TradeRequest::new(
                        trade
                            .resource
                            .offer
                            .iter()
                            .enumerate()
                            .map(|(i, count)| {
                                (TileKind::try_from(i as u8).unwrap(), *count as usize)
                            })
                            .collect(),
                        trade
                            .resource
                            .want
                            .iter()
                            .enumerate()
                            .map(|(i, count)| {
                                (TileKind::try_from(i as u8).unwrap(), *count as usize)
                            })
                            .collect(),
                        TradeTarget::Harbor,
                    ))
                    .into(),
                );
                next_trade_state.set(TradeState::WaitingResponse);
                return;
            }

            if x > -trade.draw.trade_size / 2.0
                && x < trade.draw.trade_size / 2.0
                && y > -trade.draw.trade_size
                && y < 0.0
            {
                action_writer.send(
                    GameAct::TradeRequest(TradeRequest::new(
                        trade
                            .resource
                            .offer
                            .iter()
                            .enumerate()
                            .map(|(i, count)| {
                                (TileKind::try_from(i as u8).unwrap(), *count as usize)
                            })
                            .collect(),
                        trade
                            .resource
                            .want
                            .iter()
                            .enumerate()
                            .map(|(i, count)| {
                                (TileKind::try_from(i as u8).unwrap(), *count as usize)
                            })
                            .collect(),
                        TradeTarget::Player,
                    ))
                    .into(),
                );
                next_trade_state.set(TradeState::WaitingResponse);
                return;
            }

            if x > -trade.draw.trade_size / 2.0
                && x < trade.draw.trade_size / 2.0
                && y > -trade.draw.trade_size * 2.0
                && y < -trade.draw.trade_size
            {
                next_state.set(CatanState::Menu);
                return;
            }
        }
    }
}

fn check_trade_accepting_click(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    trade: Res<TradeBoard>, mut next_trade_state: ResMut<NextState<TradeState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);

            if x > -trade.draw.trade_size / 2.0
                && x < trade.draw.trade_size / 2.0
                && y > -trade.draw.trade_size
                && y < 0.0
            {
                action_writer.send(GameAct::TradeResponse(TradeResponse::Accept).into());
                next_trade_state.set(TradeState::WaitingConfirm);
                return;
            }

            if x > -trade.draw.trade_size / 2.0
                && x < trade.draw.trade_size / 2.0
                && y > -trade.draw.trade_size * 2.0
                && y < -trade.draw.trade_size
            {
                action_writer.send(GameAct::TradeResponse(TradeResponse::Accept).into());
                next_trade_state.set(TradeState::WaitingConfirm);
                return;
            }
        }
    }
}

#[derive(Component)]
struct TradeMarker;

#[derive(Component)]
struct TradeResourceMarker {
    tile: TileKind,
    is_offer: bool,
}

#[derive(Component)]
struct TradeResourceCntMarker {
    tile: TileKind,
    is_offer: bool,
}

#[derive(Component)]
struct TradeResourceTradeCntMarker {
    tile: TileKind,
    is_offer: bool,
}

#[derive(Component)]
struct TradeAddMarker {
    is_offer: bool,
}

#[derive(Component)]
struct TradeSubMarker {
    is_offer: bool,
}

#[derive(Component, Copy, Clone)]
#[repr(u8)]
enum TradeOption {
    Bank,
    Harbor,
    Player,
    Cancel,
}

#[derive(Component, Copy, Clone)]
struct TradeTargetPlayerMarker {
    id: usize,
    player: usize,
}

#[derive(Component)]
struct TradeResponseMarker {
    player: usize,
}

fn update_trade_response_spawn(
    mut resources: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
        &mut Handle<Image>,
        &TradeResponseMarker,
    )>,
    trade: Res<TradeBoard>, state: Res<State<TradeState>>, img_store: Res<ImageStore>,
) {
    for (mut transform, mut sprite, mut visibility, mut img, response) in
        resources.iter_mut()
    {
        transform.translation = Vec3::new(0.0, -trade.draw.trade_size * 0.5, 0.1);
        sprite.custom_size = Some(Vec2::new(
            trade.draw.trade_size * 0.5,
            trade.draw.trade_size * 0.5,
        ));

        if state.eq(&TradeState::Offering)
            || state.eq(&TradeState::Accepting)
            || trade.resource.response.get(&response.player).is_none()
        {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Visible;
            *img = match trade.resource.response[&response.player] {
                TradeResponse::Accept => img_store.yes.clone(),
                TradeResponse::Reject => img_store.no.clone(),
            }
        }
    }
}

fn update_trade_target_player_spawn(
    mut resources: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
        &TradeTargetPlayerMarker,
    )>,
    trade: Res<TradeBoard>, state: Res<State<TradeState>>,
) {
    for (mut transform, mut sprite, mut visibility, player) in resources.iter_mut() {
        transform.translation = Vec3::new(
            0.0,
            trade.draw.trade_size * (1.5 - (player.id as u8) as f32),
            0.1,
        );
        sprite.custom_size =
            Some(Vec2::new(trade.draw.trade_size, trade.draw.trade_size));
        if state.eq(&TradeState::Offering) || state.eq(&TradeState::Accepting) {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Visible;
        }
    }
}

fn update_trade_option_spawn(
    mut resources: Query<(&mut Transform, &mut Sprite, &mut Visibility, &TradeOption)>,
    trade: Res<TradeBoard>, catan: Res<Catan>, state: Res<State<TradeState>>,
) {
    for (mut transform, mut sprite, mut visibility, option) in resources.iter_mut() {
        transform.translation = Vec3::new(
            0.0,
            trade.draw.trade_size * (1.5 - (*option as u8) as f32),
            0.1,
        );
        sprite.custom_size =
            Some(Vec2::new(trade.draw.trade_size, trade.draw.trade_size));
        match option {
            TradeOption::Bank => {
                if state.eq(&TradeState::Offering)
                    && catan
                        .inner
                        .check_valid_local_trade(
                            &Trade {
                                from: catan.me,
                                to: None,
                                request: TradeRequest::new(
                                    trade
                                        .resource
                                        .offer
                                        .iter()
                                        .enumerate()
                                        .map(|(i, count)| {
                                            (
                                                TileKind::try_from(i as u8).unwrap(),
                                                *count as usize,
                                            )
                                        })
                                        .collect(),
                                    trade
                                        .resource
                                        .want
                                        .iter()
                                        .enumerate()
                                        .map(|(i, count)| {
                                            (
                                                TileKind::try_from(i as u8).unwrap(),
                                                *count as usize,
                                            )
                                        })
                                        .collect(),
                                    TradeTarget::Bank,
                                ),
                            },
                            &catan.players[catan.me].inner,
                        )
                        .is_ok()
                {
                    *visibility = Visibility::Visible;
                } else {
                    *visibility = Visibility::Hidden;
                }
            },
            TradeOption::Harbor => {
                if state.eq(&TradeState::Offering)
                    && catan
                        .inner
                        .check_valid_local_trade(
                            &Trade {
                                from: catan.me,
                                to: None,
                                request: TradeRequest::new(
                                    trade
                                        .resource
                                        .offer
                                        .iter()
                                        .enumerate()
                                        .map(|(i, count)| {
                                            (
                                                TileKind::try_from(i as u8).unwrap(),
                                                *count as usize,
                                            )
                                        })
                                        .collect(),
                                    trade
                                        .resource
                                        .want
                                        .iter()
                                        .enumerate()
                                        .map(|(i, count)| {
                                            (
                                                TileKind::try_from(i as u8).unwrap(),
                                                *count as usize,
                                            )
                                        })
                                        .collect(),
                                    TradeTarget::Harbor,
                                ),
                            },
                            &catan.players[catan.me].inner,
                        )
                        .is_ok()
                {
                    *visibility = Visibility::Visible;
                } else {
                    *visibility = Visibility::Hidden;
                }
            },
            _ => {
                if state.eq(&TradeState::Offering) || state.eq(&TradeState::Accepting) {
                    *visibility = Visibility::Visible;
                } else {
                    *visibility = Visibility::Hidden;
                }
            },
        }
    }
}

fn update_resource_trade_cnt_marker_spawn(
    mut resources: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Handle<Image>,
        &TradeResourceTradeCntMarker,
    )>,
    trade_board: Res<TradeBoard>, img_store: Res<ImageStore>,
) {
    for (mut transform, mut sprite, mut img, res) in resources.iter_mut() {
        if res.is_offer {
            transform.translation = Vec3 {
                x: trade_board.draw.trade_size * 1.25,
                y: 0.,
                z: 0.1,
            };
            *img = img_store.number_img
                [trade_board.resource.offer[res.tile as usize] as usize]
                .clone();
        } else {
            transform.translation = Vec3 {
                x: -trade_board.draw.trade_size * 1.25,
                y: 0.,
                z: 0.1,
            };
            *img = img_store.number_img
                [trade_board.resource.want[res.tile as usize] as usize]
                .clone();
        }
        sprite.custom_size = Some(Vec2::new(
            trade_board.draw.trade_size * 0.5,
            trade_board.draw.trade_size * 0.5,
        ));
    }
}

fn update_resource_sub_marker_spawn(
    mut resources: Query<(&mut Transform, &mut Sprite, &TradeSubMarker)>,
    trade_board: Res<TradeBoard>,
) {
    for (mut transform, mut sprite, res) in resources.iter_mut() {
        if res.is_offer {
            transform.translation = Vec3 {
                x: trade_board.draw.trade_size,
                y: -trade_board.draw.trade_size / 4.,
                z: 0.,
            };
        } else {
            transform.translation = Vec3 {
                x: -trade_board.draw.trade_size,
                y: -trade_board.draw.trade_size / 4.,
                z: 0.,
            };
        }
        sprite.custom_size = Some(Vec2::new(
            trade_board.draw.trade_size * 0.5,
            trade_board.draw.trade_size * 0.5,
        ));
    }
}

fn update_resource_add_marker_spawn(
    mut resources: Query<(&mut Transform, &mut Sprite, &TradeAddMarker)>,
    trade_board: Res<TradeBoard>,
) {
    for (mut transform, mut sprite, res) in resources.iter_mut() {
        if res.is_offer {
            transform.translation = Vec3 {
                x: trade_board.draw.trade_size,
                y: trade_board.draw.trade_size / 4.,
                z: 0.,
            };
        } else {
            transform.translation = Vec3 {
                x: -trade_board.draw.trade_size,
                y: trade_board.draw.trade_size / 4.,
                z: 0.,
            };
        }
        sprite.custom_size = Some(Vec2::new(
            trade_board.draw.trade_size * 0.5,
            trade_board.draw.trade_size * 0.5,
        ));
    }
}

fn update_resource_cnt_marker_spawn(
    mut resources: Query<(&mut Sprite, &mut Handle<Image>, &TradeResourceCntMarker)>,
    trade_board: Res<TradeBoard>, catan: Res<Catan>, img_store: Res<ImageStore>,
) {
    for (mut sprite, mut img, res) in resources.iter_mut() {
        if res.is_offer {
            *img = img_store.number_img
                [catan.players[catan.me].inner.resources[res.tile as usize] as usize]
                .clone();
        } else {
            *img = img_store.number_img
                [catan.players[catan.me].inner.resources[res.tile as usize] as usize]
                .clone();
        }
        sprite.custom_size = Some(Vec2::new(
            trade_board.draw.trade_size * 0.5,
            trade_board.draw.trade_size * 0.5,
        ));
    }
}

fn update_resource_marker_spawn(
    mut resources: Query<(&mut Transform, &mut Sprite, &TradeResourceMarker)>,
    trade_board: Res<TradeBoard>,
) {
    for (mut transform, mut sprite, res) in resources.iter_mut() {
        if res.is_offer {
            transform.translation = trade_board.draw.offer[&res.tile];
        } else {
            transform.translation = trade_board.draw.want[&res.tile];
        }
        sprite.custom_size = Some(Vec2::new(
            trade_board.draw.trade_size,
            trade_board.draw.trade_size,
        ));
    }
}

fn update_trade_spawn(
    mut trade: Query<(&mut Transform, &mut Sprite, &TradeMarker)>,
    windows: Query<&Window>, catan: Res<Catan>, mut trade_board: ResMut<TradeBoard>,
) {
    for window in windows.iter() {
        let trade_board_size = window.width().min(window.height()) * 0.8;

        trade.single_mut().1.custom_size =
            Some(Vec2::new(trade_board_size, trade_board_size));
        let trade_size = trade_board_size
            / catan.players[catan.me]
                .inner
                .resources
                .iter()
                .enumerate()
                .filter(|(i, _)| TileKind::try_from(*i as u8).unwrap().is_resource())
                .count() as f32;
        let mut i = 0;
        for j in 0..catan.players[catan.me].inner.resources.len() {
            let kind = &TileKind::try_from(j as u8).unwrap();
            if !kind.is_resource() {
                continue;
            }
            trade_board.draw.offer.insert(
                *kind,
                Vec3 {
                    x: -trade_board_size / 2. + trade_size / 2 as f32,
                    y: trade_board_size / 2. - trade_size / 2. - trade_size * i as f32,
                    z: 0.1,
                },
            );
            trade_board.draw.want.insert(
                *kind,
                Vec3 {
                    x: trade_board_size / 2. - trade_size / 2 as f32,
                    y: trade_board_size / 2. - trade_size / 2. - trade_size * i as f32,
                    z: 0.1,
                },
            );
            i += 1;
        }
        trade_board.draw.trade_size = trade_size;
        trade_board.draw.button_size = trade_size / 4.0;
    }
}

fn spawn_trade_entry(
    command: &mut ChildBuilder, kind: TileKind, is_offer: bool,
    img_store: &Res<ImageStore>,
) {
    command
        .spawn((
            TradeResourceMarker {
                tile: kind,
                is_offer,
            },
            SpriteBundle {
                texture: img_store.resource_img.get(&kind).unwrap().clone(),
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                ..Default::default()
            },
        ))
        .with_children(|command| {
            command.spawn((
                TradeResourceCntMarker {
                    tile: kind,
                    is_offer,
                },
                SpriteBundle {
                    texture: img_store.number_img[0].clone(),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    ..Default::default()
                },
            ));
            command.spawn((
                TradeAddMarker { is_offer },
                SpriteBundle {
                    texture: img_store.add.clone(),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    ..Default::default()
                },
            ));
            command.spawn((
                TradeSubMarker { is_offer },
                SpriteBundle {
                    texture: img_store.sub.clone(),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    ..Default::default()
                },
            ));
            command.spawn((
                TradeResourceTradeCntMarker {
                    tile: kind,
                    is_offer,
                },
                SpriteBundle {
                    texture: img_store.number_img[0].clone(),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    ..Default::default()
                },
            ));
        });
}

fn despawn_trade(mut commands: Commands, trade: Query<(Entity, &TradeMarker)>) {
    for (entity, _) in trade.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_trade(mut command: Commands, catan: ResMut<Catan>, img_store: Res<ImageStore>) {
    command
        .spawn((
            TradeMarker,
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.0, 0.0, 0.0, 0.9),
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .with_children(|command| {
            for j in 0..catan.players[catan.me].inner.resources.len() {
                let kind = &TileKind::try_from(j as u8).unwrap();
                if kind.is_resource() {
                    spawn_trade_entry(command, *kind, true, &img_store);
                    spawn_trade_entry(command, *kind, false, &img_store);
                }
            }
            let mut id = 0;
            for i in 0..catan.players.len() {
                if i != catan.me {
                    command
                        .spawn((
                            TradeTargetPlayerMarker { id, player: i },
                            SpriteBundle {
                                texture: img_store.settlement_img[i].clone(),
                                ..Default::default()
                            },
                        ))
                        .with_children(|command| {
                            command.spawn((
                                TradeResponseMarker { player: i },
                                SpriteBundle {
                                    texture: img_store.number_img[0].clone(),
                                    ..Default::default()
                                },
                            ));
                        });
                    id += 1;
                }
            }

            command.spawn((
                TradeOption::Bank,
                SpriteBundle {
                    texture: img_store.bank_img.clone(),
                    ..Default::default()
                },
            ));
            command.spawn((
                TradeOption::Harbor,
                SpriteBundle {
                    texture: img_store.harbor_img.clone(),
                    ..Default::default()
                },
            ));
            command.spawn((
                TradeOption::Player,
                SpriteBundle {
                    texture: img_store.yes.clone(),
                    ..Default::default()
                },
            ));
            command.spawn((
                TradeOption::Cancel,
                SpriteBundle {
                    texture: img_store.no.clone(),
                    ..Default::default()
                },
            ));
        });
}

#[derive(Default)]
struct DropBoardDraw {
    drop: HashMap<TileKind, (Vec3, Vec3)>,
    yes: Vec3,
    button_size: f32,
}

#[derive(Default)]
struct DropBoardResource {
    drop: [u8; TileKind::Max as usize],
}

#[derive(Resource, Default)]
struct DropBoard {
    draw: DropBoardDraw,
    resource: DropBoardResource,
}

impl DropBoard {
    fn clear(&mut self) {
        self.resource = DropBoardResource::default();
        self.draw = DropBoardDraw::default();
    }
}

fn check_drop_click(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut drop_board: ResMut<DropBoard>, mut catan: ResMut<Catan>,
    mut next_state: ResMut<NextState<CatanState>>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);

            for (kind, (add, sub)) in drop_board.draw.drop.iter() {
                if x > add.x - drop_board.draw.button_size
                    && x < add.x + drop_board.draw.button_size
                    && y > add.y - drop_board.draw.button_size
                    && y < add.y + drop_board.draw.button_size
                {
                    let k = kind.clone();

                    if catan.players[catan.me].inner.resources[k as usize]
                        > drop_board.resource.drop[k as usize] as usize
                    {
                        drop_board.resource.drop[k as usize] += 1;
                        drop_board.resource.drop[k as usize] =
                            drop_board.resource.drop[k as usize].min(20);
                    }
                    return;
                }
                if x > sub.x - drop_board.draw.button_size
                    && x < sub.x + drop_board.draw.button_size
                    && y > sub.y - drop_board.draw.button_size
                    && y < sub.y + drop_board.draw.button_size
                {
                    let k = kind.clone();
                    drop_board.resource.drop[k as usize] =
                        drop_board.resource.drop[k as usize].saturating_sub(1);
                    return;
                }

                if x > drop_board.draw.yes.x - drop_board.draw.yes.z
                    && x < drop_board.draw.yes.x + drop_board.draw.yes.z
                    && y > drop_board.draw.yes.y - drop_board.draw.yes.z
                    && y < drop_board.draw.yes.y + drop_board.draw.yes.z
                    && drop_board.resource.drop.iter().sum::<u8>() as usize
                        == catan.drop_cnt
                {
                    action_writer.send(
                        GameAct::DropResource(
                            drop_board
                                .resource
                                .drop
                                .iter()
                                .enumerate()
                                .map(|(i, count)| {
                                    (
                                        TileKind::try_from(i as u8).unwrap(),
                                        *count as usize,
                                    )
                                })
                                .collect(),
                        )
                        .into(),
                    );
                    if catan.current_turn == catan.me {
                        next_state.set(CatanState::Menu);
                    } else {
                        next_state.set(CatanState::Wait);
                    }
                    drop_board.clear();
                    catan.drop_cnt = 0;
                    return;
                }
            }
        }
    }
}

fn draw_drop_resource(
    mut painter: ShapePainter, catan: ResMut<Catan>, windows: Query<&Window>,
    img_store: Res<ImageStore>, mut drop_board: ResMut<DropBoard>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let trade_board_size = window.width().min(window.height()) * 0.8;
        painter.translate(Vec3 {
            x: 0.0,
            y: 0.0,
            z: TRADEBPARD_LAYER,
        });
        painter
            .rect(Vec2 {
                x: trade_board_size,
                y: trade_board_size,
            })
            .with_children(|spawn_children| {
                let config = spawn_children.config().clone();
                let drop_size = trade_board_size
                    / img_store
                        .resource_img
                        .iter()
                        .filter(|res| res.0.is_resource())
                        .count() as f32;
                let mut i = 0;
                for j in 0..catan.players[catan.me].inner.resources.len() {
                    let kind = &TileKind::try_from(j as u8).unwrap();
                    if !kind.is_resource() {
                        continue;
                    }
                    let res = img_store.resource_img.get(kind).unwrap();
                    let size = Vec2 {
                        x: drop_size,
                        y: drop_size,
                    };

                    //offer
                    let translate = Vec3 {
                        x: -trade_board_size / 2. + drop_size / 2 as f32,
                        y: trade_board_size / 2. - drop_size / 2. - drop_size * i as f32,
                        z: 0.1,
                    };
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(translate);
                    spawn_children.image(res.clone(), size);
                    spawn_children.translate(Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.1,
                    });
                    spawn_children.image(
                        img_store.number_img[(catan.players[catan.me].inner.resources
                            [*kind as usize])
                            .min(20)]
                        .clone(),
                        Vec2::new(size.x * 0.5, size.y * 0.5),
                    );
                    //add
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(
                        Vec3 {
                            x: drop_size,
                            y: drop_size / 4.,
                            z: 0.,
                        } + translate,
                    );
                    spawn_children.image(
                        img_store.add.clone(),
                        Vec2::new(size.x * 0.5, size.y * 0.5),
                    );
                    let add_translate = spawn_children.transform.translation;

                    //sub
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(
                        Vec3 {
                            x: drop_size,
                            y: -drop_size / 4.,
                            z: 0.,
                        } + translate,
                    );
                    spawn_children.image(
                        img_store.sub.clone(),
                        Vec2::new(size.x * 0.5, size.y * 0.5),
                    );
                    let sub_translate = spawn_children.transform.translation;
                    drop_board
                        .draw
                        .drop
                        .insert(*kind, (add_translate, sub_translate));

                    //count
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(
                        Vec3 {
                            x: drop_size * 1.25,
                            y: 0.,
                            z: 0.1,
                        } + translate,
                    );
                    spawn_children.image(
                        img_store.number_img
                            [(drop_board.resource.drop[*kind as usize] as usize).min(20)]
                        .clone(),
                        Vec2::new(size.x * 0.5, size.y * 0.5),
                    );
                    drop_board.draw.button_size = drop_size * 0.25;
                    i += 1;
                }
                if drop_board.resource.drop.iter().sum::<u8>() as usize == catan.drop_cnt
                {
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(Vec3 {
                        x: 0.,
                        y: -drop_size, // * 2.,
                        z: 0.1,
                    });
                    spawn_children
                        .image(img_store.yes.clone(), Vec2::new(drop_size, drop_size));
                    drop_board.draw.yes = spawn_children.transform.translation;
                    drop_board.draw.yes.z = drop_size / 2.;
                }
            });
    }
}

#[derive(Default, Resource)]
struct ImageStore {
    operation_img: HashMap<Operation, Handle<Image>>,
    resource_img: HashMap<TileKind, Handle<Image>>,
    number_img: [Handle<Image>; 21],
    road_img: [Handle<Image>; 4],
    settlement_img: [Handle<Image>; 4],
    city_img: [Handle<Image>; 4],
    bank_img: Handle<Image>,
    harbor_img: Handle<Image>,
    robber_img: Handle<Image>,
    card_img: [Handle<Image>; DevCard::Max as usize],
    dice_img: [Handle<Image>; 6],
    yes: Handle<Image>,
    no: Handle<Image>,
    add: Handle<Image>,
    sub: Handle<Image>,
}

fn load_img(
    asset_server: Res<AssetServer>, mut image_store: ResMut<ImageStore>,
    platform: Res<Platform>,
) {
    image_store.operation_img.insert(
        Operation::Trade,
        asset_server.load(platform.load_asset("catan/trade.png")),
    );
    image_store.operation_img.insert(
        Operation::BuyCard,
        asset_server.load(platform.load_asset("catan/buy_card.png")),
    );
    image_store.operation_img.insert(
        Operation::UseCard,
        asset_server.load(platform.load_asset("catan/use_card.png")),
    );
    image_store.operation_img.insert(
        Operation::EndTurn,
        asset_server.load(platform.load_asset("catan/end_turn.png")),
    );

    image_store.resource_img.insert(
        TileKind::Brick,
        asset_server.load(platform.load_asset("catan/brick.png")),
    );
    image_store.resource_img.insert(
        TileKind::Grain,
        asset_server.load(platform.load_asset("catan/grain.png")),
    );
    image_store.resource_img.insert(
        TileKind::Stone,
        asset_server.load(platform.load_asset("catan/stone.png")),
    );
    image_store.resource_img.insert(
        TileKind::Wood,
        asset_server.load(platform.load_asset("catan/wood.png")),
    );
    image_store.resource_img.insert(
        TileKind::Wool,
        asset_server.load(platform.load_asset("catan/wool.png")),
    );
    image_store.resource_img.insert(
        TileKind::Dessert,
        asset_server.load(platform.load_asset("catan/dessert.png")),
    );

    for i in 0..image_store.number_img.len() {
        image_store.number_img[i] =
            asset_server.load(platform.load_asset(format!("catan/{}.png", i).as_str()));
    }

    for i in 0..image_store.road_img.len() {
        image_store.road_img[i] = asset_server
            .load(platform.load_asset(format!("catan/road_{}.png", i).as_str()));
    }

    for i in 0..image_store.settlement_img.len() {
        image_store.settlement_img[i] = asset_server
            .load(platform.load_asset(format!("catan/settlement_{}.png", i).as_str()));
    }

    for i in 0..image_store.dice_img.len() {
        image_store.dice_img[i] = asset_server
            .load(platform.load_asset(format!("catan/dice{}.png", i + 1).as_str()));
    }

    for i in 0..image_store.city_img.len() {
        image_store.city_img[i] = asset_server
            .load(platform.load_asset(format!("catan/city_{}.png", i).as_str()));
    }

    image_store.card_img[DevCard::Knight as usize] =
        asset_server.load(platform.load_asset("catan/knight.png"));
    image_store.card_img[DevCard::VictoryPoint as usize] =
        asset_server.load(platform.load_asset("catan/victory_point.png"));
    image_store.card_img[DevCard::RoadBuilding as usize] =
        asset_server.load(platform.load_asset("catan/road_building.png"));
    image_store.card_img[DevCard::YearOfPlenty as usize] =
        asset_server.load(platform.load_asset("catan/year_of_plenty.png"));
    image_store.card_img[DevCard::Monopoly as usize] =
        asset_server.load(platform.load_asset("catan/monopoly.png"));

    image_store.robber_img = asset_server.load(platform.load_asset("catan/robber.png"));
    image_store.yes = asset_server.load(platform.load_asset("catan/yes.png"));
    image_store.no = asset_server.load(platform.load_asset("catan/no.png"));
    image_store.add = asset_server.load(platform.load_asset("catan/add.png"));
    image_store.sub = asset_server.load(platform.load_asset("catan/sub.png"));
    image_store.bank_img = asset_server.load(platform.load_asset("catan/bank.png"));
    image_store.harbor_img = asset_server.load(platform.load_asset("catan/harbor.png"));
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum CatanLoadState {
    #[default]
    Connecting,
    Loading,
    Initialzing,
    Loaded,
}

fn image_ready(asset_server: &Res<AssetServer>, img: Handle<Image>) -> bool {
    match asset_server.get_load_state(img) {
        Some(LoadState::Loaded) => true,
        _ => false,
    }
}

fn loading(
    mut commands: Commands, asset_server: Res<AssetServer>,
    mut img_store: ResMut<ImageStore>,
    mut event_reader: ConsumableEventReader<GameEvent>,
    mut next_state: ResMut<NextState<CatanLoadState>>,
) {
    for (_, res) in img_store.resource_img.iter() {
        if !image_ready(&asset_server, res.clone()) {
            return;
        }
    }

    for (_, oper) in img_store.operation_img.iter() {
        if !image_ready(&asset_server, oper.clone()) {
            return;
        }
    }

    for image in img_store.number_img.iter() {
        if !image_ready(&asset_server, image.clone()) {
            return;
        }
    }

    for image in img_store.city_img.iter() {
        if !image_ready(&asset_server, image.clone()) {
            return;
        }
    }

    for image in img_store.settlement_img.iter() {
        if !image_ready(&asset_server, image.clone()) {
            return;
        }
    }

    for image in img_store.road_img.iter() {
        if !image_ready(&asset_server, image.clone()) {
            return;
        }
    }

    for image in img_store.card_img.iter() {
        if !image_ready(&asset_server, image.clone()) {
            return;
        }
    }

    for image in img_store.dice_img.iter() {
        if !image_ready(&asset_server, image.clone()) {
            return;
        }
    }

    if !image_ready(&asset_server, img_store.robber_img.clone()) {
        return;
    }

    if !image_ready(&asset_server, img_store.yes.clone()) {
        return;
    }

    if !image_ready(&asset_server, img_store.no.clone()) {
        return;
    }

    if !image_ready(&asset_server, img_store.add.clone()) {
        return;
    }

    if !image_ready(&asset_server, img_store.sub.clone()) {
        return;
    }

    if !image_ready(&asset_server, img_store.bank_img.clone()) {
        return;
    }

    if !image_ready(&asset_server, img_store.harbor_img.clone()) {
        return;
    }

    for event in event_reader.read() {
        info!("event: {:?}", event.deref());
        match event.consume().into() {
            GameMsg::GameStart(start) => {
                let mut catan = Catan::new(start.clone());

                let settlment_img = img_store.settlement_img[catan.me as usize].clone();
                let city_img = img_store.city_img[catan.me as usize].clone();
                let road_img = img_store.road_img[catan.me as usize].clone();

                img_store
                    .operation_img
                    .insert(Operation::BuildSettlement, settlment_img);
                img_store
                    .operation_img
                    .insert(Operation::BuildCity, city_img);
                img_store
                    .operation_img
                    .insert(Operation::BuildRoad, road_img);

                spawn_catan_tiles(&mut commands, &mut catan, &img_store);
                spawn_catan_resources(&mut commands, &mut catan, &img_store);
                spawn_operations(&mut commands, &mut catan, &img_store);
                spawn_dices(&mut commands, &mut catan, &img_store);
                commands.insert_resource(catan);
                next_state.set(CatanLoadState::Initialzing);
                break;
            },
            _ => {
                unreachable!("unexpected event")
            },
        }
    }
}

#[derive(Event, Debug)]
struct GameEvent(GameMsg);

impl Deref for GameEvent {
    type Target = GameMsg;
    fn deref(&self) -> &GameMsg {
        &self.0
    }
}

impl Into<GameMsg> for GameEvent {
    fn into(self) -> GameMsg {
        self.0
    }
}

impl From<GameMsg> for GameEvent {
    fn from(msg: GameMsg) -> Self {
        GameEvent(msg)
    }
}

#[derive(Event, Debug)]
struct GameAction(GameAct);

impl Deref for GameAction {
    type Target = GameAct;
    fn deref(&self) -> &GameAct {
        &self.0
    }
}

impl Into<GameAct> for GameAction {
    fn into(self) -> GameAct {
        self.0
    }
}

impl From<GameAct> for GameAction {
    fn from(act: GameAct) -> Self {
        GameAction(act)
    }
}

fn process_event(
    mut commands: Commands, img_store: ResMut<ImageStore>, mut catan: ResMut<Catan>,
    mut trade: ResMut<TradeBoard>, trade_state: Res<State<TradeState>>,
    state: Res<State<CatanState>>, mut next_state: ResMut<NextState<CatanState>>,
    mut next_trade_state: ResMut<NextState<TradeState>>,
    mut event_reader: ConsumableEventReader<GameEvent>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if state.eq(&CatanState::Menu)
        || state.eq(&CatanState::Wait)
        || state.eq(&CatanState::Trade)
    {
        for event in event_reader.read() {
            info!("event: {:?}", event.deref());
            match event.consume().into() {
                GameMsg::PlayerInit(player) => {
                    if player == catan.me {
                        next_state.set(CatanState::InitSettlement);
                        break;
                    }
                },
                GameMsg::PlayerTurn(player) => {
                    catan.current_turn = player;
                    if player == catan.me {
                        next_state.set(CatanState::Menu);
                        catan.used_card = false;
                        break;
                    } else {
                        next_state.set(CatanState::Wait);
                        break;
                    }
                },
                GameMsg::PlayerRollDice((dice1, dice2)) => {
                    catan.dices[0].nubmer = dice1;
                    catan.dices[1].nubmer = dice2;
                },
                GameMsg::PlayerBuildRoad(build) => {
                    catan.inner.add_road(build.player, build.road);
                    catan.players[build.player].inner.add_road(build.road);
                    if build.player != catan.me {
                        let _ = catan.players[build.player].inner.resources
                            [TileKind::Brick as usize]
                            .saturating_sub(1);
                        let _ = catan.players[build.player].inner.resources
                            [TileKind::Wood as usize]
                            .saturating_sub(1);
                    }
                    spawn_catan_road(
                        &mut commands,
                        build.road,
                        img_store.road_img[build.player].clone(),
                    );
                },
                GameMsg::PlayerBuildSettlement(build) => {
                    catan.inner.add_settlement(build.player, build.point);
                    if build.player == catan.me {
                        catan.players[build.player].inner.settlement_left -= 1;
                    } else {
                        let _ = catan.players[build.player].inner.resources
                            [TileKind::Brick as usize]
                            .saturating_sub(1);
                        let _ = catan.players[build.player].inner.resources
                            [TileKind::Grain as usize]
                            .saturating_sub(1);
                        let _ = catan.players[build.player].inner.resources
                            [TileKind::Wood as usize]
                            .saturating_sub(1);
                        let _ = catan.players[build.player].inner.resources
                            [TileKind::Wool as usize]
                            .saturating_sub(1);
                    }
                },
                GameMsg::PlayerBuildCity(build) => {
                    assert_eq!(catan.current_turn, build.player);
                    catan.inner.add_city(build.player, build.point);
                    if build.player == catan.me {
                        catan.players[build.player].inner.city_left -= 1;
                        catan.players[build.player].inner.settlement_left += 1;
                    } else {
                        let _ = catan.players[build.player].inner.resources
                            [TileKind::Grain as usize]
                            .saturating_sub(2);
                        let _ = catan.players[build.player].inner.resources
                            [TileKind::Stone as usize]
                            .saturating_sub(3);
                    }
                },
                GameMsg::PlayerBuyDevelopmentCard(buy) => {
                    assert_eq!(catan.current_turn, buy.player);
                    if buy.player == catan.me {
                        // ignore the broadcasted message
                        if buy.card.is_some() {
                            catan.players[buy.player].inner.resources
                                [TileKind::Stone as usize] -= 1;
                            catan.players[buy.player].inner.resources
                                [TileKind::Grain as usize] -= 1;
                            catan.players[buy.player].inner.resources
                                [TileKind::Wool as usize] -= 1;
                            catan.players[buy.player].inner.add_card(buy.card);
                        }
                    } else {
                        catan.players[buy.player].inner.add_card(None);
                        let _ = catan.players[buy.player].inner.resources
                            [TileKind::Grain as usize]
                            .saturating_sub(1);
                        let _ = catan.players[buy.player].inner.resources
                            [TileKind::Wool as usize]
                            .saturating_sub(1);
                        let _ = catan.players[buy.player].inner.resources
                            [TileKind::Stone as usize]
                            .saturating_sub(1);
                    }
                },
                GameMsg::PlayerUseDevelopmentCard(use_card) => {
                    assert_eq!(catan.current_turn, use_card.player);
                    if use_card.player == catan.me {
                        catan.players[use_card.player]
                            .inner
                            .remove_card(Some(use_card.card));
                    } else {
                        catan.players[use_card.player].inner.remove_card(None);
                    }

                    match use_card.usage {
                        DevelopmentCard::Monopoly(kind) => {
                            for i in 0..catan.players.len() {
                                if i != use_card.player {
                                    let count =
                                        catan.players[i].inner.resources[kind as usize];
                                    catan.players[i].inner.resources[kind as usize] = 0;
                                    catan.players[use_card.player].inner.resources
                                        [kind as usize] += count;
                                }
                            }
                        },
                        DevelopmentCard::YearOfPlenty(kind1, kind2) => {
                            catan.players[use_card.player].inner.resources
                                [kind1 as usize] += 1;
                            catan.players[use_card.player].inner.resources
                                [kind2 as usize] += 1;
                        },
                        DevelopmentCard::VictoryPoint => {
                            catan.players[use_card.player].inner.score += 1;
                        },
                        _ => {},
                    }

                    if let DevelopmentCard::Monopoly(_) = use_card.usage {}
                },
                GameMsg::PlayerSelectRobber(select_robber) => {
                    assert_eq!(catan.current_turn, select_robber.player);
                    catan.inner.set_robber(select_robber.coord);
                },
                GameMsg::PlayerTradeRequest((player, trade_req)) => {
                    if player != catan.me {
                        for offer in trade_req.from() {
                            trade.resource.offer[offer.0 as usize] = offer.1 as u8;
                        }
                        for want in trade_req.to() {
                            if catan.players[catan.me].inner.resources[want.0 as usize]
                                < want.1 as usize
                            {
                                action_writer.send(
                                    GameAct::TradeResponse(TradeResponse::Reject).into(),
                                );
                                next_trade_state.set(TradeState::WaitingConfirm);
                                next_state.set(CatanState::Trade);
                                trade.clear();
                                return;
                            }
                            trade.resource.want[want.0 as usize] = want.1 as u8;
                        }
                        next_trade_state.set(TradeState::Accepting);
                        next_state.set(CatanState::Trade);
                        break;
                    }
                },
                GameMsg::PlayerTradeResponse((player, resp)) => {
                    info!("{:?}", trade_state);
                    if trade_state.eq(&TradeState::WaitingResponse) && player != catan.me
                    {
                        trade.resource.response.insert(player, resp);
                        if trade.resource.response.len() == catan.players.len() - 1 {
                            if trade
                                .resource
                                .response
                                .iter()
                                .all(|(_, resp)| resp.eq(&TradeResponse::Accept))
                            {
                                next_trade_state.set(TradeState::Confirming);
                            } else {
                                action_writer.send(GameAct::TradeConfirm(None).into());
                            }
                        }
                    }
                },
                GameMsg::PlayerTrade(trade) => match trade {
                    Some(trade) => {
                        for (kind, count) in trade.request.from() {
                            catan.players[trade.from].inner.resources[*kind as usize] -=
                                count;
                            if let Some(to) = trade.to {
                                catan.players[to].inner.resources[*kind as usize] +=
                                    count;
                            }
                        }
                        for (kind, count) in trade.request.to() {
                            catan.players[trade.from].inner.resources[*kind as usize] +=
                                count;
                            if let Some(to) = trade.to {
                                catan.players[to].inner.resources[*kind as usize] -=
                                    count;
                            }
                        }
                        if catan.current_turn == catan.me {
                            next_state.set(CatanState::Menu);
                        } else {
                            next_state.set(CatanState::Wait);
                        }
                        break;
                    },
                    None => {
                        if catan.current_turn == catan.me {
                            next_state.set(CatanState::Menu);
                        } else {
                            next_state.set(CatanState::Wait);
                        }
                        break;
                    },
                },
                GameMsg::PlayerEndTurn(_) => {},
                GameMsg::PlayerOfferResources(offer) => {
                    catan.players[offer.player].inner.resources[offer.kind as usize] =
                        (catan.players[offer.player].inner.resources[offer.kind as usize]
                            as isize
                            + offer.count)
                            .min(20)
                            .max(0) as usize;
                },
                GameMsg::PlayerStartSelectRobber() => {
                    if catan.current_turn == catan.me {
                        next_state.set(CatanState::SelectRobber);
                    }
                },
                GameMsg::PlayerDropResources((player, count)) => {
                    if player == catan.me {
                        catan.drop_cnt = count;
                        next_state.set(CatanState::DropResource);
                        break;
                    }
                },
                _ => {
                    unreachable!("unexpected event")
                },
            }
        }
    }
}

fn process_action(
    mut action_reader: ConsumableEventReader<GameAction>, client: ResMut<NetworkClt>,
) {
    for action in action_reader.read() {
        info!("action: {:?}", action.deref());
        client.send(ClientMsg::Catan(action.consume().into()));
    }
}

fn client_process_event(
    mut client: ResMut<NetworkClt>, mut next_state: ResMut<NextState<CatanLoadState>>,
    mut event_writer: ConsumableEventWriter<GameEvent>,
) {
    while let Some(client_event) = client.try_next() {
        match client_event {
            NetworkClientEvent::Report(connection_report) => match connection_report {
                bevy_simplenet::ClientReport::Connected => {
                    next_state.set(CatanLoadState::Loading);
                },
                bevy_simplenet::ClientReport::Disconnected
                | bevy_simplenet::ClientReport::ClosedByServer(_)
                | bevy_simplenet::ClientReport::ClosedBySelf => {
                    next_state.set(CatanLoadState::Connecting);
                },
                bevy_simplenet::ClientReport::IsDead(aborted_reqs) => {
                    info!("client dead: {:?}", aborted_reqs);
                    panic!("client dead");
                },
            },
            NetworkClientEvent::Msg(message) => match message {
                boardgame_common::network::ServerMsg::Catan(msg) => {
                    event_writer.send(msg.into());
                },
                _ => {
                    info!("unexpected message: {:?}", message);
                },
            },
            _ => continue,
        }
    }
}

fn update_operation_visibility(
    mut operations: Query<&mut Visibility, With<Operation>>,
    state: Res<State<CatanState>>,
) {
    match state.get() {
        CatanState::Menu
        | CatanState::BuidSettlement
        | CatanState::BuildCity
        | CatanState::BuildRoad
        | CatanState::Trade
        | CatanState::UseDevelopmentCard => {
            for mut vis in operations.iter_mut() {
                *vis = Visibility::Visible;
            }
        },
        _ => {
            for mut vis in operations.iter_mut() {
                *vis = Visibility::Hidden;
            }
        },
    }
}

pub fn catan_run() {
    App::new()
        .add_persistent_consumable_event::<GameEvent>()
        .add_persistent_consumable_event::<GameAction>()
        .init_state::<CatanState>()
        .init_state::<CatanLoadState>()
        .init_state::<TradeState>()
        .init_state::<UseCardState>()
        .init_resource::<Events<GameEvent>>()
        .insert_resource(AssetMetaCheck::Never)
        .insert_resource(
            #[cfg(target_family = "wasm")]
            {
                Platform {
                    asset_srv_addr: "http://boardgame.studio:9000/assets".to_string(),
                }
            },
            #[cfg(not(target_family = "wasm"))]
            {
                Platform {}
            },
        )
        .insert_resource(NetworkClt::from(new_client()))
        .insert_resource(ImageStore::default())
        .insert_resource(TradeBoard::default())
        .insert_resource(DropBoard::default())
        .add_plugins((
            #[cfg(target_family = "wasm")]
            WebAssetPlugin::default(),
            WindowResizePlugin,
            CameraPlugin,
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1000., 1000.)
                        .with_scale_factor_override(1.0),
                    title: "Catan".to_string(),
                    ..default()
                }),
                ..default()
            }),
        ))
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(Shape2dPlugin::default())
        .add_systems(Startup, limit_frame)
        .add_systems(Startup, load_img)
        .add_systems(Update, client_process_event)
        .add_systems(Update, loading.run_if(in_state(CatanLoadState::Loading)))
        .add_systems(
            Update,
            intialize_game.run_if(in_state(CatanLoadState::Initialzing)),
        )
        .add_systems(OnEnter(CatanState::InitRoad), spawn_initable_roads)
        .add_systems(OnExit(CatanState::InitRoad), despawn_buildable_roads)
        .add_systems(OnEnter(UseCardState::RoadBuilding), spawn_buildable_roads)
        .add_systems(OnExit(UseCardState::RoadBuilding), despawn_buildable_roads)
        .add_systems(OnEnter(CatanState::BuildRoad), spawn_buildable_roads)
        .add_systems(OnExit(CatanState::BuildRoad), despawn_buildable_roads)
        .add_systems(OnEnter(CatanState::Trade), spawn_trade)
        .add_systems(OnExit(CatanState::Trade), despawn_trade)
        .add_systems(
            OnEnter(CatanState::UseDevelopmentCard),
            spawn_use_development_card,
        )
        .add_systems(
            OnExit(UseCardState::SelectCard),
            despawn_use_development_card,
        )
        .add_systems(
            OnExit(CatanState::UseDevelopmentCard),
            despawn_use_development_card,
        )
        .add_systems(
            Update,
            (
                process_action,
                process_event,
                update_operation_visibility.run_if(in_state(CatanLoadState::Loaded)),
                (
                    (
                        update_catan_resources_transform,
                        update_catan_resources_spawn,
                        update_catan_resources_cnt_spawn,
                    )
                        .chain(),
                    (update_dices_transform, update_dices_spawn).chain(),
                    (update_operation_transform, update_operation_spawn).chain(),
                    (
                        update_catan_tiles_transform,
                        update_catan_tiles_spawn,
                        update_catan_points_spawn,
                        update_catan_harbor_spawn,
                        update_catan_robber_spawn,
                        update_catan_road_spawn,
                        update_buildable_roads_spawn,
                    )
                        .chain(),
                ),
                check_init_settlement.run_if(in_state(CatanState::InitSettlement)),
                check_init_road.run_if(in_state(CatanState::InitRoad)),
                check_build_road.run_if(in_state(CatanState::BuildRoad)),
                check_build_settlement.run_if(in_state(CatanState::BuidSettlement)),
                check_build_city.run_if(in_state(CatanState::BuildCity)),
                check_select_robber.run_if(in_state(CatanState::SelectRobber)),
                check_steal_target.run_if(in_state(CatanState::Stealing)),
                (draw_drop_resource, check_drop_click)
                    .run_if(in_state(CatanState::DropResource)),
                draw_steal_target.run_if(in_state(CatanState::Stealing)),
                check_menu_click
                    .run_if(not(in_state(CatanState::Wait)))
                    .run_if(not(in_state(CatanState::InitRoad)))
                    .run_if(not(in_state(CatanState::InitSettlement)))
                    .run_if(not(in_state(CatanState::SelectRobber)))
                    .run_if(not(in_state(CatanState::Stealing)))
                    .run_if(not(in_state(CatanState::DropResource))),
                (
                    (
                        update_trade_spawn,
                        update_resource_marker_spawn,
                        update_resource_cnt_marker_spawn,
                        update_resource_add_marker_spawn,
                        update_resource_sub_marker_spawn,
                        update_resource_trade_cnt_marker_spawn,
                        update_trade_option_spawn,
                        update_trade_target_player_spawn,
                        update_trade_response_spawn,
                    )
                        .chain(),
                    check_trade_offering_click.run_if(in_state(TradeState::Offering)),
                    check_trade_accepting_click.run_if(in_state(TradeState::Accepting)),
                    check_trade_confirm_click.run_if(in_state(TradeState::Confirming)),
                )
                    .run_if(in_state(CatanState::Trade)),
                (
                    (
                        update_use_development_card,
                        update_development_card,
                        update_development_card_cnt,
                        check_development_card_click,
                    )
                        .run_if(in_state(UseCardState::SelectCard)),
                    (draw_monopoly, check_monopoly_click)
                        .run_if(in_state(UseCardState::Monopoly)),
                    (draw_year_of_plenty, check_year_of_plenty_click)
                        .run_if(in_state(UseCardState::YearOfPlenty)),
                    check_knight_select_robber.run_if(in_state(UseCardState::Knight)),
                    check_knight_steal_target
                        .run_if(in_state(UseCardState::KnightStealing)),
                    check_road_building_build_road
                        .run_if(in_state(UseCardState::RoadBuilding)),
                )
                    .run_if(in_state(CatanState::UseDevelopmentCard)),
            )
                .run_if(in_state(CatanLoadState::Loaded)),
        )
        .run();
}
