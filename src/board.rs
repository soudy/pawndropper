use std::fmt;
use std::ops::{Index, IndexMut};

use crate::r#move::{Move, MoveType, RANKS, FILES};

pub const BOARD_WIDTH: usize = 8;
pub const BOARD_HEIGHT: usize = 8;
pub const N_SQUARES: usize = BOARD_WIDTH*BOARD_HEIGHT;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub const N_SIDES: usize = 2;
    pub const VALUES: [Side; Self::N_SIDES] = [Self::White, Self::Black];

    #[inline]
    pub fn opposite(self) -> Self {
        Self::VALUES[1usize - (self as usize)]
    }

    pub fn from_str(string: &str) -> Self {
        match string {
            "white" => Side::White,
            "black" => Side::Black,
            _ => panic!("invalid side {}", string)
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Piece {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl Piece {
    pub const N_PIECES: usize = 6;
    pub const N_SLIDING_PIECES: usize = 3;
    pub const SLIDER_START_VALUE: usize = 2;
    pub const VALUES: [Piece; Self::N_PIECES] = [
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];
    pub const ALL_BUT_KING: [Piece; Self::N_PIECES - 1] = [
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
    ];
    pub const PROMOTION_PIECES: [Piece; 4] = [
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
    ];

    #[inline]
    pub fn is_slider(self) -> bool {
        let self_idx = self as usize;
        // Sliders are grouped in piece enum: bishop (2), rook (3), queen (4)
        self_idx > 1 && self_idx < 5
    }

    pub fn ascii(&self, side: Side) -> &str {
        if side == Side::White {
            match self {
                Piece::Pawn => "♟︎",
                Piece::Knight => "♞",
                Piece::Bishop => "♝",
                Piece::Rook => "♜",
                Piece::Queen => "♛",
                Piece::King => "♚",
            }
        } else {
            match self {
                Piece::Pawn => "♙",
                Piece::Knight => "♘",
                Piece::Bishop => "♗",
                Piece::Rook => "♖",
                Piece::Queen => "♕",
                Piece::King => "♔",
            }
        }
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let symbol = match self {
            Piece::Pawn => "p",
            Piece::Knight => "N",
            Piece::Bishop => "B",
            Piece::Rook => "R",
            Piece::Queen => "Q",
            Piece::King => "K",
        };
        fmt.write_str(symbol)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    pub pawns: [u64; Side::N_SIDES],
    pub knights: [u64; Side::N_SIDES],
    pub bishops: [u64; Side::N_SIDES],
    pub rooks: [u64; Side::N_SIDES],
    pub queens: [u64; Side::N_SIDES],
    pub king: [u64; Side::N_SIDES],

    pub side_to_move: Side,
    pub castling_right_long: [bool; Side::N_SIDES],
    pub castling_right_short: [bool; Side::N_SIDES],
    pub en_passant_square: usize,
}

impl Default for Board {
    fn default() -> Self {
        Board {
            pawns: [
                // White pawns
                0b0000000000000000000000000000000000000000000000001111111100000000,
                // Black pawns
                0b0000000011111111000000000000000000000000000000000000000000000000,
            ],
            knights: [
                // White knights
                0b0000000000000000000000000000000000000000000000000000000001000010,
                // Black knights
                0b0100001000000000000000000000000000000000000000000000000000000000,
            ],
            bishops: [
                // White bishops
                0b0000000000000000000000000000000000000000000000000000000000100100,
                // Black bishops
                0b0010010000000000000000000000000000000000000000000000000000000000,
            ],
            rooks: [
                // White rooks
                0b0000000000000000000000000000000000000000000000000000000010000001,
                // Black rooks
                0b1000000100000000000000000000000000000000000000000000000000000000,
            ],
            queens: [
                // White queens
                0b0000000000000000000000000000000000000000000000000000000000010000,
                // Black queens
                0b0001000000000000000000000000000000000000000000000000000000000000,
            ],
            king: [
                // White king
                0b0000000000000000000000000000000000000000000000000000000000001000,
                // Black king
                0b0000100000000000000000000000000000000000000000000000000000000000,
            ],

            side_to_move: Side::White,

            en_passant_square: 0usize,
            castling_right_long: [true, true],
            castling_right_short: [true, true],
        }
    }
}

impl Index<Piece> for Board {
    type Output = [u64; Side::N_SIDES];

    fn index(&self, piece: Piece) -> &Self::Output {
        match piece {
            Piece::Pawn => &self.pawns,
            Piece::Knight => &self.knights,
            Piece::Bishop => &self.bishops,
            Piece::Rook => &self.rooks,
            Piece::Queen => &self.queens,
            Piece::King => &self.king,
        }
    }
}

impl Index<(Piece, Side)> for Board {
    type Output = u64;

    fn index(&self, (piece, side): (Piece, Side)) -> &Self::Output {
        let side = side as usize;
        match piece {
            Piece::Pawn => &self.pawns[side],
            Piece::Knight => &self.knights[side],
            Piece::Bishop => &self.bishops[side],
            Piece::Rook => &self.rooks[side],
            Piece::Queen => &self.queens[side],
            Piece::King => &self.king[side],
        }
    }
}

impl IndexMut<(Piece, Side)> for Board {
    fn index_mut(&mut self, (piece, side): (Piece, Side)) -> &mut Self::Output {
        let side = side as usize;
        match piece {
            Piece::Pawn => &mut self.pawns[side],
            Piece::Knight => &mut self.knights[side],
            Piece::Bishop => &mut self.bishops[side],
            Piece::Rook => &mut self.rooks[side],
            Piece::Queen => &mut self.queens[side],
            Piece::King => &mut self.king[side],
        }
    }
}

impl Board {
    pub fn make_move(&mut self, m: &Move) {
        self.update_en_passant_flag(m);
        self.update_castling_rights(m);

        match m.move_type {
            MoveType::CastleShort => {
                // Move king
                self[(Piece::King, m.side)] >>= 2;

                // Move rook
                if m.side == Side::White {
                    self[(Piece::Rook, m.side)] &= !(1 << 0);
                    self[(Piece::Rook, m.side)] |= 1 << 2;
                } else {
                    self[(Piece::Rook, m.side)] &= !(1 << (BOARD_WIDTH*(BOARD_HEIGHT - 1)));
                    self[(Piece::Rook, m.side)] |= 1 << (BOARD_WIDTH*(BOARD_HEIGHT - 1) + 2);
                }
            },
            MoveType::CastleLong => {
                // Move king
                self[(Piece::King, m.side)] <<= 2;

                // Move rook
                if m.side == Side::White {
                    self[(Piece::Rook, m.side)] &= !(1 << (BOARD_WIDTH - 1));
                    self[(Piece::Rook, m.side)] |= 1 << (BOARD_WIDTH - 1 - 3);
                } else {
                    self[(Piece::Rook, m.side)] &= !(1 << (BOARD_WIDTH*BOARD_HEIGHT - 1));
                    self[(Piece::Rook, m.side)] |= 1 << (BOARD_WIDTH*BOARD_HEIGHT - 1 - 3)
                }
            },
            _ => {
                // Quiet, capture, and promotion moves

                // Remove piece from its current square
                self[(m.piece, m.side)] &= !(1 << m.from_square);

                // Move piece to target square
                // In the case of pawn promotion, we create the respective promotion 
                // type instead of a pawn
                let new_square_piece_type = match m.move_type {
                    MoveType::Promotion(promotion_piece)
                    | MoveType::CapturePromotion(_, promotion_piece) => promotion_piece,
                    _ => m.piece
                };
                self[(new_square_piece_type, m.side)] |= 1 << m.to_square;

                // Capture case, also remove the captured piece from enemy board
                match m.move_type {
                    MoveType::Capture(captured_piece) | MoveType::CapturePromotion(captured_piece, _) =>
                        self[(captured_piece, m.side.opposite())] &= !(1 << m.to_square),
                    MoveType::EnPassantCapture(captured_piece) => {
                        let enemy_pawn_square = (m.to_square as i64 + (((m.side as i64)*2 - 1)*(BOARD_WIDTH as i64))) as usize;
                        self[(captured_piece, m.side.opposite())] &= !(1 << enemy_pawn_square);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn undo_move(
        &mut self,
        m: &Move,
        castling_right_long: &[bool; Side::N_SIDES],
        castling_right_short: &[bool; Side::N_SIDES],
    ) {
        // Restore from values the caller saved
        self.castling_right_short = *castling_right_short;
        self.castling_right_long = *castling_right_long;

        // Other side to move
        self.side_to_move = self.side_to_move.opposite();

        self.en_passant_square = 0;

        match m.move_type {
            MoveType::CastleShort => {
                // Move king back
                self[(Piece::King, m.side)] <<= 2;

                // Move rook back
                if m.side == Side::White {
                    self[(Piece::Rook, m.side)] &= !(1 << 2);
                    self[(Piece::Rook, m.side)] |= 1 << 0;
                } else {
                    self[(Piece::Rook, m.side)] &= !(1 << (BOARD_WIDTH*(BOARD_HEIGHT - 1) + 2));
                    self[(Piece::Rook, m.side)] |= 1 << (BOARD_WIDTH*(BOARD_HEIGHT - 1));
                }
            },
            MoveType::CastleLong => {
                // Move king back
                self[(Piece::King, m.side)] >>= 2;

                // Move rook back
                if m.side == Side::White {
                    self[(Piece::Rook, m.side)] &= !(1 << (BOARD_WIDTH - 1 - 3));
                    self[(Piece::Rook, m.side)] |= 1 << (BOARD_WIDTH - 1);
                } else {
                    self[(Piece::Rook, m.side)] &= !(1 << (BOARD_WIDTH*BOARD_HEIGHT - 1 - 3));
                    self[(Piece::Rook, m.side)] |= 1 << (BOARD_WIDTH*BOARD_HEIGHT - 1);
                }
            },
            _ => {
                // Quiet, capture, and promotion moves

                // Pawn promotion, remove the promoted piece from the target square
                if let MoveType::Promotion(promoted_piece) | MoveType::CapturePromotion(_, promoted_piece) = m.move_type  {
                    self[(promoted_piece, m.side)] &= !(1 << m.to_square);
                } else {
                    // No promotion, just remove piece from its target square
                    self[(m.piece, m.side)] &= !(1 << m.to_square);
                }

                // Move piece back to from square
                self[(m.piece, m.side)] |= 1 << m.from_square;

                // Capture case, also put the captured piece back into enemy board
                if let MoveType::Capture(captured_piece) | MoveType::CapturePromotion(captured_piece, _) = m.move_type {
                    self[(captured_piece, m.side.opposite())] |= 1 << m.to_square;
                } else if let MoveType::EnPassantCapture(captured_piece) = m.move_type {
                    // Put back en passant-captured pawn
                    let enemy_pawn_square = (m.to_square as i64 + (((m.side as i64)*2 - 1)*(BOARD_WIDTH as i64))) as usize;
                    self[(captured_piece, m.side.opposite())] |= 1 << enemy_pawn_square;

                    // Re-set en passant square flag
                    self.en_passant_square = m.to_square;
                }
            }
        }
    }

    pub fn update_en_passant_flag(&mut self, m: &Move) {
        // Reset first
        self.en_passant_square = 0;

        // Then set if applicable
        if m.piece == Piece::Pawn && m.move_type == MoveType::Quiet {
            let n_ranks_moved = m.to_square as i64 - m.from_square as i64;
            if n_ranks_moved.abs() == (BOARD_WIDTH * 2) as i64 {
                // Pawn moved two ranks forward, mark to_square as the en passant square
                self.en_passant_square =
                    (m.to_square as i64 - (n_ranks_moved.signum())*(BOARD_WIDTH as i64)) as usize;
            }
        }
    }

    pub const ROOK_SHORT_SQUARES: [usize; Side::N_SIDES] = [0, BOARD_WIDTH*(BOARD_HEIGHT - 1)];
    pub const ROOK_LONG_SQUARES: [usize; Side::N_SIDES] = [
        BOARD_WIDTH - 1,
        BOARD_WIDTH*BOARD_HEIGHT - 1,
    ];
    pub fn update_castling_rights(&mut self, m: &Move) {
        match m.piece {
            Piece::King => {
                // King move (including castling), disable castling rights both sides
                self.castling_right_long[m.side as usize] = false;
                self.castling_right_short[m.side as usize] = false;
            },
            Piece::Rook => {
                // Rook move, disable castling rights for the respective side
                // (if castling was possible before)
                if self.castling_right_short[m.side as usize]
                    && m.from_square == Self::ROOK_SHORT_SQUARES[m.side as usize] {
                    self.castling_right_short[m.side as usize] = false;
                } else if self.castling_right_long[m.side as usize]
                    && m.from_square == Self::ROOK_LONG_SQUARES[m.side as usize] {
                    self.castling_right_long[m.side as usize] = false;
                }
            },
            _ => {}
        }
    }

    /// Generate bitboard of all pieces of one side in which 1 indicates a square
    /// occupied by a piece and 0 indicates a square unoccupied by a piece.
    ///
    /// * `side`: Side to get occupation bitboard for.
    pub fn occupation_board(&self, side: Side) -> u64 {
        let mut occ_bb = 0u64;

        for piece in Piece::VALUES {
            occ_bb |= self[(piece, side)];
        }

        occ_bb
    }

    pub fn to_ascii(&self, play_side: Side) -> String {
        let mut fmt = String::new();

        for mut j in 0..BOARD_HEIGHT {
            if play_side == Side::Black {
                j = BOARD_HEIGHT - 1 - j;
            }

            fmt.push_str(&format!("{}  ", RANKS[BOARD_HEIGHT - j - 1]));

            for i in 0..BOARD_WIDTH {
                let pos: u64 = 1 << ((u64::BITS as usize) - 1 - (8 * j + i));
                let mut no_piece = true;
                'piece: for piece in Piece::VALUES {
                    for side in Side::VALUES {
                        let is_black = side == Side::Black;
                        if pos & self[(piece, side)] != 0 {
                            if is_black {
                                fmt.push_str("\x1b[31m");
                            }
                            fmt.push_str(piece.ascii(side));
                            if is_black {
                                fmt.push_str("\x1b[0m");
                            }

                            no_piece = false;
                            break 'piece;
                        }
                    }
                }

                if no_piece {
                    fmt.push_str(".");
                }

                if i != BOARD_WIDTH - 1 {
                    fmt.push_str(" ");
                }
            }
            if (play_side == Side::White && j != 7) || (play_side == Side::Black && j != 0) {
                fmt.push_str("\n");
            }
        }

        fmt.push_str("\n\n   ");
        for j in 0..BOARD_WIDTH {
            fmt.push_str(&format!("{} ", FILES[BOARD_WIDTH - j - 1]));
        }
        fmt.push_str("\n");

        fmt
    }
}
