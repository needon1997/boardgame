#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use tokio::{
        select,
        sync::mpsc::{UnboundedReceiver, UnboundedSender},
    };

    use boardgame_common::{
        catan::element::*,
        element::{Coordinate, Line},
        network::{new_server, ClientMsg, NetworkServerEvent, ServerMsg},
        player::{GamePlayer, GamePlayerAction, GamePlayerMessage},
    };

    use crate::{
        data::CatanDataSetup,
        game::{Catan, CatanGame, GameUpdate},
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

    fn pop_msg_and_assert<P>(game: &mut Catan<P>, expected: GameMsg) {
        let msg = game.broadcast.pop().unwrap();

        println!("{:?}", msg);
        assert_eq!(msg, expected);
    }

    fn pop_player_msg_and_assert<P>(game: &mut Catan<P>, i: usize, expected: GameMsg) {
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
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildSettlement(build.clone()));

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 1),
                end: Coordinate::new(1, 2),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));

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
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildSettlement(build.clone()));

        let build = BuildRoad {
            player: 1,
            road: Line {
                start: Coordinate::new(1, 3),
                end: Coordinate::new(2, 3),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));

        let build = BuildSettlement {
            player: 0,
            point: Coordinate::new(1, 5),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildSettlement(build.clone()));

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 5),
                end: Coordinate::new(1, 6),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));

        let build = BuildSettlement {
            player: 1,
            point: Coordinate::new(3, 5),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildSettlement(build.clone()));

        let build = BuildRoad {
            player: 1,
            road: Line {
                start: Coordinate::new(3, 5),
                end: Coordinate::new(3, 6),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));

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
        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 6),
                end: Coordinate::new(1, 7),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));

        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;
        game.players[0].base.resources[TileKind::Wool as usize] += 1;
        game.players[0].base.resources[TileKind::Grain as usize] += 1;

        let build = BuildSettlement {
            player: 0,
            point: Coordinate::new(1, 7),
        };
        game.update(GameUpdate::BuildSettlement(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildSettlement(build.clone()));
        assert_eq!(game.players[0].base.score, 3);
        assert_eq!(game.players[0].base.get_longest_path(), 2);
        assert_eq!(game.players[1].base.score, 2);

        let build = BuyDevelopmentCard {
            player: 0,
            card: None,
        };
        game.update(GameUpdate::BuyDevelopmentCard(build.clone()))
            .expect_err("no resource");

        game.players[0].base.resources[TileKind::Stone as usize] += 1;
        game.players[0].base.resources[TileKind::Wool as usize] += 1;
        game.players[0].base.resources[TileKind::Grain as usize] += 1;
        game.update(GameUpdate::BuyDevelopmentCard(build.clone()))
            .unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuyDevelopmentCard(build.clone()));
        let card = BuyDevelopmentCard {
            player: 0,
            card: Some({
                let mut card = DevCard::Knight;
                for i in 0..game.players[0].base.cards.len() {
                    if game.players[0].base.cards[i] > 0 {
                        card = DevCard::try_from(i as u8).unwrap();
                    }
                }
                card
            }),
        };
        pop_player_msg_and_assert(
            &mut game,
            0,
            GameMsg::PlayerBuyDevelopmentCard(card.clone()),
        );

        let build = UseDevelopmentCard {
            player: 0,
            card: DevCard::YearOfPlenty,
            usage: DevelopmentCard::YearOfPlenty(TileKind::Stone, TileKind::Stone),
        };
        if card.card.unwrap() == DevCard::YearOfPlenty {
            game.update(GameUpdate::UseDevelopmentCard(build.clone()))
                .unwrap();
            pop_msg_and_assert(
                &mut game,
                GameMsg::PlayerUseDevelopmentCard(build.clone()),
            );
        } else {
            game.update(GameUpdate::UseDevelopmentCard(build.clone()))
                .expect_err("no card");
        }

        game.players[0].base.resources[TileKind::Stone as usize] += 3;
        game.players[0].base.resources[TileKind::Grain as usize] += 2;

        let build = BuildCity {
            player: 0,
            point: Coordinate::new(1, 7),
        };
        game.update(GameUpdate::BuildCity(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildCity(build.clone()));

        game.players[0].base.resources[TileKind::Stone as usize] += 3;
        game.players[0].base.resources[TileKind::Grain as usize] += 2;

        let build = BuildCity {
            player: 0,
            point: Coordinate::new(1, 5),
        };
        game.update(GameUpdate::BuildCity(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildCity(build.clone()));

        game.players[0].base.resources[TileKind::Stone as usize] += 3;
        game.players[0].base.resources[TileKind::Grain as usize] += 2;

        let build = BuildCity {
            player: 0,
            point: Coordinate::new(1, 1),
        };
        game.update(GameUpdate::BuildCity(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildCity(build.clone()));

        assert_eq!(game.players[0].base.score, 6);
        assert_eq!(game.players[0].base.get_longest_path(), 2);
        assert_eq!(game.players[1].base.score, 2);

        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;

        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 6),
                end: Coordinate::new(1, 6),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));

        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 6),
                end: Coordinate::new(0, 7),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));

        assert_eq!(game.players[0].base.score, 6);
        assert_eq!(game.players[0].base.get_longest_path(), 3);
        assert_eq!(game.players[1].base.score, 2);

        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(0, 7),
                end: Coordinate::new(0, 8),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));

        assert_eq!(game.players[0].base.score, 6);
        assert_eq!(game.players[0].base.get_longest_path(), 4);
        assert_eq!(game.players[1].base.score, 2);

        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;
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
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));
        assert_eq!(game.players[0].base.score, 6);
        assert_eq!(game.players[0].base.get_longest_path(), 5);
        assert_eq!(game.players[1].base.score, 2);

        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 7),
                end: Coordinate::new(1, 8),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));
        assert_eq!(game.players[0].base.score, 6);
        assert_eq!(game.players[0].base.get_longest_path(), 7);
        assert_eq!(game.players[1].base.score, 2);

        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 8),
                end: Coordinate::new(1, 9),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone())).unwrap();
        pop_msg_and_assert(&mut game, GameMsg::PlayerBuildRoad(build.clone()));
        assert_eq!(game.players[0].base.score, 6);
        assert_eq!(game.players[0].base.get_longest_path(), 7);
        assert_eq!(game.players[1].base.score, 2);

        game.players[0].base.resources[TileKind::Brick as usize] += 1;
        game.players[0].base.resources[TileKind::Wood as usize] += 1;
        let build = BuildRoad {
            player: 0,
            road: Line {
                start: Coordinate::new(1, 9),
                end: Coordinate::new(1, 10),
            },
        };
        game.update(GameUpdate::BuildRoad(build.clone()))
            .expect_err("invalid position");

        game.check_longest_path();
        assert_eq!(game.players[0].base.score, 8);
        assert_eq!(game.players[0].base.get_longest_path(), 7);
        assert_eq!(game.players[1].base.score, 2);
    }

    struct NetWorkPlayer {
        client_id: u128,
        tx: UnboundedSender<ServerMsg>,
        rx: UnboundedReceiver<ClientMsg>,
    }

    impl GamePlayer for NetWorkPlayer {
        fn get_name(&self) -> String {
            self.client_id.to_string()
        }

        async fn get_action(&mut self) -> GamePlayerAction {
            match self.rx.recv().await {
                Some(ClientMsg::Catan(action)) => GamePlayerAction::Catan(action),
                _ => GamePlayerAction::PlaceHolder,
            }
        }

        async fn send_message(&mut self, message: GamePlayerMessage) {
            match message {
                GamePlayerMessage::Catan(message) => {
                    self.tx.send(ServerMsg::Catan(message)).unwrap();
                },
                GamePlayerMessage::PlaceHolder => {},
            }
        }
    }

    #[tokio::test]
    async fn test_server() {
        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();
        let (player_tx, mut player_rx) =
            tokio::sync::mpsc::unbounded_channel::<NetWorkPlayer>();

        tokio::task::spawn(async move {
            loop {
                let player1 = player_rx.recv().await.unwrap();
                let player2 = player_rx.recv().await.unwrap();
                // tokio::task::spawn(async move {
                //     CatanGame::run(vec![player1], CatanDataSetup::Basic).await;
                // });

                tokio::task::spawn(async move {
                    CatanGame::run(vec![player1, player2], CatanDataSetup::Basic).await;
                });
            }
        });

        tokio::task::spawn(async move {
            let mut server = new_server();
            let mut clients = HashMap::new();
            let (server_tx, mut server_rx) =
                tokio::sync::mpsc::unbounded_channel::<(u128, ServerMsg)>();
            loop {
                select! {
                    server_event = server.next() => {
                        if let Some((client_id, server_event)) = server_event {
                            match server_event {
                                NetworkServerEvent::Report(connection_report) => {
                                    match connection_report {
                                        bevy_simplenet::ServerReport::Connected(_, _) => {
                                            // add client
                                            let (clt_tx, clt_rx) =
                                                tokio::sync::mpsc::unbounded_channel::<ClientMsg>();
                                            let (srv_tx, mut srv_rx) =
                                                tokio::sync::mpsc::unbounded_channel::<ServerMsg>();
                                            let _ = clients.insert(client_id, clt_tx);
                                            let server_tx_clone = server_tx.clone();
                                            player_tx
                                                .send(NetWorkPlayer {
                                                    client_id,
                                                    tx: srv_tx,
                                                    rx: clt_rx,
                                                })
                                                .unwrap();
                                            tokio::task::spawn(async move {
                                                loop {
                                                    match srv_rx.recv().await {
                                                        Some(msg) => {
                                                            server_tx_clone
                                                                .send((client_id, msg))
                                                                .unwrap();
                                                        },
                                                        _ => {},
                                                    };
                                                }
                                            });
                                        },
                                        bevy_simplenet::ServerReport::Disconnected => {
                                            // remove client
                                            let _ = clients.remove(&client_id);
                                        },
                                    }
                                },
                                NetworkServerEvent::Msg(msg) => match clients.get(&client_id) {
                                    Some(clt_tx) => {
                                        match clt_tx.send(msg) {
                                            Ok(_) => {},
                                            Err(_) => {
                                                let _ = clients.remove(&client_id);
                                            },
                                        }
                                    },
                                    None => {},
                                },
                                NetworkServerEvent::Request(..) => continue,
                            }
                        }
                    },
                    server_msg = server_rx.recv() => {
                        match server_msg {
                            Some((client_id, msg)) => {
                                server.send(client_id, msg);
                            },
                            None => break,
                        }
                    },
                }
            }
            let _ = close_tx.send(());
        });
        let _ = close_rx.await;
    }
}
