#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Up,
    Down,
    Left,
    Right,
    Hold,
    Block,
    DuplicateH,
    FilterR,
    Destroy,
    Empty,
    FilterU,
    FilterD,
    FilterL,
    DuplicateV,
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
            Tile::DuplicateH => 6,
            Tile::FilterR => 7,
            Tile::Destroy => 8,
            Tile::Empty => 9,
            Tile::FilterU => 10,
            Tile::FilterD => 11,
            Tile::FilterL => 12,
            Tile::DuplicateV => 13,
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
            6 => Self::DuplicateH,
            7 => Self::FilterR,
            8 => Self::Destroy,
            9 => Self::Empty,
            10 => Self::FilterU,
            11 => Self::FilterD,
            12 => Self::FilterL,
            13 => Self::DuplicateV,
            _ => Err(())?,
        })
    }
}


