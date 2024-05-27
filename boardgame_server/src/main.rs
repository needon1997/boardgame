use std::collections::HashMap;

use boardgame_common::{
    network::{new_server, ClientMsg, NetworkServerEvent, ServerMsg},
    player::{GamePlayer, GamePlayerAction, GamePlayerMessage},
};
use data::CatanDataSetup;
use game::CatanGame;
use tokio::{
    select,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

pub mod data;
pub mod game;
pub mod tests;

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

#[tokio::main]
async fn main() {
    let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();
    let (player_tx, mut player_rx) =
        tokio::sync::mpsc::unbounded_channel::<NetWorkPlayer>();

    tokio::task::spawn(async move {
        loop {
            let player1 = player_rx.recv().await.unwrap();
            println!("Player 1 received");
            let player2 = player_rx.recv().await.unwrap();
            println!("Player 2 received");
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
