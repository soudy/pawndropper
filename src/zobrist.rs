use crate::board::{N_SQUARES, Board, Piece, Side, BOARD_WIDTH};
use crate::move_bitboards::file;

use rand::Rng;

#[derive(Debug, Clone)]
pub struct ZobristHasher {
    rands: [[[u64; N_SQUARES]; Piece::N_PIECES]; Side::N_SIDES],
    black_to_move_rand: u64,
    castling_right_long_rands: [u64; 4],
    castling_right_short_rands: [u64; 4],
    ep_file_rands: [u64; BOARD_WIDTH],
}

impl ZobristHasher {
    pub fn new() -> Self {
        let mut hash_instance = Self {
            rands: [[[0u64; N_SQUARES]; Piece::N_PIECES]; Side::N_SIDES],
            black_to_move_rand: 0,
            castling_right_long_rands: [0u64; 4],
            castling_right_short_rands: [0u64; 4],
            ep_file_rands: [0u64; BOARD_WIDTH]
        };

        let mut rng = rand::thread_rng();

        for side in Side::VALUES {
            for piece in Piece::VALUES {
                for i in 0..N_SQUARES {
                    hash_instance.rands[side as usize][piece as usize][i] = rng.gen::<u64>();
                }
            }
        }

        hash_instance.black_to_move_rand = rng.gen::<u64>();

        for i in 0..4 {
            hash_instance.castling_right_long_rands[i] = rng.gen::<u64>();
            hash_instance.castling_right_short_rands[i] = rng.gen::<u64>();
        }

        for i in 0..BOARD_WIDTH {
            hash_instance.ep_file_rands[i] = rng.gen::<u64>();
        }

        hash_instance
    }

    pub fn hash(&self, board: &Board) -> u64 {
        let mut hash = 0u64;

        // Hash side to play
        if board.side_to_move == Side::Black {
            hash ^= self.black_to_move_rand;
        }

        // Hash pieces
        for side in Side::VALUES {
            for piece in Piece::VALUES {
                let mut piece_bb = board[(piece, side)];
                while piece_bb != 0 {
                    let square = piece_bb.trailing_zeros() as usize;

                    hash ^= self.rands[side as usize][piece as usize][square];

                    piece_bb &= piece_bb - (1 << square);
                }
            }
        }

        // Hash castling rights
        let long_castle_rights = board.castling_right_long[0] as usize + board.castling_right_long[1] as usize;
        let short_castle_rights = board.castling_right_short[0] as usize + board.castling_right_short[1] as usize;
        hash ^= self.castling_right_long_rands[long_castle_rights];
        hash ^= self.castling_right_short_rands[short_castle_rights];

        if board.en_passant_square != 0 {
            hash ^= self.ep_file_rands[file(board.en_passant_square)];
        }

        hash
    }
}
