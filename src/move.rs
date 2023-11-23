use crate::board::{Piece, Side};
use crate::move_bitboards::{file, rank};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MoveType {
    Quiet,
    EnPassantCapture(Piece),
    Capture(Piece),
    CastleLong,
    CastleShort,

    Promotion(Piece),
    CapturePromotion(Piece, Piece)
}

#[derive(Debug, PartialEq, Eq)]
pub enum DrawReason {
    FiftyMoveRule,
    InsufficientMaterial,
    ThreeFoldRepetition,
    Stalemate
}

#[derive(Debug, PartialEq, Eq)]
pub enum MoveResult {
    Check,
    Checkmate,
    Draw(DrawReason)
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Move {
    pub from_square: usize,
    pub to_square: usize,
    pub move_type: MoveType,
    pub piece: Piece,
    pub side: Side,
}

impl Move {
    const PIECE_SYMBOLS: &[&'static str] = &["", "N", "B", "R", "Q", "K"];
    pub fn to_algebraic(&self) -> String {
        self.to_algebraic_with_state(&vec![])
    }

    pub fn to_algebraic_with_state(&self, legal_moves: &Vec<Move>) -> String {
        let from_algabraic = idx_to_square(self.from_square);
        let to_algabraic = idx_to_square(self.to_square);
        let mut piece_symbol = Self::PIECE_SYMBOLS[self.piece as usize].to_owned();

        let is_quiet_move = self.is_quiet();
        let is_capture_move = self.is_capture();

        // Check for multiple legal knight/rook/queen moves to same target_square
        if legal_moves.len() != 0 {
            let ambiguous_pieces = [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];

            for piece in ambiguous_pieces {
                if self.piece == piece {
                    let same_pieces_to_target_rank = legal_moves.iter().filter(|m| {
                        m.piece == piece
                            && m.to_square == self.to_square
                            && (rank(m.to_square as usize) == rank(self.to_square as usize))
                    }).count();
                    let same_pieces_to_target_file = legal_moves.iter().filter(|m| {
                        m.piece == piece
                            && m.to_square == self.to_square
                            && (file(m.to_square as usize) == file(self.to_square as usize))
                    }).count();

                    if same_pieces_to_target_rank == 2 {
                        // Two pieces on the same rank can moves to same target
                        // square, include the file
                        piece_symbol.push(from_algabraic.chars().nth(0).unwrap());
                    } else if same_pieces_to_target_file == 2 {
                        // Two pieces on the same rank can moves to same target
                        // square, include the rank
                        piece_symbol.push(from_algabraic.chars().nth(1).unwrap());
                    } else if same_pieces_to_target_rank > 2 || same_pieces_to_target_file > 2 {
                        // More than two pieces can moves to same target square,
                        // include the full position
                        piece_symbol.push_str(&from_algabraic);
                    }
                }
            }
        }

        let move_str = if is_quiet_move {
            format!("{}{}", piece_symbol, to_algabraic)
        } else if is_capture_move {
            if self.piece == Piece::Pawn {
                format!("{}x{}", from_algabraic.chars().nth(0).unwrap(), to_algabraic)
            } else {
                format!("{}x{}", piece_symbol, to_algabraic)
            }
        } else if self.move_type == MoveType::CastleShort {
            "0-0".to_string()
        } else if self.move_type == MoveType::CastleLong {
            "0-0-0".to_string()
        } else {
            unreachable!()
        };

        match self.move_type {
            MoveType::Promotion(promotion_piece) | MoveType::CapturePromotion(_, promotion_piece) =>
                format!("{}={}", move_str, Self::PIECE_SYMBOLS[promotion_piece as usize]),
            _ => move_str
        }
    }

    pub fn is_quiet(&self) -> bool {
        match self.move_type {
            MoveType::Quiet | MoveType::Promotion(_) => true,
            _ => false
        }
    }

    pub fn is_capture(&self) -> bool {
        match self.move_type {
            MoveType::Capture(_) | MoveType::CapturePromotion(_, _)
                | MoveType::EnPassantCapture(_) => true,
            _ => false
        }
    }

    pub fn is_promotion(&self) -> bool {
        match self.move_type {
            MoveType::Promotion(_) | MoveType::CapturePromotion(_, _) => true,
            _ => false
        }
    }

    pub fn is_castling(&self) -> bool {
        self.move_type == MoveType::CastleShort || self.move_type == MoveType::CastleLong
    }

    pub fn prio(&self) -> u64 {
        let mut score = 10;

        if self.is_capture() {
            score += 10;
            score -= self.piece as u64;
        }

        if let MoveType::Promotion(promotion_piece) | MoveType::CapturePromotion(_, promotion_piece) = self.move_type {
            score += 20;
            score += promotion_piece as u64;
        }

        // Consider moves close to the center first
        // score -= (4 - file(self.from_square) as i8).abs() as u64;

        score
    }

}

pub const FILES: &[&str] = &["h", "g", "f", "e", "d", "c", "b", "a"];
pub const RANKS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8"];
fn idx_to_square(idx: usize) -> String {
    let file = file(idx);
    let rank = rank(idx);
    format!("{}{}", FILES[file], RANKS[rank])
}
