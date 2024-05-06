use std::sync::Arc;

use crate::catan;

use super::element::Coordinate;

pub enum GamePlayerAction {
    Catan(catan::game::PlayerActionCommand),
    PlaceHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GamePlayerMessage {
    Catan(catan::game::GameMessage),
    PlaceHolder,
}

pub trait GamePlayer {
    fn get_name(&self) -> String;
    async fn get_action(&mut self) -> GamePlayerAction;
    async fn send_message(&mut self, message: GamePlayerMessage);
}

pub struct OnlinePlayer {
    name: String,
}

impl GamePlayer for OnlinePlayer {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    async fn get_action(&mut self) -> GamePlayerAction {
        GamePlayerAction::Catan(catan::game::PlayerActionCommand::BuildRoad(
            Coordinate::new(0, 0),
            Coordinate::new(0, 1),
        ))
    }

    async fn send_message(&mut self, message: GamePlayerMessage) {
        match message {
            GamePlayerMessage::Catan(message) => {
                println!("OnlinePlayer: {:?}", message);
            },
            GamePlayerMessage::PlaceHolder => {
                println!("OnlinePlayer: Placeholder");
            },
        }
    }
}
