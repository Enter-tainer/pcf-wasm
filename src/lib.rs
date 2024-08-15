use std::sync::atomic::AtomicBool;
use fumen::Fumen;

use arrayvec::ArrayVec;

mod combination;
pub mod placeability;
pub mod common;
mod solve;

pub use combination::*;
pub use solve::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Piece {
    S,
    Z,
    J,
    L,
    T,
    O,
    I,
}

impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<usize> for Piece {
    fn from(i: usize) -> Piece {
        PIECES[i]
    }
}

pub const PIECES: [Piece; 7] = [
    Piece::S,
    Piece::Z,
    Piece::J,
    Piece::L,
    Piece::T,
    Piece::O,
    Piece::I,
];

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceSet(pub [u8; 7]);

impl PieceSet {
    #[inline]
    pub fn without(mut self, p: Piece) -> PieceSet {
        self.0[p as usize] -= 1;
        self
    }

    #[inline]
    pub fn contains(self, p: Piece) -> bool {
        self.0[p as usize] != 0
    }

    #[inline]
    pub fn with(mut self, p: Piece) -> PieceSet {
        self.0[p as usize] += 1;
        self
    }
}

impl Default for PieceSet {
    fn default() -> Self {
        PieceSet([0; 7])
    }
}

impl std::fmt::Display for PieceSet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        const NAMES: [char; 7] = ['S', 'Z', 'J', 'L', 'T', 'O', 'I'];
        for i in 0..7 {
            for _ in 0..self.0[i] {
                write!(f, "{}", NAMES[i])?;
            }
        }
        Ok(())
    }
}

impl std::iter::FromIterator<Piece> for PieceSet {
    fn from_iter<T: IntoIterator<Item = Piece>>(iter: T) -> Self {
        let mut this = PieceSet::default();
        for piece in iter {
            this = this.with(piece)
        }
        this
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct BitBoard(pub u64);

impl BitBoard {
    #[inline]
    pub fn filled(height: usize) -> BitBoard {
        BitBoard((1 << height * 10) - 1)
    }

    #[inline]
    pub fn combine(self, other: Self) -> Self {
        BitBoard(self.0 | other.0)
    }

    #[inline]
    pub fn remove(self, other: Self) -> Self {
        BitBoard(self.0 & !other.0)
    }

    #[inline]
    pub fn overlaps(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    #[inline]
    pub fn cell_filled(self, x: usize, y: usize) -> bool {
        y < 6 && self.0 & 1 << x + y * 10 != 0
    }

    #[inline]
    pub fn line_filled(self, y: usize) -> bool {
        self.0 >> 10 * y & (1 << 10) - 1 == (1 << 10) - 1
    }

    #[inline]
    pub fn lines_cleared(self) -> BitBoard {
        let mut b = 0;
        let mut row = 0;
        for y in 0..6 {
            if !self.line_filled(y) {
                b |= (self.0 >> 10 * y & (1 << 10) - 1) << 10 * row;
                row += 1;
            }
        }
        BitBoard(b)
    }

    #[inline]
    pub fn leftmost_empty_column(self, height: usize) -> usize {
        // start with completely filled row
        let mut collapsed = (1 << 10) - 1;
        for i in 0..height {
            collapsed &= self.0 >> i * 10;
        }
        // collapsed has a 0 wherever there's an empty cell in rows 0..height
        // so to find the x of first one, we need only count the number of 1s before it
        (!collapsed).trailing_zeros() as usize
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Placement {
    pub kind: PieceState,
    pub x: u8,
}

impl Placement {
    #[inline]
    pub fn board(self) -> BitBoard {
        BitBoard(self.kind.board().0 << self.x)
    }

    #[inline]
    pub fn supported_after_clears(self, mut on: BitBoard) -> bool {
        let hurdled_lines = HURDLE_MASKS[self.kind.hurdles() as usize];
        if BitBoard(hurdled_lines).remove(on) != BitBoard(0) {
            // hurdled lines not filled means the hurdled placement is impossible
            false
        } else {
            // copy lines below filled lines into filled lines
            for y in 1..6 {
                if on.line_filled(y) {
                    on.0 &= (on.0 << 10) | !((1 << 10) - 1 << 10 * y);
                }
            }
            self.kind.y() == 0 || on.overlaps(BitBoard(self.kind.below_mask().0 << self.x))
        }
    }

    #[inline]
    pub fn supported_without_clears(self, on: BitBoard) -> bool {
        let hurdled_lines = HURDLE_MASKS[self.kind.hurdles() as usize];
        if BitBoard(hurdled_lines).remove(on) != BitBoard(0) {
            // hurdled lines not filled means the hurdled placement is impossible
            false
        } else {
            self.kind.y() == 0 || on.overlaps(BitBoard(self.kind.below_mask().0 << self.x))
        }
    }

    #[inline]
    pub fn harddrop_mask(self) -> BitBoard {
        BitBoard(self.kind.harddrop_mask().0 << self.x)
    }

    #[inline]
    pub fn srs_piece(self, board: BitBoard) -> ArrayVec<[SrsPiece; 4]> {
        let mut below_lines = 0;
        for i in 0..self.kind.y() {
            if board.0 >> 10 * i & (1 << 10) - 1 == (1 << 10) - 1 {
                below_lines += 1;
            }
        }
        self.kind
            .piece_srs()
            .iter()
            .map(|&p| SrsPiece {
                x: self.x as i32 + p.x,
                y: p.y - below_lines,
                ..p
            })
            .collect()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Rotation {
    North,
    East,
    South,
    West,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SrsPiece {
    pub piece: Piece,
    pub rotation: Rotation,
    pub x: i32,
    pub y: i32,
}

pub use data::PieceState;
include!(concat!(env!("OUT_DIR"), "/data.rs"));

const HURDLE_MASKS: [u64; 64] = {
    let mut result = [0; 64];
    let mut hurdle_mask = 0;
    while hurdle_mask < 64 {
        let mut hurdled_lines = 0;
        let mut i = 5;
        while i >= 0 {
            hurdled_lines <<= 10;
            if hurdle_mask & 1 << i != 0 {
                hurdled_lines |= (1 << 10) - 1;
            }
            i -= 1;
        }
        result[hurdle_mask] = hurdled_lines;
        hurdle_mask += 1;
    }
    result
};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn get_pc(board: u64, sequence: String, allow_hold: Option<bool>) -> String {
    let mut fumen = Fumen::default();
    let allow_hold = allow_hold.unwrap_or(true);
    let board = BitBoard(board);
    let queue: Vec<_> = sequence
        .chars().filter_map(|x| match x {
            'I' => Some(Piece::I),
            'T' => Some(Piece::T),
            'O' => Some(Piece::O),
            'L' => Some(Piece::L),
            'J' => Some(Piece::J),
            'S' => Some(Piece::S),
            'Z' => Some(Piece::Z),
            _ => None,
        })
        .collect();
    solve_pc(
        &queue,
        board,
        allow_hold,
        true,
        &AtomicBool::new(false),
        placeability::simple_srs_spins,
        |soln| common::add_placement_pages(&mut fumen, board, soln),
    );
    fumen.encode()
}
