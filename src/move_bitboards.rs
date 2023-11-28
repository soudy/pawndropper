use crate::board::{Piece, Side, BOARD_HEIGHT, BOARD_WIDTH, N_SQUARES};
use std::ops::Index;

use log::info;

pub const FILE_MASKS: [u64; BOARD_WIDTH] = [
    0b100000001000000010000000100000001000000010000000100000001,
    0b1000000010000000100000001000000010000000100000001000000010,
    0b10000000100000001000000010000000100000001000000010000000100,
    0b100000001000000010000000100000001000000010000000100000001000,
    0b1000000010000000100000001000000010000000100000001000000010000,
    0b10000000100000001000000010000000100000001000000010000000100000,
    0b100000001000000010000000100000001000000010000000100000001000000,
    0b1000000010000000100000001000000010000000100000001000000010000000,
];

pub const RANK_MASKS: [u64; BOARD_WIDTH] = [
    0b11111111,
    0b1111111100000000,
    0b111111110000000000000000,
    0b11111111000000000000000000000000,
    0b1111111100000000000000000000000000000000,
    0b111111110000000000000000000000000000000000000000,
    0b11111111000000000000000000000000000000000000000000000000,
    0b1111111100000000000000000000000000000000000000000000000000000000,
];

#[inline]
pub fn file(square: usize) -> usize {
    square % BOARD_WIDTH
}

#[inline]
pub fn rank(square: usize) -> usize {
    square / BOARD_HEIGHT
}

#[derive(Debug, Copy, Clone)]
pub enum RayDirection {
    // Rook ray directions
    North,
    East,
    South,
    West,

    // Bishop ray directions
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl RayDirection {
    pub const N_DIRECTIONS: usize = 8;
    pub const BISHOP_DIRECTIONS: [Self; 4] = [
        Self::NorthEast, Self::SouthEast, Self::SouthWest, Self::NorthWest 
    ];
    pub const ROOK_DIRECTIONS: [Self; 4] = [
        Self::North, Self::East, Self::South, Self::West 
    ];
}

pub struct MoveBitboards {
    pub pawn_moves: [[u64; N_SQUARES]; Side::N_SIDES],
    pub pawn_capture_moves: [[u64; N_SQUARES]; Side::N_SIDES],
    pub knight_moves: [u64; N_SQUARES],
    pub king_moves: [u64; N_SQUARES],

    // Blocker masks for magic bitboard, which are similar to the pseudo-legal
    // moves for a square except that the terminating edge squares are omitted
    // because edge squares always block.
    pub rook_masks: [u64; N_SQUARES],
    pub bishop_masks: [u64; N_SQUARES],
    pub queen_masks: [u64; N_SQUARES],

    // Bishop and rook rays are contained in their respective direction on the
    // board in this array.
    pub rays: [[u64; N_SQUARES]; RayDirection::N_DIRECTIONS],
    // Composite rays (i.e. all directions and including board edge) used for
    // checking alignment with king for pins and checks.
    pub comp_rays: [[u64; N_SQUARES]; Piece::N_SLIDING_PIECES],
}

impl Index<(Piece, Side, usize)> for MoveBitboards {
    type Output = u64;

