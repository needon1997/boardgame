#[cfg(test)]
mod tests {

    use crate::{
        catan::{
            self,
            data::CatanDataSetup,
            element::{DevCard, TileKind},
            game::{
                BuildCity, BuildRoad, BuildSettlement, BuyDevelopmentCard, Catan,
                DevelopmentCard, GameMessage, GameUpdate, UseDevelopmentCard,
            },
        },
        common::{
            element::{Coordinate, Line},
            player::{GamePlayer, GamePlayerAction, GamePlayerMessage},
        },
    };

    struct TestPlayer {
        name: String,
    }

    impl TestPlayer {
        fn new(name: String) -> Self {
            Self { name }
        }
    }

    impl GamePlayer for TestPlayer {
        fn get_name(&self) -> String {
            self.name.clone()
        }

        async fn get_action(&mut self) -> GamePlayerAction {
            unreachable!()
        }

        async fn send_message(&mut self, _: GamePlayerMessage) {
            unreachable!()
        }
    }

    fn pop_msg_and_assert<P>(game: &mut Catan<P>, expected: GameMessage) {
        let msg = game.broadcast.pop().unwrap();

        println!("{:?}", msg);
        assert_eq!(msg, expected);
    }

    fn pop_player_msg_and_assert<P>(
        game: &mut Catan<P>, i: usize, expected: GameMessage,
    ) {
        let msg = game.players[i].message.pop().unwrap();

        println!("{:?}", msg);
        assert_eq!(msg, expected);
    }

