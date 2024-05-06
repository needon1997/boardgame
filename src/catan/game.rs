use std::{
    borrow::Borrow, collections::HashMap, f32::consts::E, hash::Hash, sync::Arc, vec,
};

use crate::common::{
    element::{Coordinate, Line},
    player::{GamePlayer, GamePlayerAction, GamePlayerMessage},
};

use super::{
    data::{CatanData, CatanDataSetup},
    element::{DevCard, Point, Tile, TileKind},
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TradeTarget {
    Player,
    Bank,
    Harbor(TileKind),
}

#[derive(Debug, PartialEq, Eq)]
struct TradeRequest {
    from: Vec<(TileKind, usize)>,
    to: Vec<(TileKind, usize)>,
    target: TradeTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TradeResponse {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BuildRoad {
    pub(super) player: usize,
    pub(super) road: Line,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BuildSettlement {
    pub(super) player: usize,
    pub(super) point: Coordinate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BuildCity {
    pub(super) player: usize,
    pub(super) point: Coordinate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BuyDevelopmentCard {
    pub(super) player: usize,
    pub(super) card: Option<DevCard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DevelopmentCard {
    Knight(SelectRobber),
    VictoryPoint,
    RoadBuilding([Line; 2]),
    Monopoly(TileKind),
    YearOfPlenty(TileKind, TileKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UseDevelopmentCard {
    pub(super) player: usize,
    pub(super) usage: DevelopmentCard,
    pub(super) card: DevCard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Trade {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) request: Arc<TradeRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SelectRobber {
    pub(super) player: usize,
    pub(super) target: Option<usize>,
    pub(super) coord: Coordinate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OfferResources {
    pub(super) player: usize,
    pub(super) count: usize,
    pub(super) kind: TileKind,
}

pub enum PlayerActionCommand {
    BuildRoad(Coordinate, Coordinate),
    BuildSettlement(Coordinate),
    BuildCity(Coordinate),
    BuyDevelopmentCard,
    UseDevelopmentCard((DevCard, DevelopmentCard)),
    Monopoly(TileKind),
    YearOfPlenty(TileKind, TileKind),
    TradeRequest(TradeRequest),
    TradeResponse(TradeResponse),
    TradeConfirm((TradeResponse, usize)),
    SelectRobber((Option<usize>, Coordinate)),
    StealResource(usize),
    EndTurn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameMessage {
    GameStart,
    PlayerInit(usize),
    PlayerTurn(usize),
    PlayerRollDice(usize),
    PlayerBuildRoad(BuildRoad),
    PlayerBuildSettlement(BuildSettlement),
    PlayerBuildCity(BuildCity),
    PlayerBuyDevelopmentCard(BuyDevelopmentCard),
    PlayerUseDevelopmentCard(UseDevelopmentCard),
    PlayerStealable(Vec<usize>),
    PlayerSelectRobber(SelectRobber),
    PlayerTradeRequest((usize, Arc<TradeRequest>)),
    PlayerTradeResponse((usize, TradeResponse)),
    PlayerTradeConfirm((usize, TradeResponse)),
    PlayerTrade(Trade),
    PlayerEndTurn(usize),
}

pub enum GameUpdate {
    HitDice(usize),
    OfferResources(OfferResources),
    BuildRoad(BuildRoad),
    BuildSettlement(BuildSettlement),
    BuildCity(BuildCity),
    BuyDevelopmentCard(BuyDevelopmentCard),
    UseDevelopmentCard(UseDevelopmentCard),
    Trade(Trade),
    SelectRobber(SelectRobber),
}

pub(super) struct Player<P> {
    pub(super) inner: P,
    pub(super) score: usize,
    pub(super) resources: [usize; TileKind::Max as usize],
    pub(super) cards: [usize; DevCard::YearOfPlenty as usize + 1],
    pub(super) roads: Vec<Line>,
    pub(super) knight_count: usize,
    pub(super) message: Vec<GameMessage>,
    settlement_left: usize,
    city_left: usize,
}

impl<P> Player<P>
where
    P: GamePlayer,
{
    fn new(inner: P) -> Self {
        Self {
            inner,
            score: 0,
            resources: Default::default(),
            cards: Default::default(),
            roads: Vec::new(),
            knight_count: 0,
            message: Vec::new(),
            settlement_left: 5,
            city_left: 4,
        }
    }

    fn name(&self) -> String {
        self.inner.get_name()
    }

    async fn get_action(&mut self) -> PlayerActionCommand {
        if let GamePlayerAction::Catan(action) = self.inner.get_action().await {
            action
        } else {
            panic!("Invalid action")
        }
    }

    async fn send_message(&mut self, message: GameMessage) {
        self.inner
            .send_message(GamePlayerMessage::Catan(message))
            .await;
    }

    fn longest_road(
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

    pub(super) fn get_longest_road(&self) -> usize {
        let mut visited = vec![false; self.roads.len()];
        self.longest_road(&mut visited, None)
    }

    fn have_roads_to(&self, to: Coordinate) -> bool {
        self.roads.iter().any(|r| r.start == to || r.end == to)
    }
}

pub struct Catan<P> {
    pub(super) dics_map: HashMap<usize, Arc<Vec<Coordinate>>>,
    pub(super) dev_cards: Vec<DevCard>,
    pub(super) roads: HashMap<Line, usize>,
    pub(super) harbors: Vec<(Line, TileKind)>,
    pub(super) tiles: Vec<Vec<Tile>>,
    pub(super) points: Vec<Vec<Point>>,
    pub(super) robber: Coordinate,
    pub(super) players: Vec<Player<P>>,
    pub(super) is_initialized: bool,
    pub(super) longest_road: Option<(usize, usize)>,
    pub(super) most_knights: Option<(usize, usize)>,
    pub(super) broadcast: Vec<GameMessage>,
    current_player: usize,
    win_score: usize,
}

impl<P> Catan<P>
where
    P: GamePlayer,
{
    pub(super) fn new(players: Vec<P>, setup: CatanDataSetup) -> Self {
        let data = CatanData::new(setup);
        Self {
            dics_map: data
                .dics_map
                .into_iter()
                .map(|(k, v)| (k, Arc::new(v)))
                .collect(),
            dev_cards: data.dev_cards,
            harbors: data.harbors,
            tiles: data.tiles,
            points: data.points,
            robber: data.robber,
            win_score: data.winscore as usize,
            roads: HashMap::new(),
            players: players.into_iter().map(|p| Player::new(p)).collect(),
            current_player: 0,
            is_initialized: false,
            longest_road: None,
            most_knights: None,
            broadcast: Vec::new(),
        }
    }

    async fn broadcast(&mut self, msg: GameMessage) {
        for player in &mut self.players {
            player.send_message(msg.clone()).await;
        }
    }

    fn get_tile(&self, x: usize, y: usize) -> Tile {
        self.tiles[x][y]
    }

    fn tile_get_points(&self, x: usize, y: usize) -> [Coordinate; 6] {
        let mut points = [Default::default(); 6];

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

    fn point_get_points(&self, x: usize, y: usize) -> [Option<Coordinate>; 3] {
        let mut points = [Default::default(); 3];

        if x % 2 == 0 && y % 2 == 0 {
            points[0] = if y >= 1 {
                Some(Coordinate::new(x, y - 1))
            } else {
                None
            };
            points[1] = if y < self.tiles[0].len() {
                Some(Coordinate::new(x, y + 1))
            } else {
                None
            };
            points[2] = if x < self.tiles.len() {
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
            points[1] = if y < self.tiles[0].len() {
                Some(Coordinate::new(x, y + 1))
            } else {
                None
            };
            points[2] = if x < self.tiles.len() {
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
            points[1] = if y < self.tiles[0].len() {
                Some(Coordinate::new(x, y + 1))
            } else {
                None
            };
            points[2] = if x < self.tiles.len() {
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
            points[1] = if y < self.tiles[0].len() {
                Some(Coordinate::new(x, y + 1))
            } else {
                None
            };
            points[2] = if x < self.tiles.len() {
                Some(Coordinate::new(x + 1, y))
            } else {
                None
            };
        }
        points
    }

    fn point_valid(&self, point: Coordinate) -> bool {
        for p in self.ponint_get_tile(point.x, point.y) {
            if let Some(p) = p {
                if !self.get_tile(p.x, p.y).is_empty() {
                    return true;
                }
            }
        }
        return false;
    }

    fn ponint_get_tile(&self, x: usize, y: usize) -> [Option<Coordinate>; 3] {
        let mut tiles = [None; 3];

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
                tiles[2] = if y >= 2 {
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
                tiles[1] = if y >= 2 {
                    Some(Coordinate::new(x, (y - 2) / 2))
                } else {
                    None
                };
                tiles[2] = if y / 2 < self.tiles[0].len() {
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
                tiles[1] = if y >= 1 {
                    Some(Coordinate::new(x, (y - 1) / 2))
                } else {
                    None
                };
                tiles[2] = if y >= 3 {
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
                tiles[2] = if y >= 1 {
                    Some(Coordinate::new(x, (y - 1) / 2))
                } else {
                    None
                };
            }
        }
        tiles
    }

    fn build_road(&mut self, build: BuildRoad) -> Result<(), String> {
        println!(
            "{} built a road from {:?}",
            self.players[build.player].name(),
            build.road
        );

        if !self.point_valid(build.road.start) || !self.point_valid(build.road.end) {
            return Err("Invalid road position".to_owned());
        }

        if self.players[build.player].roads.len() >= 15 {
            return Err("Road limit reached".to_owned());
        }

        if self.is_initialized {
            if self.players[build.player].resources[TileKind::Brick as usize] < 1
                || self.players[build.player].resources[TileKind::Wood as usize] < 1
            {
                return Err("Not enough resources".to_owned());
            } else {
                self.players[build.player].resources[TileKind::Brick as usize] -= 1;
                self.players[build.player].resources[TileKind::Wood as usize] -= 1;
            }
        }

        if self.is_initialized
            && !self.players[build.player].have_roads_to(build.road.start)
            && !self.players[build.player].have_roads_to(build.road.end)
        {
            return Err(format!("Player have no road to {:?}", build.road));
        }

        match {
            self.players[build.player].roads.push(build.road);
            self.roads.insert(build.road, build.player)
        } {
            Some(_) => {
                return Err("Road already exists".to_owned());
            },
            None => {
                self.broadcast.push(GameMessage::PlayerBuildRoad(build));
                return Ok(());
            },
        }
    }

    fn build_settlement(&mut self, build: BuildSettlement) -> Result<(), String> {
        println!(
            "{} built a settlement at {:?}",
            self.players[build.player].name(),
            build.point
        );

        if !self.point_valid(build.point) {
            return Err("Invalid settlement position".to_owned());
        }

        if self.players[build.player].settlement_left == 0 {
            return Err("Settlement limit reached".to_owned());
        }

        match self.points[build.point.x][build.point.y].owner() {
            Some(_) => {
                return Err("Point already owned".to_owned());
            },
            None => {
                if self
                    .point_get_points(build.point.x, build.point.y)
                    .iter()
                    .filter(|&p| {
                        if let Some(p) = p {
                            self.points[p.x][p.y].is_owned()
                        } else {
                            false
                        }
                    })
                    .count()
                    > 0
                {
                    return Err("Ajacent Point already owned".to_owned());
                }

                if self.is_initialized
                    && !self.players[build.player].have_roads_to(build.point)
                {
                    return Err(format!("Player have no road to {:?}", build.point));
                }

                if self.is_initialized {
                    if self.players[build.player].resources[TileKind::Brick as usize] < 1
                        || self.players[build.player].resources[TileKind::Grain as usize]
                            < 1
                        || self.players[build.player].resources[TileKind::Wool as usize]
                            < 1
                        || self.players[build.player].resources[TileKind::Wood as usize]
                            < 1
                    {
                        return Err("Not enough resources".to_owned());
                    } else {
                        self.players[build.player].resources[TileKind::Brick as usize] -=
                            1;
                        self.players[build.player].resources[TileKind::Grain as usize] -=
                            1;
                        self.players[build.player].resources[TileKind::Wool as usize] -=
                            1;
                        self.players[build.player].resources[TileKind::Wood as usize] -=
                            1;
                    }
                }

                self.points[build.point.x][build.point.y].set_owner(build.player);
                self.players[build.player].score += 1;
                self.players[build.player].settlement_left -= 1;
                self.broadcast
                    .push(GameMessage::PlayerBuildSettlement(build));
                Ok(())
            },
        }
    }

    fn build_city(&mut self, build: BuildCity) -> Result<(), String> {
        println!(
            "{} built a city at {:?}",
            self.players[build.player].name(),
            build.point
        );

        if !self.point_valid(build.point) {
            return Err("Invalid city position".to_owned());
        }

        if self.players[build.player].city_left == 0 {
            return Err("City limit reached".to_owned());
        }

        match self.points[build.point.x][build.point.y].owner() {
            Some(owner) => {
                if self.players[build.player].resources[TileKind::Stone as usize] < 3
                    || self.players[build.player].resources[TileKind::Grain as usize] < 2
                {
                    return Err("Not enough resources".to_owned());
                }
                if owner != build.player {
                    return Err("Point not owned by player".to_owned());
                }
                self.points[build.point.x][build.point.y].city = true;
                self.players[build.player].resources[TileKind::Stone as usize] -= 3;
                self.players[build.player].resources[TileKind::Grain as usize] -= 2;
                self.players[build.player].score += 1;
            },
            None => {
                return Err("Point not owned".to_owned());
            },
        }
        self.players[build.player].city_left -= 1;
        self.broadcast.push(GameMessage::PlayerBuildCity(build));
        Ok(())
    }

    fn buy_development_card(
        &mut self, mut buy: BuyDevelopmentCard,
    ) -> Result<(), String> {
        println!(
            "{} bought a development card",
            self.players[buy.player].name()
        );
        if self.players[buy.player].resources[TileKind::Grain as usize] < 1
            || self.players[buy.player].resources[TileKind::Wool as usize] < 1
            || self.players[buy.player].resources[TileKind::Stone as usize] < 1
        {
            return Err("Not enough resources".to_owned());
        }
        self.players[buy.player].resources[TileKind::Grain as usize] -= 1;
        self.players[buy.player].resources[TileKind::Wool as usize] -= 1;
        self.players[buy.player].resources[TileKind::Stone as usize] -= 1;
        let card = self.dev_cards.pop().unwrap();
        self.players[buy.player].cards[card as usize] += 1;
        self.broadcast
            .push(GameMessage::PlayerBuyDevelopmentCard(buy.clone()));
        buy.card = Some(card);
        self.players[buy.player]
            .message
            .push(GameMessage::PlayerBuyDevelopmentCard(buy));
        Ok(())
    }

    fn use_development_card(
        &mut self, use_card: UseDevelopmentCard,
    ) -> Result<(), String> {
        println!(
            "{} used a development card",
            self.players[use_card.player].name()
        );
        if self.players[use_card.player].cards[use_card.card as usize] == 0 {
            return Err("Card not found".to_owned());
        }
        match use_card.card {
            DevCard::Knight => {
                if let DevelopmentCard::Knight(select_robber) = use_card.usage.clone() {
                    self.update(GameUpdate::SelectRobber(select_robber))?;
                    self.players[use_card.player].knight_count += 1;

                    if self.players[use_card.player].knight_count >= 3 {
                        if let Some((player, knights)) = self.most_knights {
                            if knights < self.players[use_card.player].knight_count {
                                self.most_knights = Some((
                                    use_card.player,
                                    self.players[use_card.player].knight_count,
                                ));
                                self.players[player].score -= 2;
                                self.players[use_card.player].score += 2;
                            }
                        } else {
                            self.most_knights = Some((
                                use_card.player,
                                self.players[use_card.player].knight_count,
                            ));
                            self.players[use_card.player].score += 2;
                        }
                    }
                } else {
                    return Err("Invalid usage of knight card".to_owned());
                }
            },
            DevCard::VictoryPoint => {
                if let DevelopmentCard::VictoryPoint = use_card.usage {
                    self.players[use_card.player].score += 1;
                } else {
                    return Err("Invalid usage of victory point card".to_owned());
                }
            },
            DevCard::RoadBuilding => {
                if let DevelopmentCard::RoadBuilding(roads) = use_card.usage {
                    for road in roads {
                        self.update(GameUpdate::BuildRoad(BuildRoad {
                            player: use_card.player,
                            road,
                        }))
                        .unwrap()
                    }
                } else {
                    return Err("Invalid usage of road building card".to_owned());
                }
            },
            DevCard::Monopoly => {
                if let DevelopmentCard::Monopoly(kind) = use_card.usage {
                    let mut count = 0;
                    for i in 0..self.players.len() {
                        if i == use_card.player {
                            continue;
                        }
                        count += self.players[i].resources[kind as usize];
                        self.players[i].resources[kind as usize] = 0;
                    }
                    self.players[use_card.player].resources[kind as usize] += count;
                } else {
                    return Err("Invalid usage of monopoly card".to_owned());
                }
            },
            DevCard::YearOfPlenty => {
                if let DevelopmentCard::YearOfPlenty(kind1, kind2) = &use_card.usage {
                    self.players[use_card.player].resources[*kind1 as usize] += 1;
                    self.players[use_card.player].resources[*kind2 as usize] += 1;
                } else {
                    return Err("Invalid usage of year of plenty card".to_owned());
                }
            },
        }
        self.players[use_card.player].cards[use_card.card as usize] -= 1;
        self.broadcast
            .push(GameMessage::PlayerUseDevelopmentCard(use_card));
        Ok(())
    }

    fn check_winner(&self) -> Option<usize> {
        for i in 0..self.players.len() {
            if self.players[i].score > self.win_score {
                return Some(i);
            }
        }
        None
    }

    pub(super) fn check_longest_road(&mut self) {
        let longest_road = self.players[self.current_player].get_longest_road();
        if longest_road >= 5 {
            if let Some((player, length)) = self.longest_road {
                if length < longest_road {
                    self.longest_road = Some((self.current_player, longest_road));
                    self.players[player].score -= 2;
                    self.players[self.current_player].score += 2;
                }
            } else {
                self.longest_road = Some((self.current_player, longest_road));
                self.players[self.current_player].score += 2;
            }
        }
    }

    fn offer_resources(&mut self, offer: OfferResources) {
        let dice = offer.count;
        println!(
            "{} offered resources {:?}",
            self.players[offer.player].name(),
            offer,
        );
        self.players[offer.player].resources[offer.kind as usize] += offer.count;
    }

    fn hit_dice(&mut self, dice: usize) {
        for match_tile in self.dics_map.get(&dice).unwrap().clone().iter() {
            let tile = &mut self.tiles[match_tile.x][match_tile.y];
            if !tile.is_empty() {
                let kind = tile.kind();

                let points = self.tile_get_points(match_tile.x, match_tile.y);
                for point in points {
                    let point = &self.points[point.x][point.y];
                    if point.is_owned() {
                        self.update(GameUpdate::OfferResources(OfferResources {
                            player: point.owner().unwrap(),
                            count: if point.city { 2 } else { 1 },
                            kind,
                        }));
                    }
                }
            }
        }
    }

    fn select_robber(&mut self, select_robber: SelectRobber) -> Result<(), String> {
        println!(
            "{} moved the robber",
            self.players[select_robber.player].name()
        );

        if self.tiles[select_robber.coord.x][select_robber.coord.y].is_empty() {
            return Err("Invalid robber position".to_owned());
        }

        if let Some(target) = select_robber.target {
            let points =
                self.tile_get_points(select_robber.coord.x, select_robber.coord.y);
            for point in points {
                let point = &self.points[point.x][point.y];
                if point.is_owned() {
                    if point.owner().unwrap() == target {
                        let mut available = Vec::new();
                        for i in 0..self.players[target].resources.len() {
                            if self.players[target].resources[i] > 0 {
                                available.push(i);
                            }
                        }
                        if !available.is_empty() {
                            let kind =
                                available[rand::random::<usize>() % available.len()];
                            self.players[target].resources[kind] -= 1;
                            self.players[select_robber.player].resources[kind] += 1;
                            println!(
                                "{} stole a {:?} from {}",
                                self.players[select_robber.player].name(),
                                kind,
                                self.players[target].name()
                            );
                        } else {
                            return Err("No resource to steal".to_owned());
                        }
                        self.robber = select_robber.coord;
                        self.broadcast
                            .push(GameMessage::PlayerSelectRobber(select_robber));
                        return Ok(());
                    }
                }
            }
            return Err("Invalid steal target".to_owned());
        } else {
            self.robber = select_robber.coord;
            self.broadcast
                .push(GameMessage::PlayerSelectRobber(select_robber));
            Ok(())
        }
    }

    fn do_player_trade(&mut self, trade: Trade) -> Result<(), String> {
        for (kind, count) in &trade.request.from {
            if self.players[trade.from].resources[*kind as usize] < *count {
                return Err(format!(
                    "{} Not enough resources",
                    self.players[trade.from].name()
                ));
            }
        }

        for (kind, count) in &trade.request.to {
            if self.players[trade.to].resources[*kind as usize] < *count {
                return Err(format!(
                    "{} Not enough resources",
                    self.players[trade.to].name()
                ));
            }
        }

        println!(
            "{} trade with {}",
            self.players[trade.from].name(),
            self.players[trade.to].name()
        );
        for (kind, count) in &trade.request.from {
            self.players[trade.from].resources[*kind as usize] -= count;
            self.players[trade.to].resources[*kind as usize] += count;
        }

        for (kind, count) in &trade.request.to {
            self.players[trade.from].resources[*kind as usize] += count;
            self.players[trade.to].resources[*kind as usize] -= count;
        }
        Ok(())
    }

    fn do_local_trade(&mut self, trade: Trade) -> Result<(), String> {
        let mut valid_count = 0;
        let mut request_count = 0;
        let player = self.players.get_mut(trade.from).unwrap();

        match trade.request.target {
            TradeTarget::Player => {
                unreachable!("Player to player trade not reachable")
            },
            TradeTarget::Bank => {
                for (kind, count) in &trade.request.from {
                    if *count < 4 {
                        return Err(format!("Not enough {:?} to trade with bank", kind));
                    }
                    if *count % 4 != 0 {
                        return Err("Trade count must be a multiple of 4".to_owned());
                    }

                    if player.resources[*kind as usize] < *count {
                        return Err("Not enough resources".to_owned());
                    }

                    valid_count += count / 4;
                }
            },
            TradeTarget::Harbor(harbor_kind) => {
                let mut harbor = None;
                for (line, kind) in self.harbors.iter() {
                    if self.points[line.start.x][line.end.y].owner() != Some(trade.from)
                        && self.points[line.end.x][line.end.y].owner() != Some(trade.from)
                    {
                        continue;
                    } else {
                        harbor = Some(*kind);
                        break;
                    }
                }

                if harbor.is_none() {
                    return Err("No harbor owned".to_owned());
                }

                match harbor_kind {
                    TileKind::Dessert => {
                        panic!("Invalid trade with dessert harbour");
                    },
                    TileKind::Empty => {
                        for (kind, count) in &trade.request.from {
                            if *count < 3 {
                                return Err(format!(
                                    "Not enough {:?} to trade with {:?} harbour",
                                    kind, harbor_kind
                                ));
                            }
                            if *count % 3 != 0 {
                                return Err(
                                    "Trade count must be a multiple of 3".to_owned()
                                );
                            }

                            if player.resources[*kind as usize] < *count {
                                return Err("Not enough resources".to_owned());
                            }

                            valid_count += count / 3;
                        }
                    },
                    _ => {
                        for (kind, count) in &trade.request.from {
                            if *kind != harbor_kind {
                                return Err("Invalid trade with harbour".to_owned());
                            }
                            if *count % 2 != 0 {
                                return Err(
                                    "Trade count must be a multiple of 2".to_owned()
                                );
                            }

                            if player.resources[*kind as usize] < *count {
                                return Err("Not enough resources".to_owned());
                            }

                            valid_count += count / 2;
                        }
                    },
                }
            },
        };
        for (_, count) in &trade.request.to {
            request_count += count;
        }

        if valid_count != request_count {
            return Err("Invalid trade request".to_owned());
        }

        for (kind, count) in &trade.request.from {
            player.resources[*kind as usize] -= count;
        }

        for (kind, count) in &trade.request.to {
            player.resources[*kind as usize] += count;
        }
        Ok(())
    }

    fn do_trade(&mut self, trade: Trade) -> Result<(), String> {
        match trade.request.target {
            TradeTarget::Player => {
                self.do_player_trade(trade.clone())?;
            },
            _ => {
                self.do_local_trade(trade.clone())?;
            },
        }
        self.broadcast.push(GameMessage::PlayerTrade(trade));
        Ok(())
    }

    pub(super) fn update(&mut self, update: GameUpdate) -> Result<(), String> {
        match update {
            GameUpdate::BuildRoad(build) => {
                self.build_road(build)?;
            },
            GameUpdate::BuildSettlement(build) => {
                self.build_settlement(build)?;
            },
            GameUpdate::BuildCity(build) => {
                self.build_city(build)?;
            },
            GameUpdate::BuyDevelopmentCard(buy) => {
                self.buy_development_card(buy)?;
            },
            GameUpdate::UseDevelopmentCard(use_card) => {
                self.use_development_card(use_card)?;
            },
            GameUpdate::Trade(trade) => {
                self.do_trade(trade)?;
            },
            GameUpdate::SelectRobber(select_robber) => {
                self.select_robber(select_robber)?;
            },
            GameUpdate::OfferResources(offer) => self.offer_resources(offer),
            GameUpdate::HitDice(dice) => self.hit_dice(dice),
        }
        Ok(())
    }

    async fn roll_dice(&mut self) {
        let dice = 1 + rand::random::<usize>() % 6 + 1 + rand::random::<usize>() % 6;
        self.broadcast(GameMessage::PlayerRollDice(dice)).await;
        if dice == 7 {
            match self.players[self.current_player].get_action().await {
                PlayerActionCommand::SelectRobber((target, coord)) => {
                    self.update(GameUpdate::SelectRobber(SelectRobber {
                        player: self.current_player,
                        target,
                        coord,
                    }))
                    .unwrap();
                },
                _ => {
                    panic!("Invalid action")
                },
            }
        } else {
            self.update(GameUpdate::HitDice(dice)).unwrap();
        }
    }

    async fn player_action(&mut self) {
        let mut development_card_used = false;
        let mut trade_request_count = 0;
        loop {
            let action = self.players[self.current_player].get_action().await;
            match action {
                PlayerActionCommand::BuildRoad(from, to) => {
                    let road = if from.x == to.x {
                        Line::new(from, to)
                    } else {
                        Line::new(to, from)
                    };
                    self.update(GameUpdate::BuildRoad(BuildRoad {
                        player: self.current_player,
                        road,
                    }));
                },
                PlayerActionCommand::BuildSettlement(coord) => {
                    self.update(GameUpdate::BuildSettlement(BuildSettlement {
                        player: self.current_player,
                        point: coord,
                    }));
                },
                PlayerActionCommand::BuildCity(coord) => {
                    self.update(GameUpdate::BuildCity(BuildCity {
                        player: self.current_player,
                        point: coord,
                    }));
                },
                PlayerActionCommand::BuyDevelopmentCard => {
                    self.update(GameUpdate::BuyDevelopmentCard(BuyDevelopmentCard {
                        player: self.current_player,
                        card: None,
                    }))
                    .unwrap();
                },
                PlayerActionCommand::UseDevelopmentCard((dev_card, usage)) => {
                    if development_card_used {
                        panic!("Only one development card can be used per turn")
                    } else {
                        self.update(GameUpdate::UseDevelopmentCard(UseDevelopmentCard {
                            player: self.current_player,
                            usage,
                            card: dev_card,
                        }))
                        .unwrap();
                        development_card_used = true;
                    }
                },
                PlayerActionCommand::TradeRequest(trade_request) => {
                    let mut to = 0;
                    let trade_request = Arc::new(trade_request);

                    if trade_request_count >= 3 {
                        panic!("Only 3 trade requests allowed per turn")
                    } else {
                        trade_request_count += 1;

                        if trade_request.target == TradeTarget::Player {
                            self.broadcast(GameMessage::PlayerTradeRequest((
                                self.current_player,
                                trade_request.clone(),
                            )))
                            .await;

                            let mut responses =
                                vec![TradeResponse::Reject; self.players.len()];
                            for i in 0..self.players.len() {
                                if i == self.current_player {
                                    continue;
                                }
                                let player = &mut self.players[i];
                                match player.get_action().await {
                                    PlayerActionCommand::TradeResponse(resp) => {
                                        match resp {
                                            TradeResponse::Accept => {
                                                for (kind, count) in &trade_request.to {
                                                    if self.players[i].resources
                                                        [*kind as usize]
                                                        < *count
                                                    {
                                                        panic!("Not enough resources");
                                                    }
                                                }
                                            },
                                            _ => {},
                                        }
                                        responses[i] = resp;
                                        self.broadcast(GameMessage::PlayerTradeResponse(
                                            (i, resp),
                                        ))
                                        .await;
                                    },
                                    _ => {
                                        unreachable!("Invalid trade response")
                                    },
                                }
                            }

                            let action = self
                                .players
                                .get_mut(self.current_player)
                                .unwrap()
                                .get_action()
                                .await;
                            match action {
                                PlayerActionCommand::TradeConfirm((resp, i)) => {
                                    match resp {
                                        TradeResponse::Accept => {
                                            if i == self.current_player
                                                || i > self.players.len()
                                            {
                                                panic!("Invalid player index");
                                            }
                                            if responses[i] == TradeResponse::Accept {
                                            } else {
                                                panic!("Trade rejected by other player");
                                            }
                                            to = i;
                                        },
                                        TradeResponse::Reject => {
                                            println!(
                                                "{} rejected trade",
                                                self.players[self.current_player].name()
                                            );
                                        },
                                    }
                                },
                                _ => {
                                    panic!("Invalid trade response");
                                },
                            }
                        }
                        self.update(GameUpdate::Trade(Trade {
                            from: self.current_player,
                            to,
                            request: trade_request.clone(),
                        }))
                        .unwrap();
                    }
                },
                PlayerActionCommand::EndTurn => {
                    println!(
                        "{} ended their turn",
                        self.players[self.current_player].name()
                    );
                    self.broadcast(GameMessage::PlayerEndTurn(self.current_player))
                        .await;
                    break;
                },
                _ => {
                    unreachable!("Invalid action")
                },
            }
            let msgs = self.broadcast.drain(..).collect::<Vec<_>>();
            for msg in msgs {
                self.broadcast(msg).await;
            }

            let msgs = self.players[self.current_player]
                .message
                .drain(..)
                .collect::<Vec<_>>();
            for msg in msgs {
                self.players[self.current_player].send_message(msg).await;
            }
        }
    }

    async fn initialize(&mut self) {
        self.broadcast(GameMessage::GameStart).await;
        for i in (0..self.players.len()).chain((0..self.players.len()).rev()) {
            self.broadcast(GameMessage::PlayerInit(i)).await;
            let action = self.players[i].get_action().await;
            match action {
                PlayerActionCommand::BuildSettlement(coord) => {
                    self.update(GameUpdate::BuildSettlement(BuildSettlement {
                        player: i,
                        point: coord,
                    }));
                },
                _ => {
                    panic!("Invalid action")
                },
            }

            let action = self.players[i].get_action().await;
            match action {
                PlayerActionCommand::BuildRoad(from, to) => {
                    let road = if from.x == to.x {
                        Line::new(from, to)
                    } else {
                        Line::new(to, from)
                    };
                    self.update(GameUpdate::BuildRoad(BuildRoad { player: i, road }));
                },
                _ => {
                    panic!("Invalid action")
                },
            }
        }
        self.is_initialized = true;
    }

    async fn run(&mut self) {
        self.initialize().await;
        loop {
            self.broadcast(GameMessage::PlayerTurn(self.current_player))
                .await;
            self.roll_dice().await;
            self.player_action().await;
            self.check_longest_road();
            if let Some(player) = self.check_winner() {
                println!("{} won", self.players[player].name());
                break;
            }
            self.current_player = (self.current_player + 1) % self.players.len();
        }
    }
}

pub struct CatanGame {}

impl CatanGame {
    pub async fn run<P>(players: Vec<P>, setup: CatanDataSetup)
    where
        P: GamePlayer,
    {
        let mut game = Catan::new(players, setup);
        game.run().await;
    }
}
