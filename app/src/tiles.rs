#[derive(Clone, Copy, Debug)]
pub enum Tile {
    Elevator,
    Block,
    Flat,
    Right,
    Left,
    Hold, //might replace later
    Conditional,
    Duplicate,
    Spike,
    Empty,
}

impl From<Tile> for u8 {
    fn from(value: Tile) -> Self {
        match value {
            Tile::Elevator => 0,
            Tile::Block => 1,
            Tile::Flat => 2,
            Tile::Right => 3,
            Tile::Left => 4,
            Tile::Hold => 5,
            Tile::Conditional => 6,
            Tile::Duplicate => 7,
            Tile::Spike => 8,
            Tile::Empty => 9,
        }
    }
}

impl TryFrom<u8> for Tile {
    type Error = ();

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => Tile::Elevator,
            1 => Tile::Block,
            2 => Tile::Flat,
            3 => Tile::Right,
            4 => Tile::Left,
            5 => Tile::Hold,
            6 => Tile::Conditional,
            7 => Tile::Duplicate,
            8 => Tile::Spike,
            9 => Tile::Empty,
            _ => Err(())?,
        })
    }
}


