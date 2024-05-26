use std::{collections::HashMap, sync::Arc, vec};

use crate::common::{
    element::{Coordinate, Line},
    player::{GamePlayer, GamePlayerAction, GamePlayerMessage},
};

use super::{data::*, element::*};

pub(super) enum GameUpdate {
    HitDice(usize),
    OfferResources(OfferResources),
    BuildRoad(BuildRoad),
    BuildSettlement(BuildSettlement),
    BuildCity(BuildCity),
    BuyDevelopmentCard(BuyDevelopmentCard),
    UseDevelopmentCard(UseDevelopmentCard),
    Trade(Option<Trade>),
    SelectRobber(SelectRobber),
}

pub(super) struct Player<P> {
    pub(super) inner: P,
    pub(super) base: PlayerCommon,
    pub(super) knight_count: usize,
    pub(super) message: Vec<GameMsg>,
}

impl<P> Player<P>
where
    P: GamePlayer,
{
    fn new(inner: P) -> Self {
        Self {
            inner,
            base: PlayerCommon::default(),
            knight_count: 0,
            message: Vec::new(),
        }
    }

    fn name(&self) -> String {
        self.inner.get_name()
    }

    async fn get_action(&mut self) -> GameAct {
        if let GamePlayerAction::Catan(action) = self.inner.get_action().await {
            action
        } else {
            panic!("Invalid action")
        }
    }

    async fn send_message(&mut self, message: GameMsg) {
        self.inner
            .send_message(GamePlayerMessage::Catan(message))
            .await;
    }
}

pub(super) struct Catan<P> {
    pub(super) inner: CatanCommon,
    pub(super) dev_cards: Vec<DevCard>,
    pub(super) players: Vec<Player<P>>,
    pub(super) is_initialized: bool,
    pub(super) longest_road: Option<(usize, usize)>,
    pub(super) most_knights: Option<(usize, usize)>,
    pub(super) broadcast: Vec<GameMsg>,
    current_player: usize,
    win_score: usize,
}

