use bevy::asset::{AssetMetaCheck, LoadState};
use bevy::prelude::*;
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

#[derive(Resource)]
struct Catan {
    inner: CatanCommon,
    tiles: Vec<Vec<Vec3>>,
    points: Vec<Vec<Vec3>>,
    players: Vec<CatanPlayer>,
    radius: Option<f32>,
    me: usize,
    current_turn: usize,
    stealing_candidate: HashSet<usize>,
    selected_yop: Option<TileKind>,
    road_building: Option<Line>,
    used_card: bool,
    drop_cnt: usize,
    dice: (u8, u8),
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
            inner,
            tiles: draw_tiles,
            points: draw_points,
            radius: None,
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
            dice: (1, 1),
        }
    }

    fn update_radius(&mut self, radius: f32, translate: Vec3) {
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
                            self.tiles[i][j] = Vec3::new(
                                x_offset + 3. * x_offset * (i / 2) as f32,
                                y_offset + 2. * y_offset * j as f32,
                                1.,
                            ) + translate;
                        } else {
                            self.tiles[i][j] = Vec3::new(
                                2.5 * x_offset as f32 + 3. * x_offset * (i / 2) as f32,
                                2. * y_offset + 2. * y_offset * j as f32,
                                1.,
                            ) + translate;
                        }
                    }
                }

                for i in 0..self.points.len() {
                    for j in 0..self.points[i].len() {
                        if i % 2 == 0 && j % 2 == 0 {
                            self.points[i][j] = Vec3::new(
                                x_offset / 2. + 3. * x_offset * (i / 2) as f32,
                                y_offset * j as f32,
                                1.2,
                            ) + translate;
                        } else if i % 2 == 0 && j % 2 == 1 {
                            self.points[i][j] = Vec3::new(
                                3. * x_offset * (i / 2) as f32,
                                y_offset * j as f32,
                                1.2,
                            ) + translate;
                        } else if i % 2 == 1 && j % 2 == 0 {
                            self.points[i][j] = Vec3::new(
                                x_offset / 2. + x_offset + 3. * x_offset * (i / 2) as f32,
                                y_offset * j as f32,
                                1.2,
                            ) + translate;
                        } else if i % 2 == 1 && j % 2 == 1 {
                            self.points[i][j] = Vec3::new(
                                2. * x_offset + 3. * x_offset * (i / 2) as f32,
                                y_offset * j as f32,
                                1.2,
                            ) + translate;
                        }
                    }
                }
            },
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
                    if x > catan.points[i][j].x - catan.radius.unwrap() * 0.2
                        && x < catan.points[i][j].x + catan.radius.unwrap() * 0.2
                        && y > catan.points[i][j].y - catan.radius.unwrap() * 0.2
                        && y < catan.points[i][j].y + catan.radius.unwrap() * 0.2
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
                    if x > catan.points[i][j].x - catan.radius.unwrap() * 0.2
                        && x < catan.points[i][j].x + catan.radius.unwrap() * 0.2
                        && y > catan.points[i][j].y - catan.radius.unwrap() * 0.2
                        && y < catan.points[i][j].y + catan.radius.unwrap() * 0.2
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
                    if x > catan.points[i][j].x - catan.radius.unwrap() * 0.2
                        && x < catan.points[i][j].x + catan.radius.unwrap() * 0.2
                        && y > catan.points[i][j].y - catan.radius.unwrap() * 0.2
                        && y < catan.points[i][j].y + catan.radius.unwrap() * 0.2
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
                    let point = Coordinate { x: i, y: j };
                    if let Some(player) = catan.inner.point(point).owner() {
                        if player == catan.me {
                            for candidate in catan.inner.point_get_points(point) {
                                if let Some(candidate) = candidate {
                                    let road = Line::new(point, candidate);
                                    if catan.inner.roads().get(&road).is_none() {
                                        if x > (catan.points[i][j]
                                            + catan.points[candidate.x][candidate.y])
                                            .x
                                            / 2.
                                            - catan.radius.unwrap() * 0.2
                                            && x < (catan.points[i][j]
                                                + catan.points[candidate.x][candidate.y])
                                                .x
                                                / 2.
                                                + catan.radius.unwrap() * 0.2
                                            && y > (catan.points[i][j]
                                                + catan.points[candidate.x][candidate.y])
                                                .y
                                                / 2.
                                                - catan.radius.unwrap() * 0.2
                                            && y < (catan.points[i][j]
                                                + catan.points[candidate.x][candidate.y])
                                                .y
                                                / 2.
                                                + catan.radius.unwrap() * 0.2
                                        {
                                            action_writer.send(
                                                GameAct::BuildRoad(point, candidate)
                                                    .into(),
                                            );
                                            let me = catan.me;
                                            catan.inner.add_road(me, road);
                                            catan.players[me].inner.add_road(road);
                                            next_state.set(CatanState::Wait);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn draw_initable_roads(painter: &mut ShapePainter, catan: &ResMut<Catan>) {
    let config = painter.config().clone();

    for i in 0..catan.points.len() {
        for j in 0..catan.points[i].len() {
            let point = Coordinate { x: i, y: j };
            if let Some(player) = catan.inner.point(point).owner() {
                if player == catan.me {
                    for candidate in catan.inner.point_get_points(point) {
                        if let Some(candidate) = candidate {
                            let road = Line::new(point, candidate);
                            if catan.inner.roads().get(&road).is_none() {
                                painter.reset();
                                painter.translate(
                                    (catan.points[i][j]
                                        + catan.points[candidate.x][candidate.y])
                                        / 2.,
                                );
                                painter.color = Color::rgb(1.0, 1.0, 1.0);
                                painter.circle(catan.radius.unwrap() * 0.2);
                            }
                        }
                    }
                }
            }
        }
    }
    painter.set_config(config);
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
                    - catan.radius.unwrap() * 0.2
                    && x < (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .x
                        / 2.
                        + catan.radius.unwrap() * 0.2
                    && y > (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .y
                        / 2.
                        - catan.radius.unwrap() * 0.2
                    && y < (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .y
                        / 2.
                        + catan.radius.unwrap() * 0.2
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
                    - catan.radius.unwrap() * 0.2
                    && x < (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .x
                        / 2.
                        + catan.radius.unwrap() * 0.2
                    && y > (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .y
                        / 2.
                        - catan.radius.unwrap() * 0.2
                    && y < (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        .y
                        / 2.
                        + catan.radius.unwrap() * 0.2
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

fn draw_buildable_roads(painter: &mut ShapePainter, catan: &ResMut<Catan>) {
    let config = painter.config().clone();
    let mut buildable_road = HashSet::new();

    for (road, player) in catan.inner.roads() {
        if *player == catan.me {
            for candidate in catan.inner.point_get_points(road.start) {
                if let Some(candidate) = candidate {
                    let road = Line::new(road.start, candidate);
                    if catan.inner.roads().get(&road).is_none()
                        && catan.inner.point_valid(candidate)
                        && Some(road) != catan.road_building
                    {
                        buildable_road.insert(road);
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
                        buildable_road.insert(road);
                    }
                }
            }
        }
    }

    match catan.road_building {
        Some(road) => {
            for candidate in catan.inner.point_get_points(road.start) {
                if let Some(candidate) = candidate {
                    let road = Line::new(road.start, candidate);
                    if catan.inner.roads().get(&road).is_none()
                        && catan.inner.point_valid(candidate)
                        && Some(road) != catan.road_building
                    {
                        buildable_road.insert(road);
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
                        buildable_road.insert(road);
                    }
                }
            }
        },
        None => {},
    }

    for road in buildable_road {
        painter.reset();
        painter.translate(
            (catan.points[road.start.x][road.start.y]
                + catan.points[road.end.x][road.end.y])
                / 2.,
        );
        painter.color = Color::rgb(1.0, 1.0, 1.0);
        painter.circle(catan.radius.unwrap() * 0.2);
    }
    painter.set_config(config);
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

fn draw_roads(
    painter: &mut ShapePainter, catan: &ResMut<Catan>, img_store: &Res<ImageStore>,
) {
    let config = painter.config().clone();

    for (road, player) in catan.inner.roads() {
        let start = catan.points[road.start.x][road.start.y];
        let end = catan.points[road.end.x][road.end.y];
        painter.reset();
        painter.translate((start + end) / 2. + Vec3::new(0., 0., -0.1));
        painter.rotate_z(road_get_rotate(start, end));
        painter.image(
            img_store.road_img[*player].clone(),
            Vec2::new(catan.radius.unwrap() * 0.5, catan.radius.unwrap()),
        );
    }

    if let Some(road) = &catan.road_building {
        let start = catan.points[road.start.x][road.start.y];
        let end = catan.points[road.end.x][road.end.y];
        painter.reset();
        painter.translate((start + end) / 2. + Vec3::new(0., 0., -0.1));
        painter.rotate_z(road_get_rotate(start, end));
        painter.image(
            img_store.road_img[catan.me].clone(),
            Vec2::new(catan.radius.unwrap() * 0.5, catan.radius.unwrap()),
        );
    }
    painter.set_config(config);
}

enum PointDraw {
    Img(Handle<Image>),
    Circle(Color),
}

fn draw_points<F>(painter: &mut ShapePainter, catan: &ResMut<Catan>, f: F)
where
    F: Fn(Coordinate) -> Option<PointDraw>,
{
    let config = painter.config().clone();

    for i in 0..catan.points.len() {
        for j in 0..catan.points[i].len() {
            let point = Coordinate { x: i, y: j };
            if let Some(draw) = f(point) {
                painter.reset();
                painter.translate(catan.points[i][j]);
                match draw {
                    PointDraw::Img(image) => {
                        painter.image(
                            image,
                            Vec2::new(catan.radius.unwrap(), catan.radius.unwrap()),
                        );
                    },
                    PointDraw::Circle(color) => {
                        painter.color = color;
                        painter.circle(catan.radius.unwrap() * 0.2);
                    },
                }
            }
        }
    }
    painter.set_config(config);
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
                    if x > catan.tiles[i][j].x - catan.radius.unwrap() * 0.9
                        && x < catan.tiles[i][j].x + catan.radius.unwrap() * 0.9
                        && y > catan.tiles[i][j].y - catan.radius.unwrap() * 0.9
                        && y < catan.tiles[i][j].y + catan.radius.unwrap() * 0.9
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
                    if x > catan.tiles[i][j].x - catan.radius.unwrap() * 0.9
                        && x < catan.tiles[i][j].x + catan.radius.unwrap() * 0.9
                        && y > catan.tiles[i][j].y - catan.radius.unwrap() * 0.9
                        && y < catan.tiles[i][j].y + catan.radius.unwrap() * 0.9
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

fn draw_tiles(
    painter: &mut ShapePainter, catan: &ResMut<Catan>, img_store: &Res<ImageStore>,
    state: &CatanState, card_state: &UseCardState,
) {
    let config = painter.config().clone();

    for i in 0..catan.tiles.len() {
        for j in 0..catan.tiles[i].len() {
            let coord = Coordinate { x: i, y: j };
            if catan.inner.tile(Coordinate { x: i, y: j }).kind() != TileKind::Empty {
                painter.reset();
                painter.translate(Vec3::new(
                    catan.tiles[i][j].x as f32,
                    catan.tiles[i][j].y as f32,
                    0.1,
                ));
                painter.image(
                    img_store.resource_img
                        [&catan.inner.tile(Coordinate { x: i, y: j }).kind()]
                        .clone(),
                    Vec2::new(catan.radius.unwrap() * 1.8, catan.radius.unwrap() * 1.8),
                );
                painter.translate(Vec3::new(0., 0., 0.1));
                match catan.inner.tile(coord).number() {
                    Some(number) => {
                        painter.image(
                            img_store.number_img[number].clone(),
                            Vec2::new(catan.radius.unwrap(), catan.radius.unwrap()),
                        );
                    },
                    _ => {},
                }
                if catan.inner.robber() == coord {
                    painter.translate(Vec3::new(0., 0., 0.1));
                    painter.image(
                        img_store.robber_img.clone(),
                        Vec2::new(catan.radius.unwrap() * 2., catan.radius.unwrap() * 2.),
                    );
                } else if (state.eq(&CatanState::SelectRobber)
                    || card_state.eq(&UseCardState::Knight))
                    && catan.inner.tile(Coordinate { x: i, y: j }).kind()
                        != TileKind::Dessert
                {
                    painter.translate(Vec3::new(0.0, 0.0, 0.1));
                    painter.color = Color::rgba(1.0, 1.0, 1.0, 0.5);
                    painter.circle(catan.radius.unwrap() * 0.5);
                }
            }
        }
    }
    painter.set_config(config);
}

fn draw_harbour(
    painter: &mut ShapePainter, catan: &ResMut<Catan>, img_store: &Res<ImageStore>,
) {
    let config = painter.config().clone();

    for (road, kind) in catan.inner.harbors() {
        match *kind {
            TileKind::Wood
            | TileKind::Brick
            | TileKind::Grain
            | TileKind::Wool
            | TileKind::Stone
            | TileKind::Dessert => {
                painter.reset();
                painter.translate(
                    (catan.points[road.start.x][road.start.y]
                        + catan.points[road.end.x][road.end.y])
                        / 2.,
                );
                painter.image(
                    img_store.resource_img[kind].clone(),
                    Vec2::new(catan.radius.unwrap() * 0.3, catan.radius.unwrap() * 0.3),
                );
            },
            _ => {
                unreachable!()
            },
        }
    }
    painter.set_config(config);
}

fn draw_board(
    mut painter: ShapePainter, mut catan: ResMut<Catan>, windows: Query<&Window>,
    state: Res<State<CatanState>>, card_state: Res<State<UseCardState>>,
    img_store: Res<ImageStore>,
) {
    for window in windows.iter() {
        info!(
            "window width: {} height: {}",
            window.width(),
            window.height()
        );
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let board_size = window.width().min(window.height() * 0.7);
        let board_translate = Vec3 {
            x: 0.0,
            y: window.height() / 2. - board_size / 2.0,
            z: 0.0,
        };
        let element_translate = Vec3 {
            x: -board_size / 2.0,
            y: -board_size / 2.0,
            z: 0.0,
        };
        painter.translate(board_translate);
        painter.with_children(|child_painter| {
            child_painter.translate(element_translate);

            let radius = board_size
                / if catan.tiles.len() % 2 == 0 {
                    (catan.tiles.len() * 3 / 2) as f32 + 0.25
                } else {
                    (catan.tiles.len() * 3 / 2) as f32 + 1.
                }
                .max(0.866 * 2. * catan.tiles[0].len() as f32);
            catan.update_radius(radius, child_painter.transform.translation);
            draw_tiles(
                child_painter,
                &catan,
                &img_store,
                state.get(),
                card_state.get(),
            );
            draw_harbour(child_painter, &catan, &img_store);
            draw_roads(child_painter, &catan, &img_store);
            draw_points(child_painter, &catan, |point| {
                if catan.inner.point_valid(point)
                    && catan.inner.point(point).owner().is_some()
                {
                    if !catan.inner.point(point).is_city() {
                        Some(PointDraw::Img(
                            img_store.settlement_img
                                [catan.inner.point(point).owner().unwrap()]
                            .clone(),
                        ))
                    } else {
                        Some(PointDraw::Img(
                            img_store.city_img[catan.inner.point(point).owner().unwrap()]
                                .clone(),
                        ))
                    }
                } else {
                    None
                }
            });

            if state.eq(&CatanState::BuidSettlement) {
                draw_points(child_painter, &catan, |point| {
                    if catan.inner.point_valid(point)
                        && catan.inner.point(point).owner().is_none()
                        && catan.players[catan.me].inner.can_build_settlement()
                        && catan.players[catan.me].inner.have_roads_to(point)
                        && !catan.inner.point_get_points(point).iter().any(|&p| {
                            if let Some(p) = p {
                                catan.inner.point(p).is_owned()
                            } else {
                                false
                            }
                        })
                    {
                        Some(PointDraw::Circle(Color::WHITE))
                    } else {
                        None
                    }
                });
            } else if state.eq(&CatanState::BuildCity) {
                draw_points(child_painter, &catan, |point| {
                    if catan.inner.point(point).owner().is_some()
                        && catan.inner.point(point).owner().unwrap() == catan.me
                        && !catan.inner.point(point).is_city()
                        && catan.players[catan.me].inner.can_build_city()
                    {
                        Some(PointDraw::Circle(Color::WHITE))
                    } else {
                        None
                    }
                });
            } else if state.eq(&CatanState::InitSettlement) {
                draw_points(child_painter, &catan, |point| {
                    if catan.inner.point_valid(point)
                        && catan.inner.point(point).owner().is_none()
                        && !catan.inner.point_get_points(point).iter().any(|&p| {
                            if let Some(p) = p {
                                catan.inner.point(p).is_owned()
                            } else {
                                false
                            }
                        })
                    {
                        Some(PointDraw::Circle(Color::WHITE))
                    } else {
                        None
                    }
                });
            } else if state.eq(&CatanState::BuildRoad)
                || card_state.eq(&UseCardState::RoadBuilding)
            {
                draw_buildable_roads(child_painter, &catan);
            } else if state.eq(&CatanState::InitRoad) {
                draw_initable_roads(child_painter, &catan);
            }
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
fn intialize_game(mut next_state: ResMut<NextState<CatanLoadState>>) {
    next_state.set(CatanLoadState::Loaded);
}

fn draw_player_board(
    mut painter: ShapePainter, catan: Res<Catan>, windows: Query<&Window>,
    img_store: Res<ImageStore>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let player_card_y_size = window.height() * 0.1;
        let player_card_x_size =
            (window.width() - window.height() * 0.2) / catan.players.len() as f32;
        let board_translate = Vec3 {
            x: 0.0,
            y: -window.height() * 0.25,
            z: 0.0,
        };

        painter.translate(board_translate);
        painter.with_children(|child_painter| {
            child_painter.translate(Vec3 {
                x: -player_card_y_size / 2.,
                y: 0.0,
                z: 0.1,
            });
            child_painter.image(
                img_store.dice_img[catan.dice.0 as usize - 1].clone(),
                Vec2 {
                    x: player_card_y_size,
                    y: player_card_y_size,
                },
            );
            child_painter.translate(Vec3 {
                x: player_card_y_size,
                y: 0.0,
                z: 0.1,
            });
            child_painter.image(
                img_store.dice_img[catan.dice.1 as usize - 1].clone(),
                Vec2 {
                    x: player_card_y_size,
                    y: player_card_y_size,
                },
            );
        });
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Default, Debug, Clone, PartialEq)]
struct OperationEntry {
    operation: Operation,
    translate: Vec3,
    size: Vec2,
}

#[derive(Resource)]
struct OperationMenu([OperationEntry; 7]);

fn draw_menu(
    mut painter: ShapePainter, windows: Query<&Window>, mut menu: ResMut<OperationMenu>,
    img_store: Res<ImageStore>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let operation_board_x_size = window.width().min(window.height()) * 0.5;
        let operation_board_y_size = window.width().min(window.height()) * 0.1;

        let board_translate = Vec3 {
            x: 0.0,
            y: -window.height() * 0.45,
            z: 0.0,
        };

        painter.translate(board_translate);
        painter.with_children(|spawn_children| {
            let config = spawn_children.config().clone();
            let operation_size = operation_board_x_size / menu.0.len() as f32;
            let xoffset = -operation_board_x_size / 2. + operation_size * 0.5;
            for (i, oper) in menu.0.iter_mut().enumerate() {
                let size = Vec2 {
                    x: operation_size * 0.95,
                    y: operation_size * 0.95,
                };
                spawn_children.set_config(config.clone());
                spawn_children.translate(Vec3 {
                    x: xoffset + operation_size * i as f32,
                    y: 0.0,
                    z: 0.1,
                });
                spawn_children.color = Color::rgb(0.2, 0.5, 0.5);
                spawn_children.rect(size);
                spawn_children.translate(Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.1,
                });
                spawn_children
                    .image(img_store.operation_img[&oper.operation].clone(), size);
                oper.translate = spawn_children.transform.translation;
                oper.size = size;
            }
        });
    }
}

fn limit_frame(mut settings: ResMut<bevy_framepace::FramepaceSettings>) {
    settings.limiter = bevy_framepace::Limiter::from_framerate(10.0);
}

fn change_state(
    keyboard_input: Res<ButtonInput<KeyCode>>, state: Res<State<CatanState>>,
    mut next_state: ResMut<NextState<CatanState>>,
) {
    if keyboard_input.pressed(KeyCode::Space) {
        match state.get() {
            CatanState::Wait => {
                next_state.set(CatanState::Menu);
            },
            CatanState::Menu => {
                next_state.set(CatanState::BuidSettlement);
            },
            CatanState::BuidSettlement => {
                next_state.set(CatanState::BuildCity);
            },
            CatanState::BuildCity => {
                next_state.set(CatanState::BuildRoad);
            },
            CatanState::BuildRoad => {
                next_state.set(CatanState::Trade);
            },
            CatanState::Trade => {
                next_state.set(CatanState::SelectRobber);
            },
            CatanState::SelectRobber => {
                next_state.set(CatanState::Wait);
            },
            _ => {},
        }
    };
}

fn check_menu_click(
    windows: Query<&Window>, mouse_button_input: Res<ButtonInput<MouseButton>>,
    state: Res<State<CatanState>>, mut next_state: ResMut<NextState<CatanState>>,
    mut next_trade_state: ResMut<NextState<TradeState>>,
    mut next_card_state: ResMut<NextState<UseCardState>>, menu: ResMut<OperationMenu>,
    catan: Res<Catan>, mut trade: ResMut<TradeBoard>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);

            for i in 0..menu.0.len() {
                let entry = &menu.0[i];
                if x > entry.translate.x - entry.size.x / 2.
                    && x < entry.translate.x + entry.size.x / 2.
                    && y > entry.translate.y - entry.size.y / 2.
                    && y < entry.translate.y + entry.size.y / 2.
                {
                    match entry.operation {
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
                }
            }
        }
    }
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
            let card_board_x_size = windows.iter().next().unwrap().width();
            let card_board_y_size = windows.iter().next().unwrap().height() * 0.2;
            let card_size = card_board_y_size * 0.9;
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

fn draw_resource(
    mut painter: ShapePainter, catan: ResMut<Catan>, windows: Query<&Window>,
    img_store: Res<ImageStore>,
) {
    for window in windows.iter() {
        painter.color = Color::rgb(0.0, 0.0, 0.0);
        let operation_board_x_size = window.width().min(window.height()) * 0.5;
        let operation_board_y_size = window.width().min(window.height()) * 0.1;

        let board_translate = Vec3 {
            x: 0.0,
            y: -window.height() * 0.35,
            z: 0.0,
        };

        painter.translate(board_translate);
        painter.with_children(|spawn_children| {
            let config = spawn_children.config().clone();
            let operation_size = operation_board_x_size
                / img_store
                    .resource_img
                    .iter()
                    .filter(|res| res.0.is_resource())
                    .count() as f32;
            let size = (operation_size * 0.95).min(operation_board_y_size * 0.95);
            let xoffset = -operation_board_x_size * 0.5 + operation_size * 0.5;
            let mut i = 0;
            for j in 0..catan.players[catan.me].inner.resources.len() {
                let kind = &TileKind::try_from(j as u8).unwrap();
                if !kind.is_resource() {
                    continue;
                }
                let res = img_store.resource_img.get(kind).unwrap();
                let size = Vec2 { x: size, y: size };
                spawn_children.set_config(config.clone());
                spawn_children.translate(Vec3 {
                    x: xoffset + operation_size * i as f32,
                    y: 0.0,
                    z: 0.1,
                });
                spawn_children.image(res.clone(), size);
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
        });
    }
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
    offer: HashMap<TileKind, (Vec3, Vec3)>,
    want: HashMap<TileKind, (Vec3, Vec3)>,
    response: HashMap<usize, Vec3>,
    bank_yes: Vec3,
    harbor_yes: Vec3,
    player_yes: Vec3,
    no: Vec3,
    button_size: f32,
    icon_size: f32,
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
    mut next_trade_state: ResMut<NextState<TradeState>>, trade: Res<TradeBoard>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        // convert the mouse position to the window position
        if let Some(mouse) = windows.iter().next().unwrap().cursor_position() {
            let x = mouse.x - windows.iter().next().unwrap().width() / 2.;
            let y = -(mouse.y - windows.iter().next().unwrap().height() / 2.);

            for yes in trade.draw.response.iter() {
                if x > yes.1.x - trade.draw.icon_size / 2.0
                    && x < yes.1.x + trade.draw.icon_size / 2.0
                    && y > yes.1.y - trade.draw.icon_size / 2.0
                    && y < yes.1.y + trade.draw.icon_size / 2.0
                {
                    action_writer.send(GameAct::TradeConfirm(Some(*yes.0)).into());
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

            for (kind, (add, sub)) in trade.draw.offer.iter() {
                if x > add.x - trade.draw.button_size
                    && x < add.x + trade.draw.button_size
                    && y > add.y - trade.draw.button_size
                    && y < add.y + trade.draw.button_size
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
                if x > sub.x - trade.draw.button_size
                    && x < sub.x + trade.draw.button_size
                    && y > sub.y - trade.draw.button_size
                    && y < sub.y + trade.draw.button_size
                {
                    let k = kind.clone();
                    trade.resource.offer[k as usize] =
                        trade.resource.offer[k as usize].saturating_sub(1);
                    return;
                }
            }

            for (kind, (add, sub)) in trade.draw.want.iter() {
                if x > add.x - trade.draw.button_size
                    && x < add.x + trade.draw.button_size
                    && y > add.y - trade.draw.button_size
                    && y < add.y + trade.draw.button_size
                {
                    let k = kind.clone();
                    trade.resource.want[k as usize] += 1;
                    trade.resource.want[k as usize] =
                        trade.resource.want[k as usize].min(20);
                    return;
                }
                if x > sub.x - trade.draw.button_size
                    && x < sub.x + trade.draw.button_size
                    && y > sub.y - trade.draw.button_size
                    && y < sub.y + trade.draw.button_size
                {
                    let k = kind.clone();
                    trade.resource.want[k as usize] =
                        trade.resource.want[k as usize].saturating_sub(1);
                    return;
                }
            }

            if x > trade.draw.bank_yes.x - trade.draw.bank_yes.z
                && x < trade.draw.bank_yes.x + trade.draw.bank_yes.z
                && y > trade.draw.bank_yes.y - trade.draw.bank_yes.z
                && y < trade.draw.bank_yes.y + trade.draw.bank_yes.z
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

            if x > trade.draw.harbor_yes.x - trade.draw.harbor_yes.z
                && x < trade.draw.harbor_yes.x + trade.draw.harbor_yes.z
                && y > trade.draw.harbor_yes.y - trade.draw.harbor_yes.z
                && y < trade.draw.harbor_yes.y + trade.draw.harbor_yes.z
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

            if x > trade.draw.player_yes.x - trade.draw.player_yes.z
                && x < trade.draw.player_yes.x + trade.draw.player_yes.z
                && y > trade.draw.player_yes.y - trade.draw.player_yes.z
                && y < trade.draw.player_yes.y + trade.draw.player_yes.z
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

            if x > trade.draw.no.x - trade.draw.no.z
                && x < trade.draw.no.x + trade.draw.no.z
                && y > trade.draw.no.y - trade.draw.no.z
                && y < trade.draw.no.y + trade.draw.no.z
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

            if x > trade.draw.player_yes.x - trade.draw.player_yes.z
                && x < trade.draw.player_yes.x + trade.draw.player_yes.z
                && y > trade.draw.player_yes.y - trade.draw.player_yes.z
                && y < trade.draw.player_yes.y + trade.draw.player_yes.z
            {
                action_writer.send(GameAct::TradeResponse(TradeResponse::Accept).into());
                next_trade_state.set(TradeState::WaitingConfirm);
                return;
            }

            if x > trade.draw.no.x - trade.draw.no.z
                && x < trade.draw.no.x + trade.draw.no.z
                && y > trade.draw.no.y - trade.draw.no.z
                && y < trade.draw.no.y + trade.draw.no.z
            {
                action_writer.send(GameAct::TradeResponse(TradeResponse::Accept).into());
                next_trade_state.set(TradeState::WaitingConfirm);
                return;
            }
        }
    }
}

fn draw_trade(
    mut painter: ShapePainter, catan: ResMut<Catan>, windows: Query<&Window>,
    img_store: Res<ImageStore>, mut trade: ResMut<TradeBoard>,
    state: Res<State<TradeState>>,
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
                let trade_size = trade_board_size
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
                        x: trade_size,
                        y: trade_size,
                    };

                    //offer
                    let translate = Vec3 {
                        x: -trade_board_size / 2. + trade_size / 2 as f32,
                        y: trade_board_size / 2.
                            - trade_size / 2.
                            - trade_size * i as f32,
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
                    if state.eq(&TradeState::Offering) {
                        //add
                        spawn_children.set_config(config.clone());
                        spawn_children.translate(
                            Vec3 {
                                x: trade_size,
                                y: trade_size / 4.,
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
                                x: trade_size,
                                y: -trade_size / 4.,
                                z: 0.,
                            } + translate,
                        );
                        spawn_children.image(
                            img_store.sub.clone(),
                            Vec2::new(size.x * 0.5, size.y * 0.5),
                        );
                        let sub_translate = spawn_children.transform.translation;
                        trade
                            .draw
                            .offer
                            .insert(*kind, (add_translate, sub_translate));
                    }
                    //count
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(
                        Vec3 {
                            x: trade_size * 1.25,
                            y: 0.,
                            z: 0.1,
                        } + translate,
                    );
                    spawn_children.image(
                        img_store.number_img
                            [(trade.resource.offer[*kind as usize] as usize).min(20)]
                        .clone(),
                        Vec2::new(size.x * 0.5, size.y * 0.5),
                    );

                    //want
                    let translate = Vec3 {
                        x: trade_board_size / 2. - trade_size / 2 as f32,
                        y: trade_board_size / 2.
                            - trade_size / 2.
                            - trade_size * i as f32,
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
                    if state.eq(&TradeState::Offering) {
                        //add
                        spawn_children.set_config(config.clone());
                        spawn_children.translate(
                            Vec3 {
                                x: -trade_size,
                                y: trade_size / 4.,
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
                                x: -trade_size,
                                y: -trade_size / 4.,
                                z: 0.,
                            } + translate,
                        );
                        spawn_children.image(
                            img_store.sub.clone(),
                            Vec2::new(size.x * 0.5, size.y * 0.5),
                        );
                        let sub_translate = spawn_children.transform.translation;
                        trade
                            .draw
                            .want
                            .insert(*kind, (add_translate, sub_translate));
                    }
                    //count
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(
                        Vec3 {
                            x: -trade_size * 1.25,
                            y: 0.,
                            z: 0.1,
                        } + translate,
                    );
                    spawn_children.image(
                        img_store.number_img
                            [(trade.resource.want[*kind as usize] as usize).min(20)]
                        .clone(),
                        Vec2::new(size.x * 0.5, size.y * 0.5),
                    );
                    trade.draw.button_size = trade_size * 0.25;
                    i += 1;
                }

                if state.eq(&TradeState::Offering) {
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(Vec3 {
                        x: 0.,
                        y: trade_size * 1.5, // * 2.,
                        z: 0.1,
                    });
                    if catan
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
                        spawn_children.image(
                            img_store.bank_img.clone(),
                            Vec2::new(trade_size, trade_size),
                        );
                    }
                    trade.draw.bank_yes = spawn_children.transform.translation;
                    trade.draw.bank_yes.z = trade_size / 2.;
                    spawn_children.translate(Vec3 {
                        x: 0.,
                        y: -trade_size, // * 2.,
                        z: 0.0,
                    });

                    if catan
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
                        spawn_children.image(
                            img_store.harbor_img.clone(),
                            Vec2::new(trade_size, trade_size),
                        );
                    }
                    trade.draw.harbor_yes = spawn_children.transform.translation;
                    trade.draw.harbor_yes.z = trade_size / 2.;

                    spawn_children.translate(Vec3 {
                        x: 0.,
                        y: -trade_size, // * 2.,
                        z: 0.0,
                    });
                    spawn_children
                        .image(img_store.yes.clone(), Vec2::new(trade_size, trade_size));
                    trade.draw.player_yes = spawn_children.transform.translation;
                    trade.draw.player_yes.z = trade_size / 2.;
                    spawn_children.translate(Vec3 {
                        x: 0.,
                        y: -trade_size, // * 2.,
                        z: 0.0,
                    });
                    spawn_children
                        .image(img_store.no.clone(), Vec2::new(trade_size, trade_size));
                    trade.draw.no = spawn_children.transform.translation;
                    trade.draw.no.z = trade_size / 2.;
                } else if state.eq(&TradeState::Accepting) {
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(Vec3 {
                        x: 0.,
                        y: trade_size / 2., // * 2.,
                        z: 0.1,
                    });
                    spawn_children
                        .image(img_store.yes.clone(), Vec2::new(trade_size, trade_size));
                    trade.draw.player_yes = spawn_children.transform.translation;
                    trade.draw.player_yes.z = trade_size / 2.;
                    spawn_children.translate(Vec3 {
                        x: 0.,
                        y: -trade_size, // * 2.,
                        z: 0.1,
                    });
                    spawn_children
                        .image(img_store.no.clone(), Vec2::new(trade_size, trade_size));
                    trade.draw.no = spawn_children.transform.translation;
                    trade.draw.no.z = trade_size / 2.;
                } else if state.eq(&TradeState::WaitingResponse)
                    || state.eq(&TradeState::Confirming)
                {
                    let icon_size = trade_size * 0.8;

                    trade.draw.icon_size = icon_size;
                    spawn_children.set_config(config.clone());
                    spawn_children.translate(Vec3 {
                        x: 0.,
                        y: 0.,
                        z: 0.1,
                    });

                    let i = 0;
                    for id in 0..catan.players.len() {
                        if id != catan.me {
                            spawn_children.set_config(config.clone());
                            spawn_children.translate(Vec3 {
                                x: 0.,
                                y: trade_size / 2.
                                    - icon_size / 2.
                                    - icon_size * i as f32,
                                z: 0.1,
                            });
                            spawn_children.image(
                                img_store.settlement_img[id].clone(),
                                Vec2::new(icon_size, icon_size),
                            );
                            spawn_children.translate(Vec3 {
                                x: 0.,
                                y: -icon_size / 2.,
                                z: 0.1,
                            });
                            match trade.resource.response.get(&id) {
                                Some(TradeResponse::Accept) => {
                                    trade
                                        .draw
                                        .response
                                        .insert(id, spawn_children.transform.translation);

                                    spawn_children.image(
                                        img_store.yes.clone(),
                                        Vec2::new(icon_size * 0.4, icon_size * 0.4),
                                    );
                                },

                                Some(TradeResponse::Reject) => {
                                    spawn_children.image(
                                        img_store.no.clone(),
                                        Vec2::new(icon_size * 0.4, icon_size * 0.4),
                                    );
                                },
                                None => {},
                            }
                        }
                    }
                }
            });
    }
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
                let catan = Catan::new(start.clone());

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
    mut catan: ResMut<Catan>, mut trade: ResMut<TradeBoard>,
    trade_state: Res<State<TradeState>>, mut next_state: ResMut<NextState<CatanState>>,
    mut next_trade_state: ResMut<NextState<TradeState>>,
    mut event_reader: ConsumableEventReader<GameEvent>,
    mut action_writer: ConsumableEventWriter<GameAction>,
) {
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
                catan.dice = (dice1, dice2);
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
                    catan.players[buy.player].inner.resources
                        [TileKind::Stone as usize] -= 1;
                    catan.players[buy.player].inner.resources
                        [TileKind::Grain as usize] -= 1;
                    catan.players[buy.player].inner.resources[TileKind::Wool as usize] -=
                        1;
                    catan.players[buy.player].inner.add_card(buy.card);
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
                        catan.players[use_card.player].inner.resources[kind1 as usize] +=
                            1;
                        catan.players[use_card.player].inner.resources[kind2 as usize] +=
                            1;
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
                if trade_state.eq(&TradeState::WaitingResponse) && player != catan.me {
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
                            catan.players[to].inner.resources[*kind as usize] += count;
                        }
                    }
                    for (kind, count) in trade.request.to() {
                        catan.players[trade.from].inner.resources[*kind as usize] +=
                            count;
                        if let Some(to) = trade.to {
                            catan.players[to].inner.resources[*kind as usize] -= count;
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
        .insert_resource(OperationMenu([
            OperationEntry {
                operation: Operation::BuildSettlement,
                ..default()
            },
            OperationEntry {
                operation: Operation::BuildCity,
                ..default()
            },
            OperationEntry {
                operation: Operation::BuildRoad,
                ..default()
            },
            OperationEntry {
                operation: Operation::Trade,
                ..default()
            },
            OperationEntry {
                operation: Operation::BuyCard,
                ..default()
            },
            OperationEntry {
                operation: Operation::UseCard,
                ..default()
            },
            OperationEntry {
                operation: Operation::EndTurn,
                ..default()
            },
        ]))
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
        .add_systems(
            Update,
            (
                process_action,
                (
                    process_event.run_if(in_state(CatanState::Menu)),
                    process_event.run_if(in_state(CatanState::Wait)),
                    process_event.run_if(in_state(CatanState::Trade)),
                ),
                check_init_settlement.run_if(in_state(CatanState::InitSettlement)),
                check_init_road.run_if(in_state(CatanState::InitRoad)),
                check_build_road.run_if(in_state(CatanState::BuildRoad)),
                check_build_settlement.run_if(in_state(CatanState::BuidSettlement)),
                check_build_city.run_if(in_state(CatanState::BuildCity)),
                check_select_robber.run_if(in_state(CatanState::SelectRobber)),
                check_steal_target.run_if(in_state(CatanState::Stealing)),
                draw_board,
                draw_player_board,
                draw_resource,
                (draw_drop_resource, check_drop_click)
                    .run_if(in_state(CatanState::DropResource)),
                draw_steal_target.run_if(in_state(CatanState::Stealing)),
                (draw_menu, check_menu_click)
                    .run_if(not(in_state(CatanState::Wait)))
                    .run_if(not(in_state(CatanState::InitRoad)))
                    .run_if(not(in_state(CatanState::InitSettlement)))
                    .run_if(not(in_state(CatanState::SelectRobber)))
                    .run_if(not(in_state(CatanState::Stealing)))
                    .run_if(not(in_state(CatanState::DropResource))),
                (
                    draw_trade,
                    check_trade_offering_click.run_if(in_state(TradeState::Offering)),
                    check_trade_accepting_click.run_if(in_state(TradeState::Accepting)),
                    check_trade_confirm_click.run_if(in_state(TradeState::Confirming)),
                )
                    .run_if(in_state(CatanState::Trade)),
                (
                    (draw_development_card, check_development_card_click)
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
                update_player_text,
                change_state,
            )
                .run_if(in_state(CatanLoadState::Loaded)),
        )
        .run();
}
