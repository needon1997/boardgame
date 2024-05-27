use super::catan::element::{GameAct, GameMsg};

pub enum GamePlayerAction {
    Catan(GameAct),
    PlaceHolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GamePlayerMessage {
    Catan(GameMsg),
    PlaceHolder,
}

pub trait GamePlayer {
    fn get_name(&self) -> String;
    async fn get_action(&mut self) -> GamePlayerAction;
    async fn send_message(&mut self, message: GamePlayerMessage);
}
