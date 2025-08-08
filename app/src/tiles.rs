#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Up,
    Down,
    Left,
    Right,
    Hold,
    Block,
    Duplicate,
    Filter,
    Destroy,
}

impl From<Tile> for u8 {
    fn from(value: Tile) -> Self {
        match value {
            Tile::Up => 0,
            Tile::Down => 1,
            Tile::Left => 2,
            Tile::Right => 3,
            Tile::Hold => 4,
            Tile::Block => 5,
            Tile::Duplicate => 6,
            Tile::Filter => 7,
            Tile::Destroy => 8,
        }
    }
}

impl TryFrom<u8> for Tile {
    type Error = ();

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => Tile::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            4 => Self::Hold,
            5 => Self::Block,
            6 => Self::Duplicate,
            7 => Self::Filter,
            8 => Self::Destroy,
            _ => Err(())?,
        })
    }
}


