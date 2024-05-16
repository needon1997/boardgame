use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::common::element::{Coordinate, Line};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DevCard {
    Knight,
    VictoryPoint,
    RoadBuilding,
    Monopoly,
    YearOfPlenty,
    Max,
}

impl TryFrom<u8> for DevCard {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DevCard::Knight),
            1 => Ok(DevCard::VictoryPoint),
            2 => Ok(DevCard::RoadBuilding),
            3 => Ok(DevCard::Monopoly),
            4 => Ok(DevCard::YearOfPlenty),
            _ => Err(()),
        }
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TileKind {
    #[default]
    Empty,
    Dessert,
    Wood,
    Brick,
    Grain,
    Wool,
    Stone,
    Max,
}

impl TryFrom<u8> for TileKind {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TileKind::Empty),
            1 => Ok(TileKind::Dessert),
            2 => Ok(TileKind::Wood),
            3 => Ok(TileKind::Brick),
            4 => Ok(TileKind::Grain),
            5 => Ok(TileKind::Wool),
            6 => Ok(TileKind::Stone),
            _ => Err(()),
        }
    }
}

impl TileKind {
    pub fn is_resource(&self) -> bool {
        *self == TileKind::Wood
            || *self == TileKind::Brick
            || *self == TileKind::Grain
            || *self == TileKind::Wool
            || *self == TileKind::Stone
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub struct Tile {
    kind: TileKind,
    number: Option<usize>,
}

impl Tile {
    pub fn is_empty(&self) -> bool {
        self.kind == TileKind::Empty
    }

    pub fn set_number(&mut self, number: usize) {
        self.number = Some(number);
    }

    pub fn number(&self) -> Option<usize> {
        self.number
    }

    pub fn set_kind(&mut self, kind: TileKind) {
        self.kind = kind;
    }

    pub fn kind(&self) -> TileKind {
        self.kind
    }

    pub fn is_resource(&self) -> bool {
        self.kind.is_resource()
    }
}

#[derive(Default, Copy, Clone)]
pub struct Point {
    pub owner: Option<usize>,
    pub city: bool,
}

impl Point {
    pub fn is_owned(&self) -> bool {
        self.owner.is_some()
    }

    pub fn is_city(&self) -> bool {
        self.city
    }

    pub fn owner(&self) -> Option<usize> {
        self.owner
    }

    pub fn set_owner(&mut self, owner: usize) {
        self.owner = Some(owner);
    }
}

pub struct CatanCommon {
    tiles: Vec<Vec<Tile>>,
    points: Vec<Vec<Point>>,
    roads: HashMap<Line, usize>,
    harbors: Vec<(Line, TileKind)>,
    robber: Coordinate,
    dice_map: HashMap<usize, Vec<Coordinate>>,
}

impl CatanCommon {
    pub fn new(
        tiles: Vec<Vec<Tile>>, points: Vec<Vec<Point>>, roads: HashMap<Line, usize>,
        harbors: Vec<(Line, TileKind)>, dice_map: HashMap<usize, Vec<Coordinate>>,
        robber: Coordinate,
    ) -> Self {
        Self {
            tiles,
            points,
            roads,
            harbors,
            robber,
            dice_map,
        }
    }

    pub fn tile(&self, coordinate: Coordinate) -> &Tile {
        &self.tiles[coordinate.x][coordinate.y]
    }

    pub fn tiles(&self) -> &Vec<Vec<Tile>> {
        &self.tiles
    }

    pub fn dice_map(&self) -> &HashMap<usize, Vec<Coordinate>> {
        &self.dice_map
    }

    pub fn harbors(&self) -> &Vec<(Line, TileKind)> {
        &self.harbors
    }

    pub fn roads(&self) -> &HashMap<Line, usize> {
        &self.roads
    }

    pub fn set_robber(&mut self, coord: Coordinate) {
        self.robber = coord;
    }

    pub fn robber(&self) -> Coordinate {
        self.robber
    }

    pub fn point(&self, coordinate: Coordinate) -> Point {
        self.points[coordinate.x][coordinate.y]
    }

    pub fn add_road(&mut self, player: usize, road: Line) -> Option<usize> {
        self.roads.insert(road, player)
    }

    pub fn add_settlement(&mut self, player: usize, point: Coordinate) {
        self.points[point.x][point.y].set_owner(player);
    }

    pub fn add_city(&mut self, player: usize, point: Coordinate) {
        self.points[point.x][point.y].city = true;
        self.points[point.x][point.y].set_owner(player);
    }

    pub fn tile_get_points(&self, coord: Coordinate) -> [Coordinate; 6] {
        let mut points = [Default::default(); 6];
        let x = coord.x;
        let y = coord.y;

        if x % 2 == 0 {
            points[0] = Coordinate::new(x, 2 * y);
            points[1] = Coordinate::new(x, 2 * y + 1);
            points[2] = Coordinate::new(x, 2 * y + 2);
            points[3] = Coordinate::new(x + 1, 2 * y);
            points[4] = Coordinate::new(x + 1, 2 * y + 1);
            points[5] = Coordinate::new(x + 1, 2 * y + 2);
        } else {
            points[0] = Coordinate::new(x, 2 * y + 1);
            points[1] = Coordinate::new(x, 2 * y + 2);
            points[2] = Coordinate::new(x, 2 * y + 3);
            points[3] = Coordinate::new(x + 1, 2 * y + 1);
            points[4] = Coordinate::new(x + 1, 2 * y + 2);
            points[5] = Coordinate::new(x + 1, 2 * y + 3);
        }
        points
    }

    pub fn point_get_points(&self, coord: Coordinate) -> [Option<Coordinate>; 3] {
        let mut points = [Default::default(); 3];
        let x = coord.x;
        let y = coord.y;

        if x % 2 == 0 && y % 2 == 0 {
            points[0] = if y >= 1 {
                Some(Coordinate::new(x, y - 1))
            } else {
                None
            };
            points[1] = if y < self.points[0].len() - 1 {
                Some(Coordinate::new(x, y + 1))
            } else {
                None
            };
            points[2] = if x < self.points.len() - 1 {
                Some(Coordinate::new(x + 1, y))
            } else {
                None
            };
        } else if x % 2 == 0 && y % 2 == 1 {
            points[0] = if y >= 1 {
                Some(Coordinate::new(x, y - 1))
            } else {
                None
            };
            points[1] = if y < self.points[0].len() - 1 {
                Some(Coordinate::new(x, y + 1))
            } else {
                None
            };
            points[2] = if x >= 1 {
                Some(Coordinate::new(x - 1, y))
            } else {
                None
            };
        } else if x % 2 == 1 && y % 2 == 0 {
            points[0] = if y >= 1 {
                Some(Coordinate::new(x, y - 1))
            } else {
                None
            };
            points[1] = if y < self.points[0].len() - 1 {
                Some(Coordinate::new(x, y + 1))
            } else {
                None
            };
            points[2] = if x >= 1 {
                Some(Coordinate::new(x - 1, y))
            } else {
                None
            };
        } else {
            points[0] = if y >= 1 {
                Some(Coordinate::new(x, y - 1))
            } else {
                None
            };
            points[1] = if y < self.points[0].len() - 1 {
                Some(Coordinate::new(x, y + 1))
            } else {
                None
            };
            points[2] = if x < self.points.len() - 1 {
                Some(Coordinate::new(x + 1, y))
            } else {
                None
            };
        }
        points
    }

    pub fn point_valid(&self, point: Coordinate) -> bool {
        for p in self.ponint_get_tile(point) {
            if let Some(p) = p {
                if !self.tile(p).is_empty() {
                    return true;
                }
            }
        }
        return false;
    }

    pub fn ponint_get_tile(&self, coord: Coordinate) -> [Option<Coordinate>; 3] {
        let mut tiles = [None; 3];
        let x = coord.x;
        let y = coord.y;

        if x >= self.points.len() || y >= self.points[0].len() {
            return tiles;
        }

        if y % 2 == 0 {
            if x % 2 == 1 {
                tiles[0] = if x >= 1 && y >= 2 {
                    Some(Coordinate::new(x - 1, (y - 2) / 2))
                } else {
                    None
                };

                tiles[1] = if x >= 1 && y / 2 < self.tiles[0].len() {
                    Some(Coordinate::new(x - 1, y / 2))
                } else {
                    None
                };
                tiles[2] = if y >= 2 && x < self.tiles.len() {
                    Some(Coordinate::new(x, (y - 2) / 2))
                } else {
                    None
                };
            } else {
                tiles[0] = if x >= 1 && y >= 2 {
                    Some(Coordinate::new(x - 1, (y - 2) / 2))
                } else {
                    None
                };
                tiles[1] = if y >= 2 && x < self.tiles.len() {
                    Some(Coordinate::new(x, (y - 2) / 2))
                } else {
                    None
                };
                tiles[2] = if y / 2 < self.tiles[0].len() && x < self.tiles.len() {
                    Some(Coordinate::new(x, y / 2))
                } else {
                    None
                }
            }
        } else {
            if x % 2 == 1 {
                tiles[0] = if x >= 1 && y >= 1 {
                    Some(Coordinate::new(x - 1, (y - 1) / 2))
                } else {
                    None
                };
                tiles[1] = if y >= 1 && x < self.tiles.len() {
                    Some(Coordinate::new(x, (y - 1) / 2))
                } else {
                    None
                };
                tiles[2] = if y >= 3 && x < self.tiles.len() {
                    Some(Coordinate::new(x, (y - 3) / 2))
                } else {
                    None
                };
            } else {
                tiles[0] = if x >= 1 && y >= 1 {
                    Some(Coordinate::new(x - 1, (y - 1) / 2))
                } else {
                    None
                };
                tiles[1] = if x >= 1 && y >= 3 {
                    Some(Coordinate::new(x - 1, (y - 3) / 2))
                } else {
                    None
                };
                tiles[2] = if y >= 1 && x < self.tiles.len() {
                    Some(Coordinate::new(x, (y - 1) / 2))
                } else {
                    None
                };
            }
        }
        tiles
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCommon {
    pub score: usize,
    pub resources: [usize; TileKind::Max as usize],
    pub cards: [usize; DevCard::Max as usize],
    pub roads: Vec<Line>,
    pub settlement_left: usize,
    pub city_left: usize,
}

impl Default for PlayerCommon {
    fn default() -> Self {
        Self {
            score: 0,
            resources: Default::default(),
            cards: Default::default(),
            roads: Vec::new(),
            settlement_left: 5,
            city_left: 4,
        }
    }
}

impl PlayerCommon {
    pub fn longest_road(
        &self, visited: &mut Vec<bool>, current: Option<Coordinate>,
    ) -> usize {
        let mut longest = 0;
        for i in 0..self.roads.len() {
            if visited[i] {
                continue;
            }
            visited[i] = true;
            let length = 1 + if current.is_none() {
                self.longest_road(visited, Some(self.roads[i].start))
                    .max(self.longest_road(visited, Some(self.roads[i].end)))
            } else if self.roads[i].end == current.unwrap() {
                self.longest_road(visited, Some(self.roads[i].start))
            } else if self.roads[i].start == current.unwrap() {
                self.longest_road(visited, Some(self.roads[i].end))
            } else {
                visited[i] = false;
                continue;
            };

            if length > longest {
                longest = length;
            }
            visited[i] = false;
        }
        longest
    }

    pub fn get_longest_road(&self) -> usize {
        let mut visited = vec![false; self.roads.len()];
        self.longest_road(&mut visited, None)
    }

    pub fn have_roads_to(&self, to: Coordinate) -> bool {
        self.roads.iter().any(|r| r.start == to || r.end == to)
    }

    pub fn can_build_road(&self) -> bool {
        self.resources[TileKind::Brick as usize] >= 1
            && self.resources[TileKind::Wood as usize] >= 1
            && self.roads.len() < 15
    }

    pub fn can_build_settlement(&self) -> bool {
        self.resources[TileKind::Brick as usize] >= 1
            && self.resources[TileKind::Grain as usize] >= 1
            && self.resources[TileKind::Wool as usize] >= 1
            && self.resources[TileKind::Wood as usize] >= 1
            && self.settlement_left > 0
    }

    pub fn can_build_city(&self) -> bool {
        self.resources[TileKind::Stone as usize] >= 3
            && self.resources[TileKind::Grain as usize] >= 2
            && self.city_left > 0
    }

    pub fn can_buy_development_card(&self) -> bool {
        self.resources[TileKind::Grain as usize] >= 1
            && self.resources[TileKind::Wool as usize] >= 1
            && self.resources[TileKind::Stone as usize] >= 1
    }

    pub fn can_use_development_card(&self) -> bool {
        self.cards.iter().any(|r| *r > 0)
    }

    pub fn can_trade(&self) -> bool {
        self.resources.iter().any(|&r| r > 0)
    }

    pub fn resources_count(&self) -> usize {
        self.resources.iter().sum()
    }

    pub fn card_count(&self) -> usize {
        self.cards.iter().sum()
    }

    pub fn add_card(&mut self, card: Option<DevCard>) {
        match card {
            Some(card) => self.cards[card as usize] += 1,
            None => self.cards[0] += 1,
        }
    }

    pub fn remove_card(&mut self, card: Option<DevCard>) {
        match card {
            Some(card) => self.cards[card as usize] -= 1,
            None => self.cards[0] -= 1,
        }
    }

    pub fn add_road(&mut self, road: Line) {
        self.roads.push(road);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeTarget {
    Player,
    Bank,
    Harbor(TileKind),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct _TradeRequest {
    pub from: Vec<(TileKind, usize)>,
    pub to: Vec<(TileKind, usize)>,
    pub target: TradeTarget,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TradeRequest {
    inner: Arc<_TradeRequest>,
}

impl TradeRequest {
    pub fn new(
        from: Vec<(TileKind, usize)>, to: Vec<(TileKind, usize)>, target: TradeTarget,
    ) -> Self {
        Self {
            inner: Arc::new(_TradeRequest { from, to, target }),
        }
    }

    pub fn from(&self) -> &Vec<(TileKind, usize)> {
        &self.inner.from
    }

    pub fn to(&self) -> &Vec<(TileKind, usize)> {
        &self.inner.to
    }

    pub fn target(&self) -> &TradeTarget {
        &self.inner.target
    }
}

impl Serialize for TradeRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TradeRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            inner: Arc::new(_TradeRequest::deserialize(deserializer)?),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeResponse {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRoad {
    pub player: usize,
    pub road: Line,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildSettlement {
    pub player: usize,
    pub point: Coordinate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildCity {
    pub player: usize,
    pub point: Coordinate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuyDevelopmentCard {
    pub player: usize,
    pub card: Option<DevCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentCard {
    Knight(SelectRobber),
    VictoryPoint,
    RoadBuilding([Line; 2]),
    Monopoly(TileKind),
    YearOfPlenty(TileKind, TileKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseDevelopmentCard {
    pub player: usize,
    pub usage: DevelopmentCard,
    pub card: DevCard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub from: usize,
    pub to: usize,
    pub request: TradeRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectRobber {
    pub player: usize,
    pub target: Option<usize>,
    pub coord: Coordinate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfferResources {
    pub player: usize,
    pub count: usize,
    pub kind: TileKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAct {
    BuildRoad(Coordinate, Coordinate),
    BuildSettlement(Coordinate),
    BuildCity(Coordinate),
    BuyDevelopmentCard,
    UseDevelopmentCard((DevCard, DevelopmentCard)),
    Monopoly(TileKind),
    YearOfPlenty(TileKind, TileKind),
    TradeRequest(TradeRequest),
    TradeResponse(TradeResponse),
    TradeConfirm(usize),
    SelectRobber((Option<usize>, Coordinate)),
    StealResource(usize),
    EndTurn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameStart {
    pub tile: Vec<Vec<Tile>>,
    pub harbor: Vec<(Line, TileKind)>,
    pub robber: Coordinate,
    pub dice_map: HashMap<usize, Vec<Coordinate>>,
    pub players: Vec<PlayerCommon>,
    pub you: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMsg {
    GameStart(GameStart),
    PlayerInit(usize),
    PlayerTurn(usize),
    PlayerRollDice(usize),
    PlayerBuildRoad(BuildRoad),
    PlayerBuildSettlement(BuildSettlement),
    PlayerBuildCity(BuildCity),
    PlayerBuyDevelopmentCard(BuyDevelopmentCard),
    PlayerUseDevelopmentCard(UseDevelopmentCard),
    PlayerSelectRobber(SelectRobber),
    PlayerTradeRequest((usize, TradeRequest)),
    PlayerTradeResponse((usize, TradeResponse)),
    PlayerTradeConfirm((usize, TradeResponse)),
    PlayerTrade(Trade),
    PlayerOfferResources(OfferResources),
    PlayerEndTurn(usize),
}