impl<P> Catan<P>
where
    P: GamePlayer,
{
    pub fn new(players: Vec<P>, setup: CatanDataSetup) -> Self {
        let data = CatanData::new(setup);
        Self {
            dev_cards: data.dev_cards,
            inner: CatanCommon::new(
                data.tiles,
                data.points,
                HashMap::new(),
                data.harbors,
                data.dics_map,
                data.robber,
            ),
            win_score: data.winscore as usize,
            players: players.into_iter().map(|p| Player::new(p)).collect(),
            current_player: 0,
            is_initialized: false,
            longest_road: None,
            most_knights: None,
            broadcast: Vec::new(),
        }
    }

    async fn broadcast(&mut self, msg: GameMsg) {
        for player in &mut self.players {
            player.send_message(msg.clone()).await;
        }
    }

    fn build_road(&mut self, build: BuildRoad) -> Result<(), String> {
        println!(
            "{} built a road from {:?}",
            self.players[build.player].name(),
            build.road
        );

        if !self.inner.point_valid(build.road.start)
            || !self.inner.point_valid(build.road.end)
        {
            return Err("Invalid road position".to_owned());
        }

        if self.players[build.player].base.roads.len() >= 15 {
            return Err("Road limit reached".to_owned());
        }

        if self.is_initialized {
            if self.players[build.player].base.resources[TileKind::Brick as usize] < 1
                || self.players[build.player].base.resources[TileKind::Wood as usize] < 1
            {
                return Err("Not enough resources".to_owned());
            } else {
                self.players[build.player].base.resources[TileKind::Brick as usize] -= 1;
                self.players[build.player].base.resources[TileKind::Wood as usize] -= 1;
            }
        }

        if self.is_initialized
            && !self.players[build.player]
                .base
                .have_roads_to(build.road.start)
            && !self.players[build.player]
                .base
                .have_roads_to(build.road.end)
        {
            return Err(format!("Player have no road to {:?}", build.road));
        }

        match {
            self.players[build.player].base.add_road(build.road);
            self.inner.add_road(build.player, build.road)
        } {
            Some(_) => {
                return Err("Road already exists".to_owned());
            },
            None => {
                self.broadcast.push(GameMsg::PlayerBuildRoad(build));
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

        if !self.inner.point_valid(build.point) {
            return Err("Invalid settlement position".to_owned());
        }

        if self.players[build.player].base.settlement_left == 0 {
            return Err("Settlement limit reached".to_owned());
        }

        match self.inner.point(build.point).owner() {
            Some(_) => {
                return Err("Point already owned".to_owned());
            },
            None => {
                if self
                    .inner
                    .point_get_points(build.point)
                    .iter()
                    .filter(|&p| {
                        if let Some(p) = p {
                            self.inner.point(*p).is_owned()
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
                    && !self.players[build.player].base.have_roads_to(build.point)
                {
                    return Err(format!("Player have no road to {:?}", build.point));
                }

                if self.is_initialized {
                    if self.players[build.player].base.resources[TileKind::Brick as usize]
                        < 1
                        || self.players[build.player].base.resources
                            [TileKind::Grain as usize]
                            < 1
                        || self.players[build.player].base.resources
                            [TileKind::Wool as usize]
                            < 1
                        || self.players[build.player].base.resources
                            [TileKind::Wood as usize]
                            < 1
                    {
                        return Err("Not enough resources".to_owned());
                    } else {
                        self.players[build.player].base.resources
                            [TileKind::Brick as usize] -= 1;
                        self.players[build.player].base.resources
                            [TileKind::Grain as usize] -= 1;
                        self.players[build.player].base.resources
                            [TileKind::Wool as usize] -= 1;
                        self.players[build.player].base.resources
                            [TileKind::Wood as usize] -= 1;
                    }
                }

                self.inner.add_settlement(build.player, build.point);
                self.players[build.player].base.score += 1;
                self.players[build.player].base.settlement_left -= 1;
                self.broadcast.push(GameMsg::PlayerBuildSettlement(build));
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

        if !self.inner.point_valid(build.point) {
            return Err("Invalid city position".to_owned());
        }

        if self.players[build.player].base.city_left == 0 {
            return Err("City limit reached".to_owned());
        }

        match self.inner.point(build.point).owner() {
            Some(owner) => {
                if self.players[build.player].base.resources[TileKind::Stone as usize] < 3
                    || self.players[build.player].base.resources[TileKind::Grain as usize]
                        < 2
                {
                    return Err("Not enough resources".to_owned());
                }
                if owner != build.player {
                    return Err("Point not owned by player".to_owned());
                }
                self.inner.add_city(build.player, build.point);
                self.players[build.player].base.resources[TileKind::Stone as usize] -= 3;
                self.players[build.player].base.resources[TileKind::Grain as usize] -= 2;
                self.players[build.player].base.score += 1;
            },
            None => {
                return Err("Point not owned".to_owned());
            },
        }
        self.players[build.player].base.city_left -= 1;
        self.players[build.player].base.settlement_left += 1;
        self.broadcast.push(GameMsg::PlayerBuildCity(build));
        Ok(())
    }

    fn buy_development_card(
        &mut self, mut buy: BuyDevelopmentCard,
    ) -> Result<(), String> {
        println!(
            "{} bought a development card",
            self.players[buy.player].name()
        );
        if self.players[buy.player].base.resources[TileKind::Grain as usize] < 1
            || self.players[buy.player].base.resources[TileKind::Wool as usize] < 1
            || self.players[buy.player].base.resources[TileKind::Stone as usize] < 1
        {
            return Err("Not enough resources".to_owned());
        }
        self.players[buy.player].base.resources[TileKind::Grain as usize] -= 1;
        self.players[buy.player].base.resources[TileKind::Wool as usize] -= 1;
        self.players[buy.player].base.resources[TileKind::Stone as usize] -= 1;
        match self.dev_cards.pop() {
            Some(card) => {
                self.players[buy.player].base.cards[card as usize] += 1;
                self.broadcast
                    .push(GameMsg::PlayerBuyDevelopmentCard(buy.clone()));
                buy.card = Some(card);
                self.players[buy.player]
                    .message
                    .push(GameMsg::PlayerBuyDevelopmentCard(buy));
            },
            None => {},
        }

        Ok(())
    }

    fn use_development_card(
        &mut self, use_card: UseDevelopmentCard,
    ) -> Result<(), String> {
        println!(
            "{} used a development card",
            self.players[use_card.player].name()
        );
        if self.players[use_card.player].base.cards[use_card.card as usize] == 0 {
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
                                self.players[player].base.score -= 2;
                                self.players[use_card.player].base.score += 2;
                            }
                        } else {
                            self.most_knights = Some((
                                use_card.player,
                                self.players[use_card.player].knight_count,
                            ));
                            self.players[use_card.player].base.score += 2;
                        }
                    }
                } else {
                    return Err("Invalid usage of knight card".to_owned());
                }
            },
            DevCard::VictoryPoint => {
                if let DevelopmentCard::VictoryPoint = use_card.usage {
                    self.players[use_card.player].base.score += 1;
                } else {
                    return Err("Invalid usage of victory point card".to_owned());
                }
            },
            DevCard::RoadBuilding => {
                if let DevelopmentCard::RoadBuilding(roads) = use_card.usage {
                    for road in roads {
                        self.players[use_card.player].base.resources
                            [TileKind::Brick as usize] += 1;
                        self.players[use_card.player].base.resources
                            [TileKind::Wood as usize] += 1;
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
                        count += self.players[i].base.resources[kind as usize];
                        self.players[i].base.resources[kind as usize] = 0;
                    }
                    self.players[use_card.player].base.resources[kind as usize] += count;
                } else {
                    return Err("Invalid usage of monopoly card".to_owned());
                }
            },
            DevCard::YearOfPlenty => {
                if let DevelopmentCard::YearOfPlenty(kind1, kind2) = &use_card.usage {
                    self.players[use_card.player].base.resources[*kind1 as usize] += 1;
                    self.players[use_card.player].base.resources[*kind2 as usize] += 1;
                } else {
                    return Err("Invalid usage of year of plenty card".to_owned());
                }
            },
            _ => {
                unreachable!("Invalid development card");
            },
        }
        self.players[use_card.player].base.cards[use_card.card as usize] -= 1;
        self.broadcast
            .push(GameMsg::PlayerUseDevelopmentCard(use_card));
        Ok(())
    }

    fn check_winner(&self) -> Option<usize> {
        for i in 0..self.players.len() {
            if self.players[i].base.score > self.win_score {
                return Some(i);
            }
        }
        None
    }

    pub fn check_longest_road(&mut self) {
        let longest_road = self.players[self.current_player].base.get_longest_road();
        if longest_road >= 5 {
            if let Some((player, length)) = self.longest_road {
                if length < longest_road {
                    self.longest_road = Some((self.current_player, longest_road));
                    self.players[player].base.score -= 2;
                    self.players[self.current_player].base.score += 2;
                }
            } else {
                self.longest_road = Some((self.current_player, longest_road));
                self.players[self.current_player].base.score += 2;
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
        self.players[offer.player].base.resources[offer.kind as usize] =
            (self.players[offer.player].base.resources[offer.kind as usize] as isize
                + offer.count)
                .min(0)
                .max(20) as usize;
        self.broadcast.push(GameMsg::PlayerOfferResources(offer));
    }

    fn hit_dice(&mut self, dice: usize) {
        println!("Dice: {}", dice);
        for match_tile in self.inner.dice_map().get(&dice).unwrap().clone().iter() {
            println!("Match Tile: {:?}", match_tile);
            let tile = &mut self.inner.tile(*match_tile);
            let kind = tile.kind();
            if !tile.is_empty() && *match_tile != self.inner.robber() {
                let points = self.inner.tile_get_points(*match_tile);
                for point in points {
                    let point = &self.inner.point(point);
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

        if self.inner.tile(select_robber.coord).is_empty() {
            return Err("Invalid robber position".to_owned());
        }

        if let Some(target) = select_robber.target {
            let points = self.inner.tile_get_points(select_robber.coord);
            for point in points {
                let point = &self.inner.point(point);
                if point.is_owned() {
                    if point.owner().unwrap() == target {
                        let mut available = Vec::new();
                        for i in 0..self.players[target].base.resources.len() {
                            if self.players[target].base.resources[i] > 0 {
                                available.push(i);
                            }
                        }
                        if !available.is_empty() {
                            let kind =
                                available[rand::random::<usize>() % available.len()];
                            println!(
                                "{} stole a {:?} from {}",
                                self.players[select_robber.player].name(),
                                kind,
                                self.players[target].name()
                            );
                            self.update(GameUpdate::OfferResources(OfferResources {
                                player: target,
                                count: -1,
                                kind: TileKind::try_from(kind as u8).unwrap(),
                            }));
                            self.update(GameUpdate::OfferResources(OfferResources {
                                player: select_robber.player,
                                count: 1,
                                kind: TileKind::try_from(kind as u8).unwrap(),
                            }));
                        } else {
                            return Err("No resource to steal".to_owned());
                        }
                        self.inner.set_robber(select_robber.coord);
                        self.broadcast
                            .push(GameMsg::PlayerSelectRobber(select_robber));
                        return Ok(());
                    }
                }
            }
            return Err("Invalid steal target".to_owned());
        } else {
            self.inner.set_robber(select_robber.coord);
            self.broadcast
                .push(GameMsg::PlayerSelectRobber(select_robber));
            Ok(())
        }
    }

    fn do_player_trade(&mut self, trade: Trade) -> Result<(), String> {
        for (kind, count) in trade.request.from() {
            if self.players[trade.from].base.resources[*kind as usize] < *count {
                return Err(format!(
                    "{} Not enough resources",
                    self.players[trade.from].name()
                ));
            }
        }

        for (kind, count) in trade.request.to() {
            if self.players[trade.to].base.resources[*kind as usize] < *count {
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
        for (kind, count) in trade.request.from() {
            self.players[trade.from].base.resources[*kind as usize] -= count;
            self.players[trade.to].base.resources[*kind as usize] += count;
        }

        for (kind, count) in trade.request.to() {
            self.players[trade.from].base.resources[*kind as usize] += count;
            self.players[trade.to].base.resources[*kind as usize] -= count;
        }
        Ok(())
    }

    fn do_local_trade(&mut self, trade: Trade) -> Result<(), String> {
        let mut valid_count = 0;
        let mut request_count = 0;
        let player = self.players.get_mut(trade.from).unwrap();

        match trade.request.target() {
            TradeTarget::Player => {
                unreachable!("Player to player trade not reachable")
            },
            TradeTarget::Bank => {
                for (kind, count) in trade.request.from() {
                    if *count < 4 {
                        return Err(format!("Not enough {:?} to trade with bank", kind));
                    }
                    if *count % 4 != 0 {
                        return Err("Trade count must be a multiple of 4".to_owned());
                    }

                    if player.base.resources[*kind as usize] < *count {
                        return Err("Not enough resources".to_owned());
                    }

                    valid_count += count / 4;
                }
            },
            TradeTarget::Harbor(harbor_kind) => {
                let mut harbor = None;
                for (line, kind) in self.inner.harbors().iter() {
                    if self.inner.point(line.start).owner() != Some(trade.from)
                        && self.inner.point(line.end).owner() != Some(trade.from)
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
                        for (kind, count) in trade.request.from() {
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

                            if player.base.resources[*kind as usize] < *count {
                                return Err("Not enough resources".to_owned());
                            }

                            valid_count += count / 3;
                        }
                    },
                    _ => {
                        for (kind, count) in trade.request.from() {
                            if *kind != *harbor_kind {
                                return Err("Invalid trade with harbour".to_owned());
                            }
                            if *count % 2 != 0 {
                                return Err(
                                    "Trade count must be a multiple of 2".to_owned()
                                );
                            }

                            if player.base.resources[*kind as usize] < *count {
                                return Err("Not enough resources".to_owned());
                            }

                            valid_count += count / 2;
                        }
                    },
                }
            },
        };
        for (_, count) in trade.request.to() {
            request_count += count;
        }

        if valid_count != request_count {
            return Err("Invalid trade request".to_owned());
        }

        for (kind, count) in trade.request.from() {
            player.base.resources[*kind as usize] -= count;
        }

        for (kind, count) in trade.request.to() {
            player.base.resources[*kind as usize] += count;
        }
        Ok(())
    }

    fn do_trade(&mut self, trade: Option<Trade>) -> Result<(), String> {
        match trade {
            Some(trade) => {
                match trade.request.target() {
                    TradeTarget::Player => {
                        self.do_player_trade(trade.clone())?;
                    },
                    _ => {
                        self.do_local_trade(trade.clone())?;
                    },
                };
                self.broadcast.push(GameMsg::PlayerTrade(Some(trade)));
            },
            None => {
                self.broadcast.push(GameMsg::PlayerTrade(None));
            },
        }
        Ok(())
    }

    pub fn update(&mut self, update: GameUpdate) -> Result<(), String> {
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
        self.broadcast(GameMsg::PlayerRollDice(dice)).await;
        if dice == 7 {
            match self.players[self.current_player].get_action().await {
                GameAct::SelectRobber((target, coord)) => {
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
        self.flush_messages().await;
    }

    async fn flush_messages(&mut self) {
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

    async fn player_action(&mut self) {
        let mut development_card_used = false;
        let mut trade_request_count = 0;
        loop {
            let action = self.players[self.current_player].get_action().await;
            match action {
                GameAct::BuildRoad(from, to) => {
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
                GameAct::BuildSettlement(coord) => {
                    self.update(GameUpdate::BuildSettlement(BuildSettlement {
                        player: self.current_player,
                        point: coord,
                    }));
                },
                GameAct::BuildCity(coord) => {
                    self.update(GameUpdate::BuildCity(BuildCity {
                        player: self.current_player,
                        point: coord,
                    }));
                },
                GameAct::BuyDevelopmentCard => {
                    self.update(GameUpdate::BuyDevelopmentCard(BuyDevelopmentCard {
                        player: self.current_player,
                        card: None,
                    }))
                    .unwrap();
                },
                GameAct::UseDevelopmentCard((dev_card, usage)) => {
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
                GameAct::TradeRequest(trade_request) => {
                    let mut to = 0;

                    if trade_request_count >= 3 {
                        panic!("Only 3 trade requests allowed per turn")
                    } else {
                        trade_request_count += 1;

                        if *trade_request.target() == TradeTarget::Player {
                            self.broadcast(GameMsg::PlayerTradeRequest((
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
                                    GameAct::TradeResponse(resp) => {
                                        match resp {
                                            TradeResponse::Accept => {
                                                for (kind, count) in trade_request.to() {
                                                    if self.players[i].base.resources
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
                                        self.broadcast(GameMsg::PlayerTradeResponse((
                                            i, resp,
                                        )))
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
                                GameAct::TradeConfirm(i) => match i {
                                    Some(i) => {
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
                                        self.update(GameUpdate::Trade(Some(Trade {
                                            from: self.current_player,
                                            to,
                                            request: trade_request.clone(),
                                        })))
                                        .unwrap();
                                    },
                                    None => {
                                        println!("Trade rejected by player");
                                        self.update(GameUpdate::Trade(None)).unwrap();
                                    },
                                },
                                _ => {
                                    panic!("Invalid trade response: {:?}", action);
                                },
                            }
                        }
                    }
                },
                GameAct::EndTurn => {
                    println!(
                        "{} ended their turn",
                        self.players[self.current_player].name()
                    );
                    self.broadcast(GameMsg::PlayerEndTurn(self.current_player))
                        .await;
                    break;
                },
                _ => {
                    println!("Invalid action {:?}", action)
                },
            }
            self.flush_messages().await;
        }
    }

    async fn initialize(&mut self) {
        for i in 0..self.players.len() {
            let msg = GameMsg::GameStart(GameStart {
                tile: self.inner.tiles().clone(),
                harbor: self.inner.harbors().clone(),
                robber: self.inner.robber(),
                dice_map: self.inner.dice_map().clone(),
                players: self.players.iter().map(|p| p.base.clone()).collect(),
                you: i,
            });
            self.players[i].send_message(msg).await;
        }

        for i in (0..self.players.len()) {
            self.broadcast(GameMsg::PlayerInit(i)).await;
            let action = self.players[i].get_action().await;
            match action {
                GameAct::BuildSettlement(coord) => {
                    self.update(GameUpdate::BuildSettlement(BuildSettlement {
                        player: i,
                        point: coord,
                    }));
                },
                _ => {
                    panic!("Invalid action")
                },
            }
            self.flush_messages().await;
            let action = self.players[i].get_action().await;
            match action {
                GameAct::BuildRoad(from, to) => {
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
            self.flush_messages().await;
        }

        for i in (0..self.players.len()).rev() {
            self.broadcast(GameMsg::PlayerInit(i)).await;
            let action = self.players[i].get_action().await;
            match action {
                GameAct::BuildSettlement(coord) => {
                    self.update(GameUpdate::BuildSettlement(BuildSettlement {
                        player: i,
                        point: coord,
                    }));
                    let tiles = self.inner.ponint_get_tile(coord);
                    for tile in tiles {
                        if let Some(tile) = tile {
                            if self.inner.tile(tile).is_resource() {
                                self.update(GameUpdate::OfferResources(OfferResources {
                                    player: i,
                                    count: 1,
                                    kind: self.inner.tile(tile).kind(),
                                }));
                            }
                        }
                    }
                },
                _ => {
                    panic!("Invalid action")
                },
            }
            self.flush_messages().await;
            let action = self.players[i].get_action().await;
            match action {
                GameAct::BuildRoad(from, to) => {
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
            self.flush_messages().await;
        }
        self.is_initialized = true;
    }

    async fn run(&mut self) {
        self.initialize().await;
        loop {
            self.broadcast(GameMsg::PlayerTurn(self.current_player))
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
