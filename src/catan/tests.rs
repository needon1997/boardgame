#[cfg(test)]
mod tests {

    use std::sync::{Arc, Mutex};

    use enfync::Handle;
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

    use crate::{
        catan::{self, data::*, element::*, game::*},
        common::{
            element::{Coordinate, Line},
            network::{
                new_server, ClientMsg, NetworkServer, NetworkServerEvent, ServerMsg,
            },
            player::{self, GamePlayer, GamePlayerAction, GamePlayerMessage},
        },
    };
    //third-party shortcuts
    use bevy::prelude::*;
    use bevy::{app::*, utils::HashMap};

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
        assert_eq!(game.players[0].base.get_longest_road(), 2);
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
            card: catan::element::DevCard::YearOfPlenty,
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
        assert_eq!(game.players[0].base.get_longest_road(), 2);
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
        assert_eq!(game.players[0].base.get_longest_road(), 3);
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
        assert_eq!(game.players[0].base.get_longest_road(), 4);
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
        assert_eq!(game.players[0].base.get_longest_road(), 5);
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
        assert_eq!(game.players[0].base.get_longest_road(), 7);
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
        assert_eq!(game.players[0].base.get_longest_road(), 7);
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

        game.check_longest_road();
        assert_eq!(game.players[0].base.score, 8);
        assert_eq!(game.players[0].base.get_longest_road(), 7);
        assert_eq!(game.players[1].base.score, 2);
    }

    #[derive(Resource, Default)]
    struct ClientConnections(HashMap<u128, UnboundedSender<ClientMsg>>);

    #[derive(Resource, Default, Clone)]
    struct ClientPending(Arc<Mutex<HashMap<u128, Vec<ServerMsg>>>>);

    #[derive(Resource, Clone)]
    struct PlayerTx(UnboundedSender<NetWorkPlayer>);

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

    fn handle_server_events(
        mut server: ResMut<NetworkServer>, mut clients: ResMut<ClientConnections>,
        clients_pending: ResMut<ClientPending>, player_tx: Res<PlayerTx>,
    ) {
        while let Some((client_id, server_event)) = server.next() {
            match server_event {
                NetworkServerEvent::Report(connection_report) => {
                    match connection_report {
                        bevy_simplenet::ServerReport::Connected(_, _) => {
                            // add client
                            let (clt_tx, clt_rx) =
                                tokio::sync::mpsc::unbounded_channel::<ClientMsg>();
                            let (srv_tx, mut srv_rx) =
                                tokio::sync::mpsc::unbounded_channel::<ServerMsg>();
                            let _ = clients.0.insert(client_id, clt_tx);
                            let pending = clients_pending.as_ref().clone();
                            let player_tx = player_tx.clone();
                            enfync::builtin::native::TokioHandle::default().spawn(
                                async move {
                                    player_tx
                                        .0
                                        .send(NetWorkPlayer {
                                            client_id,
                                            tx: srv_tx,
                                            rx: clt_rx,
                                        })
                                        .unwrap();
                                },
                            );
                            enfync::builtin::native::TokioHandle::default().spawn(
                                async move {
                                    loop {
                                        match srv_rx.recv().await {
                                            Some(msg) => {
                                                let mut lock = pending.0.lock().unwrap();
                                                let pending = lock
                                                    .entry(client_id)
                                                    .or_insert(Vec::new());
                                                pending.push(msg);
                                            },
                                            _ => {},
                                        };
                                    }
                                },
                            );
                            // // send current server state to client
                            // // - we must use new_button_state to ensure the order of events is preserved
                            // let current_state = new_button_state;
                            // server.send(client_id, DemoServerMsg::Current(current_state));
                        },
                        bevy_simplenet::ServerReport::Disconnected => {
                            // remove client
                            let _ = clients.0.remove(&client_id);
                        },
                    }
                },
                NetworkServerEvent::Msg(msg) => match clients.0.get(&client_id) {
                    Some(clt_tx) => {
                        clt_tx.send(msg).unwrap();
                    },
                    None => {},
                },
                NetworkServerEvent::Request(..) => continue,
            }
        }

        let mut lock = clients_pending.0.lock().unwrap();
        let drain = lock.drain();
        for (client_id, pending) in drain {
            for msg in pending.iter() {
                server.send(client_id, msg.clone());
            }
        }
        drop(lock)
    }

    #[tokio::test]
    async fn test_server() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let (player_tx, mut player_rx) =
            tokio::sync::mpsc::unbounded_channel::<NetWorkPlayer>();

        tokio::task::spawn(async move {
            loop {
                let player1 = player_rx.recv().await.unwrap();
                // let player2 = player_rx.recv().await.unwrap();
                tokio::task::spawn(async move {
                    CatanGame::run(vec![player1], CatanDataSetup::Basic).await;
                });

                // tokio::task::spawn(async move {
                //     CatanGame::run(vec![player1, player2], CatanDataSetup::Basic).await;
                // });
            }
        });
        tokio::task::spawn_blocking(|| {
            let server = new_server();
            let mut app = App::empty();

            app.add_plugins(ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_millis(100),
            ))
            .init_schedule(Main)
            .insert_resource(PlayerTx(player_tx))
            .insert_resource(server)
            .insert_resource(ClientConnections::default())
            .insert_resource(ClientPending::default())
            .add_systems(Main, handle_server_events)
            .run();
            tx.send(()).unwrap();
        });
        rx.await.unwrap();
    }
}
