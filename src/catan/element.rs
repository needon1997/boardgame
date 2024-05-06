use crate::common::element::{Coordinate, Line};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DevCard {
    Knight,
    VictoryPoint,
    RoadBuilding,
    Monopoly,
    YearOfPlenty,
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

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum TileKind {
    #[default]
    Empty,
    Dessert,
    Wood,
    Brick,
    Grain,
    Wool,
    Stone,
    Max,
}

#[derive(Default, Copy, Clone)]
pub struct Tile {
    pub kind: TileKind,
}

impl Tile {
    pub fn is_empty(&self) -> bool {
        self.kind == TileKind::Empty
    }

    pub fn kind(&self) -> TileKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: TileKind) {
        self.kind = kind;
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