    fn index(&self, (piece, side, square): (Piece, Side, usize)) -> &Self::Output {
        match piece {
            Piece::Pawn => &self.pawn_moves[side as usize][square],
            Piece::Knight => &self.knight_moves[square],
            Piece::Bishop => &self.bishop_masks[square],
            Piece::Rook => &self.rook_masks[square],
            Piece::Queen => &self.queen_masks[square],
            Piece::King => &self.king_moves[square],
        }
    }
}

impl Default for MoveBitboards {
    fn default() -> Self {
        Self {
            pawn_moves: [[0; N_SQUARES]; Side::N_SIDES],
            pawn_capture_moves: [[0; N_SQUARES]; Side::N_SIDES],
            knight_moves: [0; N_SQUARES],
            king_moves: [0; N_SQUARES],

            rook_masks: [0; N_SQUARES],
            bishop_masks: [0; N_SQUARES],
            queen_masks: [0; N_SQUARES],

            rays: [[0; N_SQUARES]; RayDirection::N_DIRECTIONS],
            comp_rays: [[0; N_SQUARES]; Piece::N_SLIDING_PIECES],
        }
    }
}

impl MoveBitboards {
    #[inline]
    pub fn get_piece_blocker_mask(&self, piece: Piece, square: usize) -> u64 {
        match piece {
            Piece::Rook => self.rook_masks[square],
            Piece::Bishop => self.bishop_masks[square],
            Piece::Queen => self.queen_masks[square],
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_comp_rays(&self, piece: Piece) -> &[u64; N_SQUARES] {
        &self.comp_rays[(piece as usize) - Piece::SLIDER_START_VALUE]
    }

    #[inline]
    fn set_comp_rays(&mut self, piece: Piece, square: usize, bb: u64) {
        self.comp_rays[(piece as usize) - Piece::SLIDER_START_VALUE][square] = bb;
    }

    pub fn init_legal_moves() -> Self {
        info!("Initialising pseudo-legal moves and ray masks");

        let mut legal_moves = MoveBitboards::default();
        for square in 0..N_SQUARES {
            let piece_file = file(square);
            let piece_rank = rank(square);

            legal_moves.init_pawn_moves(square, piece_file, piece_rank);
            legal_moves.init_knight_moves(square, piece_file, piece_rank);
            legal_moves.init_bishop_moves(square, piece_file, piece_rank);
            legal_moves.init_rook_moves(square, piece_file, piece_rank);
            legal_moves.init_queen_moves(square);
            legal_moves.init_king_moves(square, piece_file, piece_rank);
        }

        legal_moves
    }

    pub fn get_bishop_rays(&self, square: usize, blocker_mask: u64) -> u64 {
        let mut moves_bb = 0u64;

        for direction in RayDirection::BISHOP_DIRECTIONS {
            let direction_idx = direction as usize;
            let ray = self.rays[direction_idx][square];
            moves_bb |= ray;

            let blockers = ray & blocker_mask;
            if blockers != 0 {
                let nearest_blocker_square = match direction {
                    RayDirection::NorthEast | RayDirection::NorthWest =>
                        blockers.trailing_zeros() as usize,
                    RayDirection::SouthEast | RayDirection::SouthWest =>
                        (u64::BITS - 1 - blockers.leading_zeros()) as usize,
                    _ => unreachable!()
                };
                moves_bb &= !self.rays[direction_idx][nearest_blocker_square];
            }
        }

        moves_bb
    }

    pub fn get_rook_rays(&self, square: usize, blocker_mask: u64) -> u64 {
        let mut moves_bb = 0u64;

        for direction in RayDirection::ROOK_DIRECTIONS {
            let direction_idx = direction as usize;
            let ray = self.rays[direction_idx][square];
            moves_bb |= ray;

            let blockers = ray & blocker_mask;
            if blockers != 0 {
                let nearest_blocker_square = match direction {
                    RayDirection::North | RayDirection::West =>
                        blockers.trailing_zeros() as usize,
                    RayDirection::East | RayDirection::South =>
                        (u64::BITS - 1 - blockers.leading_zeros()) as usize,
                    _ => unreachable!()
                };
                moves_bb &= !self.rays[direction_idx][nearest_blocker_square];
            }
        }

        moves_bb
    }

    fn init_pawn_moves(&mut self, square: usize, file: usize, rank: usize) {
        // white pawns move forward
        if rank != BOARD_HEIGHT - 1 {
            self.pawn_moves[Side::White as usize][square] |= 1 << (square + BOARD_WIDTH);

            if rank == 1 {
                // can also move two squares if at starting rank
                self.pawn_moves[Side::White as usize][square] |= 1 << (square + 2 * BOARD_WIDTH);
            }

            // Captures
            if file != BOARD_WIDTH - 1 {
                self.pawn_capture_moves[Side::White as usize][square] |=
                    1 << (square + BOARD_WIDTH + 1);
            }

            if file != 0 {
                self.pawn_capture_moves[Side::White as usize][square] |=
                    1 << (square + BOARD_WIDTH - 1);
            }
        }

        // black pawns move "backward"
        if rank != 0 {
            self.pawn_moves[Side::Black as usize][square] |= 1 << (square - BOARD_WIDTH);

            if rank == BOARD_HEIGHT - 2 {
                // can also move two squares if at starting rank
                self.pawn_moves[Side::Black as usize][square] |= 1 << (square - 2 * BOARD_WIDTH);
            }

            // Captures
            if file != BOARD_WIDTH - 1 {
                self.pawn_capture_moves[Side::Black as usize][square] |=
                    1 << (square - BOARD_WIDTH + 1);
            }

            if file != 0 {
                self.pawn_capture_moves[Side::Black as usize][square] |=
                    1 << (square - BOARD_WIDTH - 1);
            }
        }
    }

    fn init_knight_moves(&mut self, square: usize, file: usize, rank: usize) {
        // Up and left-right
        if rank <= BOARD_WIDTH - 3 {
            if file != BOARD_WIDTH - 1 {
                self.knight_moves[square] |= 1 << (square + 2 * BOARD_WIDTH + 1);
            }

            if file != 0 {
                self.knight_moves[square] |= 1 << (square + 2 * BOARD_WIDTH - 1);
            }
        }

        // Down and left-right
        if rank >= 2 {
            if file != BOARD_WIDTH - 1 {
                self.knight_moves[square] |= 1 << (square - 2 * BOARD_WIDTH + 1);
            }

            if file != 0 {
                self.knight_moves[square] |= 1 << (square - 2 * BOARD_WIDTH - 1);
            }
        }

        // Left and up-down
        if file <= BOARD_WIDTH - 3 {
            if rank != 7 {
                self.knight_moves[square] |= 1 << (square + 2 + BOARD_WIDTH);
            }

            if rank != 0 {
                self.knight_moves[square] |= 1 << (square + 2 - BOARD_WIDTH);
            }
        }

        // Right and up-down
        if file >= 2 {
            if rank != 7 {
                self.knight_moves[square] |= 1 << (square - 2 + BOARD_WIDTH);
            }

            if rank != 0 {
                self.knight_moves[square] |= 1 << (square - 2 - BOARD_WIDTH);
            }
        }
    }

    fn init_bishop_moves(&mut self, square: usize, file: usize, rank: usize) {
        // Left diagonal
        // North-west
        if rank < BOARD_WIDTH - 1 && file < BOARD_HEIGHT - 1 {
            let mut up_left_diag_idx = square + BOARD_WIDTH + 1;
            let mut i = 1;

            while up_left_diag_idx <= BOARD_WIDTH * BOARD_HEIGHT - 1 && i < BOARD_WIDTH - file {
                self.rays[RayDirection::NorthWest as usize][square] |= 1 << up_left_diag_idx;
                up_left_diag_idx += BOARD_WIDTH + 1;
                i += 1;
            }
        }

        // South-west
        if rank > 0 && file > 0 {
            let mut down_left_diag_idx: i64 = (square as i64) - (BOARD_WIDTH as i64) - 1;
            let mut i = 1;
            while down_left_diag_idx >= 0 && i <= file {
                self.rays[RayDirection::SouthEast as usize][square] |= 1 << down_left_diag_idx;
                down_left_diag_idx -= (BOARD_WIDTH as i64) + 1;
                i += 1;
            }
        }

        // Right diagonal
        // North-east
        if rank < BOARD_WIDTH - 1 && file > 0 {
            let mut up_right_diag_idx = square + BOARD_WIDTH - 1;
            let mut i = 1;

            while up_right_diag_idx <= BOARD_WIDTH * BOARD_HEIGHT - 1 && i <= file {
                self.rays[RayDirection::NorthEast as usize][square] |= 1 << up_right_diag_idx;
                up_right_diag_idx += BOARD_WIDTH - 1;
                i += 1;
            }
        }

        // South-east
        if rank > 0 && file < BOARD_WIDTH - 1 {
            let mut down_right_diag_idx: i64 = (square as i64) - (BOARD_WIDTH as i64) + 1;
            let mut i = 1;

            while down_right_diag_idx >= 0 && i < BOARD_WIDTH - file {
                self.rays[RayDirection::SouthWest as usize][square] |= 1 << down_right_diag_idx;
                down_right_diag_idx -= (BOARD_WIDTH as i64) - 1;
                i += 1;
            }
        }

        self.bishop_masks[square] = self.rays[RayDirection::NorthWest as usize][square]
            | self.rays[RayDirection::SouthWest as usize][square]
            | self.rays[RayDirection::SouthEast as usize][square]
            | self.rays[RayDirection::NorthEast as usize][square];

        // Also save composite ray _with_ edge squares
        self.set_comp_rays(Piece::Bishop, square, self.bishop_masks[square]);

        // Remove edge squares for blocker masks
        //
        // Bit mask for removing 1s around the edge of the board for the bishop
        // blocker masks.
        //
        // "00000000"
        // "01111110"
        // "01111110"
        // "01111110"
        // "01111110"
        // "01111110"
        // "01111110"
        // "00000000"
        const EDGE_MASK: u64 = 35604928818740736;
        self.bishop_masks[square] &= EDGE_MASK;
    }

    fn init_rook_moves(&mut self, square: usize, file: usize, rank: usize) {
        // Rank moves
        // West
        for i in 1..=(BOARD_WIDTH - 1 - file) {
            self.rays[RayDirection::West as usize][square] |= 1 << square + i;
        }

        // East
        for i in 1..=file {
            self.rays[RayDirection::East as usize][square] |= 1 << square - i;
        }

        // File moves
        // North
        for i in 1..=(BOARD_HEIGHT - 1 - rank) {
            self.rays[RayDirection::North as usize][square] |= 1 << square + i * BOARD_WIDTH;
        }

        // South
        for i in 1..=rank {
            self.rays[RayDirection::South as usize][square] |= 1 << square - i * BOARD_WIDTH;
        }

        self.rook_masks[square] = self.rays[RayDirection::North as usize][square]
            | self.rays[RayDirection::South as usize][square]
            | self.rays[RayDirection::West as usize][square]
            | self.rays[RayDirection::East as usize][square];

        // Also save composite ray _with_ edge squares
        self.set_comp_rays(Piece::Rook, square, self.rook_masks[square]);

        // Remove edge squares for blocker masks
        // File edges
        self.rook_masks[square] &= !(1 << file);
        self.rook_masks[square] &= !(1 << ((BOARD_WIDTH - 1) * BOARD_WIDTH + file));

        // Rank edges
        self.rook_masks[square] &= !(1 << BOARD_WIDTH * rank + BOARD_WIDTH - 1);
        self.rook_masks[square] &= !(1 << BOARD_WIDTH * rank);
    }

    fn init_queen_moves(&mut self, square: usize) {
        // Blocker masks
        self.queen_masks[square] = self.rook_masks[square] | self.bishop_masks[square];

        // Also save composite ray _with_ edge squares
        self.set_comp_rays(Piece::Queen, square,
            self.get_comp_rays(Piece::Bishop)[square] | self.get_comp_rays(Piece::Rook)[square]);
    }

    fn init_king_moves(&mut self, square: usize, file: usize, rank: usize) {
        // Rank moves
        // Left
        if file != 7 {
            self.king_moves[square] |= 1 << (square + 1);
        }

        // Right
        if file != 0 {
            self.king_moves[square] |= 1 << (square - 1);
        }

        // File moves
        // Up
        if rank != 7 {
            self.king_moves[square] |= 1 << (square + BOARD_WIDTH);
        }

        // Down
        if rank != 0 {
            self.king_moves[square] |= 1 << (square - BOARD_WIDTH);
        }

        // Diagonal moves
        // Up-Left
        if file != 7 && rank != 7 {
            self.king_moves[square] |= 1 << (square + BOARD_WIDTH + 1);
        }

        // Up-Right
        if file != 0 && rank != 7 {
            self.king_moves[square] |= 1 << (square + BOARD_WIDTH - 1);
        }

        // Down-Left
        if file != 7 && rank != 0 {
            self.king_moves[square] |= 1 << (square - BOARD_WIDTH + 1);
        }

        // Down-Right
        if file != 0 && rank != 0 {
            self.king_moves[square] |= 1 << (square - BOARD_WIDTH - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moves() {
        let legal_moves = MoveBitboards::init_legal_moves();

        // king moves
        assert_eq!(legal_moves.king_moves[0], 770);
        assert_eq!(legal_moves.king_moves[7], 49216);
        assert_eq!(legal_moves.king_moves[35], 30872694685696);

        // knight moves
        assert_eq!(legal_moves.knight_moves[0], 132096);
        assert_eq!(legal_moves.knight_moves[36], 11333767002587136);
        assert_eq!(legal_moves.knight_moves[23], 275414786112);
        assert_eq!(legal_moves.knight_moves[60], 19184278881435648);

        // pawn moves
        assert_eq!(legal_moves.pawn_moves[Side::White as usize][8], 16842752);
        assert_eq!(legal_moves.pawn_moves[Side::White as usize][23], 2147483648);
        assert_eq!(
            legal_moves.pawn_moves[Side::White as usize][51],
            576460752303423488
        );
        assert_eq!(
            legal_moves.pawn_moves[Side::White as usize][40],
            281474976710656
        );
        assert_eq!(legal_moves.pawn_moves[Side::Black as usize][40], 4294967296);
        assert_eq!(
            legal_moves.pawn_moves[Side::Black as usize][50],
            4415226380288
        );
        assert_eq!(legal_moves.pawn_moves[Side::Black as usize][10], 4);

        assert_eq!(
            legal_moves.pawn_capture_moves[Side::White as usize][8],
            131072
        );
        assert_eq!(
            legal_moves.pawn_capture_moves[Side::White as usize][9],
            327680
        );
        assert_eq!(legal_moves.pawn_capture_moves[Side::Black as usize][8], 2);
        assert_eq!(legal_moves.pawn_capture_moves[Side::Black as usize][9], 5);

        // blocker masks
        // rook
        assert_eq!(legal_moves.rook_masks[28], 4521262379438080);
        assert_eq!(legal_moves.rook_masks[0], 282578800148862);
        assert_eq!(legal_moves.rook_masks[63], 9115426935197958144);

        // bishop
        assert_eq!(legal_moves.bishop_masks[43], 5629586008178688);
        assert_eq!(legal_moves.bishop_masks[2], 275415828992);

        // queen
        assert_eq!(legal_moves.queen_masks[36], 23705944086286848);

        // Rays
        assert_eq!(
            legal_moves.rays[RayDirection::North as usize][10],
            289360691352305664
        );
        assert_eq!(legal_moves.rays[RayDirection::South as usize][10], 4);
        assert_eq!(legal_moves.rays[RayDirection::East as usize][10], 768);
        assert_eq!(legal_moves.rays[RayDirection::West as usize][10], 63488);

        assert_eq!(
            legal_moves.rays[RayDirection::NorthEast as usize][10],
            16908288
        );
        assert_eq!(legal_moves.rays[RayDirection::SouthEast as usize][10], 2);
        assert_eq!(legal_moves.rays[RayDirection::SouthWest as usize][10], 8);
        assert_eq!(
            legal_moves.rays[RayDirection::NorthWest as usize][10],
            36099303471054848
        );
    }
}