    #[tokio::test]
    async fn test_game() {
        let player1 = TestPlayer::new("Player1".to_string());
        let player2 = TestPlayer::new("Player2".to_string());

        let mut game = Catan::new(vec![player1, player2], CatanDataSetup::Basic);

        let build = BuildSettlement {
            player: 0,
            point: Coordinate::new(1, 1),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildSettlement(build.clone()));

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 1),
                end: Coordinate::new(1, 2),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));

        let build = BuildSettlement {
            player: 1,
            point: Coordinate::new(1, 1),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .expect_err("duplicate build");

        let build = BuildRoad {
            player: 1,
            road: Line {
                start: Coordinate::new(1, 1),
                end: Coordinate::new(1, 2),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone()))
            .expect_err("duplicate build");

        let build = BuildSettlement {
            player: 1,
            point: Coordinate::new(1, 2),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .expect_err("impossible build");

        let build = BuildSettlement {
            player: 1,
            point: Coordinate::new(1, 3),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildSettlement(build.clone()));

        let build = BuildRoad {
            player: 1,
            road: Line {
                start: Coordinate::new(1, 3),
                end: Coordinate::new(2, 3),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));

        let build = BuildSettlement {
            player: 0,
            point: Coordinate::new(1, 5),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildSettlement(build.clone()));

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 5),
                end: Coordinate::new(1, 6),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));

        let build = BuildSettlement {
            player: 1,
            point: Coordinate::new(3, 5),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildSettlement(build.clone()));

        let build = BuildRoad {
            player: 1,
            road: Line {
                start: Coordinate::new(3, 5),
                end: Coordinate::new(3, 6),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));

        game.is_initialized = true;

        let build = BuildSettlement {
            player: 0,
            point: Coordinate::new(1, 7),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .expect_err("no road");

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 6),
                end: Coordinate::new(1, 7),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone()))
            .expect_err("no resource");
        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 6),
                end: Coordinate::new(1, 7),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));

        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;
        game.players[0].resources[TileKind::Wool as usize] += 1;
        game.players[0].resources[TileKind::Grain as usize] += 1;

        let build = BuildSettlement {
            player: 0,
            point: Coordinate::new(1, 7),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildSettlement(build.clone()));
        assert_eq!(game.players[0].score, 3);
        assert_eq!(game.players[0].get_longest_road(), 2);
        assert_eq!(game.players[1].score, 2);

        let build = BuyDevelopmentCard {
            player: 0,
            card: None,
        };
        game.update(GameUpdate::BuyDevelopmentCard(build.clone()))
            .expect_err("no resource");

        game.players[0].resources[TileKind::Stone as usize] += 1;
        game.players[0].resources[TileKind::Wool as usize] += 1;
        game.players[0].resources[TileKind::Grain as usize] += 1;
        game.update(GameUpdate::BuyDevelopmentCard(build.clone()))
            .unwrap();
        pop_msg_and_assert(
            &mut game,
            GameMessage::PlayerBuyDevelopmentCard(build.clone()),
        );
        let card = BuyDevelopmentCard {
            player: 0,
            card: Some({
                let mut card = DevCard::Knight;
                for i in 0..game.players[0].cards.len() {
                    if game.players[0].cards[i] > 0 {
                        card = DevCard::try_from(i as u8).unwrap();
                    }
                }
                card
            }),
        };
        pop_player_msg_and_assert(
            &mut game,
            0,
            GameMessage::PlayerBuyDevelopmentCard(card.clone()),
        );

        let build = UseDevelopmentCard {
            player: 0,
            card: catan::element::DevCard::YearOfPlenty,
            usage: DevelopmentCard::YearOfPlenty(TileKind::Stone, TileKind::Stone),
        };
        if card.card.unwrap() == DevCard::YearOfPlenty {
            game.update(GameUpdate::UseDevelopmentCard(build.clone()))
                .unwrap();
            pop_msg_and_assert(
                &mut game,
                GameMessage::PlayerUseDevelopmentCard(build.clone()),
            );
        } else {
            game.update(GameUpdate::UseDevelopmentCard(build.clone()))
                .expect_err("no card");
        }

        game.players[0].resources[TileKind::Stone as usize] += 3;
        game.players[0].resources[TileKind::Grain as usize] += 2;

        let build = BuildCity {
            player: 0,
            point: Coordinate::new(1, 7),
        };
        game.update(GameUpdate::BuildCity(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildCity(build.clone()));

        game.players[0].resources[TileKind::Stone as usize] += 3;
        game.players[0].resources[TileKind::Grain as usize] += 2;

        let build = BuildCity {
            player: 0,
            point: Coordinate::new(1, 5),
        };
        game.update(GameUpdate::BuildCity(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildCity(build.clone()));

        game.players[0].resources[TileKind::Stone as usize] += 3;
        game.players[0].resources[TileKind::Grain as usize] += 2;

        let build = BuildCity {
            player: 0,
            point: Coordinate::new(1, 1),
        };
        game.update(GameUpdate::BuildCity(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildCity(build.clone()));

        assert_eq!(game.players[0].score, 6);
        assert_eq!(game.players[0].get_longest_road(), 2);
        assert_eq!(game.players[1].score, 2);

        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 6),
                end: Coordinate::new(1, 6),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));

        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 6),
                end: Coordinate::new(0, 7),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));

        assert_eq!(game.players[0].score, 6);
        assert_eq!(game.players[0].get_longest_road(), 3);
        assert_eq!(game.players[1].score, 2);

        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 7),
                end: Coordinate::new(0, 8),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));

        assert_eq!(game.players[0].score, 6);
        assert_eq!(game.players[0].get_longest_road(), 4);
        assert_eq!(game.players[1].score, 2);

        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 8),
                end: Coordinate::new(0, 9),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone()))
            .expect_err("invalid location");

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 9),
                end: Coordinate::new(0, 10),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone()))
            .expect_err("invalid location");

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 8),
                end: Coordinate::new(1, 8),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));
        assert_eq!(game.players[0].score, 6);
        assert_eq!(game.players[0].get_longest_road(), 5);
        assert_eq!(game.players[1].score, 2);

        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 7),
                end: Coordinate::new(1, 8),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));
        assert_eq!(game.players[0].score, 6);
        assert_eq!(game.players[0].get_longest_road(), 7);
        assert_eq!(game.players[1].score, 2);

        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 8),
                end: Coordinate::new(1, 9),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMessage::PlayerBuildRoad(build.clone()));
        assert_eq!(game.players[0].score, 6);
        assert_eq!(game.players[0].get_longest_road(), 7);
        assert_eq!(game.players[1].score, 2);

        game.players[0].resources[TileKind::Brick as usize] += 1;
        game.players[0].resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 9),
                end: Coordinate::new(1, 10),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone()))
            .expect_err("invalid position");

        game.check_longest_road();
        assert_eq!(game.players[0].score, 8);
        assert_eq!(game.players[0].get_longest_road(), 7);
        assert_eq!(game.players[1].score, 2);
    }
}
