use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::element::{Coordinate, Line};

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
    Ocean,
    Gold,
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
            2 => Ok(TileKind::Ocean),
            3 => Ok(TileKind::Gold),
            4 => Ok(TileKind::Wood),
            5 => Ok(TileKind::Brick),
            6 => Ok(TileKind::Grain),
            7 => Ok(TileKind::Wool),
            8 => Ok(TileKind::Stone),
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

    pub fn is_resource_or_gold(&self) -> bool {
        self.is_resource() || *self == TileKind::Gold
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

    pub fn is_ocean(&self) -> bool {
        self.kind == TileKind::Ocean
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
    boats: HashMap<Line, usize>,
    harbors: Vec<(Line, TileKind)>,
    robber: Coordinate,
    dice_map: HashMap<usize, Vec<Coordinate>>,
}

impl CatanCommon {
    pub fn new(
        tiles: Vec<Vec<Tile>>, points: Vec<Vec<Point>>, roads: HashMap<Line, usize>,
        boats: HashMap<Line, usize>, harbors: Vec<(Line, TileKind)>,
        dice_map: HashMap<usize, Vec<Coordinate>>, robber: Coordinate,
    ) -> Self {
        Self {
            tiles,
            points,
            roads,
            boats,
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

    pub fn boats(&self) -> &HashMap<Line, usize> {
        &self.boats
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

    pub fn add_boat(&mut self, player: usize, boat: Line) -> Option<usize> {
        self.boats.insert(boat, player)
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
        return self.ponint_get_tile(point).iter().any(|&p| {
            if let Some(p) = p {
                if !self.tile(p).is_empty() {
                    return true;
                }
            }
            return false;
        });
    }

    pub fn point_valid_settlement(&self, point: Coordinate) -> bool {
        return self.ponint_get_tile(point).iter().any(|&p| {
            if let Some(p) = p {
                if !self.tile(p).is_empty() && !self.tile(p).is_ocean() {
                    return true;
                }
            }
            return false;
        });
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

    pub fn check_valid_local_trade(
        &self, trade: &Trade, player: &PlayerCommon,
    ) -> Result<(), String> {
        let mut valid_count = 0;
        let mut request_count = 0;

        if trade
            .request
            .from()
            .iter()
            .fold(0, |sum, (_, count)| sum + *count)
            == 0
        {
            return Err("No resources to trade".to_owned());
        }
        match trade.request.target() {
            TradeTarget::Player => {
                unreachable!("Player to player trade not reachable")
            },
            TradeTarget::Bank => {
                for (kind, count) in trade.request.from() {
                    if *count > 0 {
                        if *count < 4 {
                            return Err(format!(
                                "Not enough {:?} to trade with bank",
                                kind
                            ));
                        }
                        if *count % 4 != 0 {
                            return Err("Trade count must be a multiple of 4".to_owned());
                        }

                        if player.resources[*kind as usize] < *count {
                            return Err("Not enough resources".to_owned());
                        }

                        valid_count += count / 4;
                    }
                }
            },
            TradeTarget::Harbor => {
                let mut harbor = HashSet::new();
                for (line, kind) in self.harbors().iter() {
                    if self.point(line.start).owner() != Some(trade.from)
                        && self.point(line.end).owner() != Some(trade.from)
                    {
                        continue;
                    } else {
                        harbor.insert(*kind);
                        break;
                    }
                }

                if harbor.is_empty() {
                    return Err("No harbor owned".to_owned());
                }
                for (kind, count) in trade.request.from() {
                    if harbor.contains(kind) {
                        if *count % 2 != 0 {
                            return Err("Trade count must be a multiple of 2".to_owned());
                        }

                        if player.resources[*kind as usize] < *count {
                            return Err("Not enough resources".to_owned());
                        }

                        valid_count += count / 2;
                    } else if harbor.contains(&TileKind::Dessert) {
                        if *count % 3 != 0 {
                            return Err("Trade count must be a multiple of 3".to_owned());
                        }

                        if player.resources[*kind as usize] < *count {
                            return Err("Not enough resources".to_owned());
                        }

                        valid_count += count / 3;
                    } else {
                        return Err("Invalid trade with harbour".to_owned());
                    }
                }
            },
        };
        for (_, count) in trade.request.to() {
            request_count += count;
        }

        if valid_count != request_count {
            return Err("Invalid trade request".to_owned());
        }
        Ok(())
    }

    pub fn check_valid_buildable_boat(&self, boat: Line) -> bool {
        let start_tile = self.ponint_get_tile(boat.start);
        let end_tile = self.ponint_get_tile(boat.end);

        start_tile.iter().any(|&tile| {
            end_tile.iter().any(|&start_tile| {
                if let Some(tile) = tile {
                    if let Some(start_tile) = start_tile {
                        if start_tile == tile && self.tile(tile).kind() == TileKind::Ocean
                        {
                            return true;
                        }
                    }
                }
                return false;
            })
        }) && self.roads().get(&boat).is_none()
            && self.boats().get(&boat).is_none()
            && self.point_valid(boat.start)
            && self.point_valid(boat.end)
    }

    pub fn check_valid_buildable_road(&self, road: Line) -> bool {
        let start_tile = self.ponint_get_tile(road.start);
        let end_tile = self.ponint_get_tile(road.end);

        start_tile.iter().any(|&tile| {
            end_tile.iter().any(|&start_tile| {
                if let Some(tile) = tile {
                    if let Some(start_tile) = start_tile {
                        if start_tile == tile
                            && self.tile(tile).kind().is_resource_or_gold()
                        {
                            return true;
                        }
                    }
                }
                return false;
            })
        }) && self.roads().get(&road).is_none()
            && self.boats().get(&road).is_none()
            && self.point_valid_settlement(road.start)
            && self.point_valid_settlement(road.end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCommon {
    pub score: usize,
    pub resources: [usize; TileKind::Max as usize],
    pub cards: [usize; DevCard::Max as usize],
    pub roads: Vec<Line>,
    pub boats: Vec<Line>,
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
            boats: Vec::new(),
            settlement_left: 5,
            city_left: 4,
        }
    }
}

impl PlayerCommon {
    pub fn longest_path(
        &self, visited: &mut Vec<bool>, current: Option<Coordinate>,
    ) -> usize {
        let mut longest = 0;
        let mut path = self.roads.clone();
        path.append(&mut self.boats.clone());
        for i in 0..path.len() {
            if visited[i] {
                continue;
            }
            visited[i] = true;
            let length = 1 + if current.is_none() {
                self.longest_path(visited, Some(path[i].start))
                    .max(self.longest_path(visited, Some(path[i].end)))
            } else if path[i].end == current.unwrap() {
                self.longest_path(visited, Some(path[i].start))
            } else if path[i].start == current.unwrap() {
                self.longest_path(visited, Some(path[i].end))
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

    pub fn get_longest_path(&self) -> usize {
        let mut visited = vec![false; self.roads.len() + self.boats.len()];
        self.longest_path(&mut visited, None)
    }

    pub fn have_roads_to(&self, to: Coordinate) -> bool {
        self.roads.iter().any(|r| r.start == to || r.end == to)
    }

    pub fn have_boats_to(&self, to: Coordinate) -> bool {
        self.boats.iter().any(|r| r.start == to || r.end == to)
    }

    pub fn can_build_road(&self) -> bool {
        self.resources[TileKind::Brick as usize] >= 1
            && self.resources[TileKind::Wood as usize] >= 1
            && self.roads.len() < 15
    }

    pub fn can_build_boat(&self) -> bool {
        self.resources[TileKind::Wool as usize] >= 1
            && self.resources[TileKind::Wood as usize] >= 1
            && self.boats.len() < 15
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

    pub fn add_boat(&mut self, boat: Line) {
        self.boats.push(boat);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeTarget {
    Player,
    Bank,
    Harbor,
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
    pub to: Option<usize>,
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
    pub count: isize,
    pub kind: TileKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAct {
    BuildRoad(Line),
    BuildBoat(Line),
    BuildSettlement(Coordinate),
    BuildCity(Coordinate),
    BuyDevelopmentCard,
    UseDevelopmentCard((DevCard, DevelopmentCard)),
    Monopoly(TileKind),
    YearOfPlenty(TileKind, TileKind),
    TradeRequest(TradeRequest),
    TradeResponse(TradeResponse),
    TradeConfirm(Option<usize>),
    SelectRobber((Option<usize>, Coordinate)),
    DropResource(Vec<(TileKind, usize)>),
    PickResource(Vec<(TileKind, usize)>),
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
    PlayerRollDice((u8, u8)),
    PlayerBuildRoad(BuildRoad),
    PlayerBuildBoat(BuildRoad),
    PlayerBuildSettlement(BuildSettlement),
    PlayerBuildCity(BuildCity),
    PlayerBuyDevelopmentCard(BuyDevelopmentCard),
    PlayerUseDevelopmentCard(UseDevelopmentCard),
    PlayerStartSelectRobber(),
    PlayerSelectRobber(SelectRobber),
    PlayerTradeRequest((usize, TradeRequest)),
    PlayerTradeResponse((usize, TradeResponse)),
    PlayerTrade(Option<Trade>),
    PlayerOfferResources(OfferResources),
    PlayerDropResources((usize, usize)),
    PlayerPickResources((usize, usize)),
    PlayerEndTurn(usize),
}
