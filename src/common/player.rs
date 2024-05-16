use crate::catan::element;

use super::element::Coordinate;

pub enum GamePlayerAction {
    Catan(element::GameAct),
    PlaceHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GamePlayerMessage {
    Catan(element::GameMsg),
    PlaceHolder,
}

pub trait GamePlayer {
    fn get_name(&self) -> String;
    async fn get_action(&mut self) -> GamePlayerAction;
    async fn send_message(&mut self, message: GamePlayerMessage);
}
