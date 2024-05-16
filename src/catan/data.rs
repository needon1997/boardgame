use super::element::{Point, TileKind};
use crate::{
    catan::element::{DevCard, Tile},
    common::element::{Coordinate, Line},
};
use rand::{prelude::SliceRandom, thread_rng};
use std::collections::HashMap;

pub struct CatanData {
    pub robber: Coordinate,
    pub dev_cards: Vec<DevCard>,
    pub tiles: Vec<Vec<Tile>>,
    pub dics_map: HashMap<usize, Vec<Coordinate>>,
    pub harbors: Vec<(Line, TileKind)>,
    pub points: Vec<Vec<Point>>,
    pub winscore: u8,
}

pub enum CatanDataSetup {
    Basic,
}

impl CatanData {
    pub fn new(setup: CatanDataSetup) -> Self {
        match setup {
            CatanDataSetup::Basic => Self::basic(),
        }
    }

    fn basic() -> Self {
        const ASSIGNABLE: [[bool; 5]; 5] = [
            [false, true, true, true, false],
            [true, true, true, true, false],
            [true, true, true, true, true],
            [true, true, true, true, false],
            [false, true, true, true, false],
        ];

        const HARBOR_CANDIATE: [Line; 9] = [
            Line {
                start: Coordinate { x: 0, y: 1 },
                end: Coordinate { x: 1, y: 0 },
            },
            Line {
                start: Coordinate { x: 0, y: 3 },
                end: Coordinate { x: 1, y: 4 },
            },
            Line {
                start: Coordinate { x: 1, y: 0 },
                end: Coordinate { x: 2, y: 0 },
            },
            Line {
                start: Coordinate { x: 1, y: 4 },
                end: Coordinate { x: 2, y: 4 },
            },
            Line {
                start: Coordinate { x: 2, y: 0 },
                end: Coordinate { x: 3, y: 0 },
            },
            Line {
                start: Coordinate { x: 2, y: 4 },
                end: Coordinate { x: 3, y: 4 },
            },
            Line {
                start: Coordinate { x: 3, y: 0 },
                end: Coordinate { x: 4, y: 1 },
            },
            Line {
                start: Coordinate { x: 3, y: 4 },
                end: Coordinate { x: 4, y: 3 },
            },
            Line {
                start: Coordinate { x: 4, y: 1 },
                end: Coordinate { x: 4, y: 3 },
            },
        ];

        const TILE_KINDS: [TileKind; 19] = [
            TileKind::Dessert,
            TileKind::Wool,
            TileKind::Wool,
            TileKind::Wool,
            TileKind::Wool,
            TileKind::Wood,
            TileKind::Wood,
            TileKind::Wood,
            TileKind::Wood,
            TileKind::Grain,
            TileKind::Grain,
            TileKind::Grain,
            TileKind::Grain,
            TileKind::Stone,
            TileKind::Stone,
            TileKind::Stone,
            TileKind::Brick,
            TileKind::Brick,
            TileKind::Brick,
        ];

        const HARBOR_TILE_KINDS: [TileKind; 9] = [
            TileKind::Wool,
            TileKind::Wood,
            TileKind::Grain,
            TileKind::Stone,
            TileKind::Brick,
            TileKind::Empty,
            TileKind::Empty,
            TileKind::Empty,
            TileKind::Empty,
        ];

        const DEVELOPMENT_CARD: [(DevCard, usize); 5] = [
            (DevCard::Knight, 14),
            (DevCard::VictoryPoint, 5),
            (DevCard::RoadBuilding, 2),
            (DevCard::Monopoly, 2),
            (DevCard::YearOfPlenty, 2),
        ];

        const DICE_COUNT: [u8; 13] = [0, 0, 1, 2, 2, 2, 2, 1, 2, 2, 2, 2, 1];

        let mut tile_kind = TILE_KINDS.to_vec();
        let mut harbor_tile_kind = HARBOR_TILE_KINDS.to_vec();
        let mut tiles = vec![vec![Tile::default(); 5]; 5];
        let mut valid_dice_coord = Vec::new();
        let mut dics_map = HashMap::new();
        let mut harbors = Vec::new();
        let mut robber = Coordinate { x: 0, y: 0 };

        tile_kind.shuffle(&mut thread_rng());
        harbor_tile_kind.shuffle(&mut thread_rng());

        for i in 0..5 {
            for j in 0..5 {
                if ASSIGNABLE[i][j] {
                    let kind = tile_kind.pop().unwrap();
                    tiles[i][j].set_kind(kind);
                    valid_dice_coord.push(Coordinate { x: i, y: j });
                    if kind == TileKind::Dessert {
                        robber = Coordinate { x: i, y: j };
                        let entry = dics_map.entry(7).or_insert(Vec::new());
                        entry.push(robber);
                    }
                }
            }
        }

        for i in 0..9 {
            let harbor = HARBOR_CANDIATE[i];
            let kind = harbor_tile_kind.pop().unwrap();
            harbors.push((harbor, kind));
        }

        valid_dice_coord.shuffle(&mut thread_rng());
        for i in 0..DICE_COUNT.len() {
            let mut dice = DICE_COUNT[i];
            while dice > 0 {
                if i == 7 {
                    break;
                }
                let c = valid_dice_coord.pop().unwrap();
                if c == robber {
                    continue;
                }
                let entry = dics_map.entry(i).or_insert(Vec::new());
                entry.push(c);
                tiles[c.x][c.y].set_number(i);
                dice -= 1;
            }
        }

        let mut dev_cards = Vec::new();
        for (card, count) in DEVELOPMENT_CARD.iter() {
            for _ in 0..*count {
                dev_cards.push(*card);
            }
        }
        dev_cards.shuffle(&mut thread_rng());

        let points_x = tiles.len() + 1;
        let points_y = tiles.len() * 2 + 1;
        let points = vec![vec![Point::default(); points_y]; points_x];

        Self {
            robber,
            dev_cards,
            tiles,
            dics_map,
            harbors,
            points,
            winscore: 10,
        }
    }
}
