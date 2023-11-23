use crate::game::GameState;
use crate::board::{Piece, Side, N_SQUARES, BOARD_WIDTH, BOARD_HEIGHT};
use crate::move_bitboards::{rank, file, FILE_MASKS};

// Middlegame evaluations
const PIECES_VALUES_MG: [f64; Piece::N_PIECES] = [
    0.8, // Pawn
    3.1, // Knight
    3.2, // Bishop
    5.1, // Rook
    9.2, // Queen
    0.0, // King
];
pub const PIECE_PLACEMENT_VALUES_MG: [[f64; N_SQUARES]; Piece::N_PIECES] = [
    // Pawn
    [
        0.0,    0.0,   0.0,  0.0,  0.0,  0.0,  0.0,  0.0,
        0.4,    0.4,   0.4,  0.4,  0.4,  0.4,  0.4,  0.4,
        0.1,    0.1,   0.2, 0.35, 0.35,  0.2,  0.1,  0.1,
        0.05,  0.05,  0.25,  0.3,  0.3, 0.25,  0.05, 0.05,
        0.0,    0.1,   0.1, 0.25, 0.25,  0.0,  0.0,  0.0,
        0.05, -0.05,  -0.1,  0.0,  0.0, -0.1, -0.05, 0.05,
        0.05,   -0.1,   0.1, -0.3, -0.3,  0.1,  0.1,  0.05,
        0.0,    -0.1,   0.0,  0.0,  0.0,  0.0,  -0.1,  0.0,
    ],
    // Knight
    [
        -0.5,  -0.3,  -0.3,  -0.3,  -0.3, -0.3, -0.3, -0.5,
        -0.3,  -0.2,   0.0,   0.0,   0.0,  0.0, -0.2, -0.3,
        -0.3,   0.0,   0.1,   0.2,   0.2,  0.1,  0.0, -0.3,
        -0.3,   0.0,   0.2,   0.25,   0.25,  0.2,  0.0, -0.3,
        -0.3,   0.0,   0.2,   0.25,   0.25,  0.2,  0.0, -0.3,
        -0.3,   0.0,   0.1,   0.2,   0.2,  0.1,  0.0, -0.3,
        -0.3,  -0.2,   0.0,  0.05,  0.05,  0.0, -0.2, -0.3,
        -0.5,  -0.1,  0.0,  -0.3,  -0.3, 0.0, -0.1, -0.5,
    ],
    // Bishop
    [
        -0.2, -0.1, -0.1, -0.1, -0.1, -0.1, -0.1, -0.2,
        -0.1,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0, -0.1,
        -0.1,  0.0, 0.05,  0.1,  0.1, 0.05,  0.0, -0.1,
        -0.1, 0.05, 0.05,  0.1,  0.1, 0.05, 0.05, -0.1,
        -0.1,  0.0,  0.1,  0.1,  0.1,  0.1,  0.0, -0.1,
        -0.1,  0.1,  0.1,  0.1,  0.1,  0.1,  0.1, -0.1,
        -0.1,  0.05,  0.0,  0.0,  0.0,  0.0,  0.05, -0.1,
        -0.2, -0.1, -0.1, -0.1, -0.1, -0.1, -0.1, -0.2,
    ],
    // Rook
    [
        0.0,   0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.05,  0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        0.0,   0.0, 0.0, 0.15, 0.15, 0.1, 0.0, 0.0,
    ],
    // Queen
    [
        -0.2, -0.1, -0.1, -0.02, -0.02, -0.1, -0.1, -0.2,
        -0.1,  0.0,  0.0,   0.0,   0.0,  0.0,  0.0, -0.1,
        -0.05, 0.0,  0.0,   0.0,   0.0,  0.0,  0.0, -0.05,
        0.0,   0.0,  0.0,   -0.1,   -0.1,  0.0,  0.0, 0.0,
        0.0,   0.0,  0.0,   -0.1,   -0.1,  0.0,  0.0, 0.0,
        -0.05, 0.0,  -0.1,   0.0,   0.0,  -0.1,  0.0, -0.05,
        -0.1,  0.0,  0.0,   0.0,   0.0,  0.0,  0.0, -0.1,
        -0.2,  0.0,  0.0,   0.1,   0.0,  0.0,  0.0, -0.2,
    ],
    // King
    [
        -0.3, -0.4, -0.4, -0.5, -0.5, -0.4, -0.4, -0.3,
        -0.3, -0.4, -0.4, -0.5, -0.5, -0.4, -0.4, -0.3,
        -0.3, -0.4, -0.4, -0.5, -0.5, -0.4, -0.4, -0.3,
        -0.3, -0.4, -0.4, -0.5, -0.5, -0.4, -0.4, -0.3,
        -0.2, -0.3, -0.3, -0.4, -0.4, -0.3, -0.3, -0.2,
        -0.1, -0.2, -0.2, -0.3, -0.3, -0.2, -0.2, -0.1,
        0.15, 0.15,  0.0, -0.1, -0.1,  0.0,  0.15, 0.15,
        0.2,   0.25,  0.1,  0.0,  0.0,  0.1,   0.25, 0.2,
    ]
];

// Endgame evaluations
const PIECES_VALUES_EG: [f64; Piece::N_PIECES] = [
    1.1, // Pawn
    2.4, // Knight
    2.6, // Bishop
    5.2, // Rook
    8.9, // Queen
    0.0, // King
];
pub const PIECE_PLACEMENT_VALUES_EG: [[f64; N_SQUARES]; Piece::N_PIECES] = [
    // Pawn
    [
        0.0,    0.0,   0.0,  0.0,  0.0,  0.0,  0.0,  0.0,
        1.0,    1.0,   1.0,  1.0,  1.0,  1.0,  1.0,  1.0,
        0.6,    0.6,   0.6,  0.7,  0.7,  0.6,  0.6,  0.6,
        0.4,  0.05,  0.25,  0.4,  0.4, 0.25,  0.05, 0.4,
        0.3,    0.2,   0.2, 0.3, 0.3,  0.0,  0.0,  0.3,
        0.05,   0.1,   0.1,  0.0,  0.0,  0.1,  0.1, 0.05,
        0.05,   0.1,   0.1, -0.2, -0.2,  0.1,  0.1,  0.05,
        0.0,    0.0,   0.0,  0.0,  0.0,  0.0,  0.0,  0.0,
    ],
    // Knight
    [
        -0.5,  -0.3,  -0.3,  -0.3,  -0.3, -0.3, -0.3, -0.5,
        -0.3,  -0.2,   0.0,   0.0,   0.0,  0.0, -0.2, -0.3,
        -0.3,   0.0,   0.1,   0.2,   0.2,  0.1,  0.0, -0.3,
        -0.3,   0.0,   0.2,   0.25,   0.25,  0.2,  0.0, -0.3,
        -0.3,   0.0,   0.2,   0.25,   0.25,  0.2,  0.0, -0.3,
        -0.3,   0.0,   0.1,   0.2,   0.2,  0.1,  0.0, -0.3,
        -0.3,  -0.2,   0.0,  0.05,  0.05,  0.0, -0.2, -0.3,
        -0.5,  -0.1,  0.0,  -0.3,  -0.3, 0.0, -0.1, -0.5,
    ],
    // Bishop
    [
        -0.2, -0.1, -0.1, -0.1, -0.1, -0.1, -0.1, -0.2,
        -0.1,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0, -0.1,
        -0.1,  0.0, 0.05,  0.1,  0.1, 0.05,  0.0, -0.1,
        -0.1, 0.05, 0.05,  0.1,  0.1, 0.05, 0.05, -0.1,
        -0.1,  0.0,  0.1,  0.1,  0.1,  0.1,  0.0, -0.1,
        -0.1,  0.1,  0.1,  0.1,  0.1,  0.1,  0.1, -0.1,
        -0.1,  0.3,  0.0,  0.0,  0.0,  0.0,  0.3, -0.1,
        -0.2, -0.1, -0.1, -0.1, -0.1, -0.1, -0.1, -0.2,
    ],
    // Rook
    [
        0.05,   0.4, 0.4, 0.4, 0.4, 0.04, 0.4, 0.05,
        0.1,  0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        -0.05, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.05,
        0.0,   0.0, 0.0, 0.5, 0.5, 0.2, 0.0, 0.0,
    ],
    // Queen
    [
        -0.2,  0.2,  0.2,   0.2,  0.2,  0.1, 0.1, 0.2,
        -0.1,  0.2,  0.3,   0.4,   0.5,  0.2,  0.3, 0.0,
        -0.05, 0.1,  0.1,   0.4,   0.4,  0.1,  0.1, 0.05,
        0.0,   0.2,  0.3,   0.5,   0.5,  0.3,  0.1, 0.0,
        0.0,   0.2,  0.3,   0.5,   0.5,  0.3,  0.1, 0.0,
        -0.05, 0.2,  0.3,   0.3,   0.1,  0.3,  0.1, -0.05,
        -0.1,  0.0,  0.2,   0.0,   0.0,  0.1,  0.1, -0.1,
        -0.2, -0.1, -0.1, -0.05, -0.05, -0.1, -0.1, -0.2,
    ],
    // King
    [
        -0.5, -0.4, -0.3, -0.2, -0.2, -0.3, -0.4, -0.5,
        -0.3, -0.2, -0.1, 0.0, 0.0, -0.1, -0.2, -0.3,
        -0.3, -0.1, 0.2, 0.3, 0.3, 0.2, -0.1, -0.3,
        -0.3, -0.1, 0.3, 0.4, 0.4, 0.3, -0.1, -0.3,
        -0.3, -0.1, 0.3, 0.4, 0.4, 0.3, -0.1, -0.3,
        -0.3, -0.1, 0.2, 0.3, 0.3, 0.2, -0.1, -0.3,
        -0.3, -0.3,  0.0, 0.0, 0.0,  0.0,  -0.3, -0.3,
        -0.5, -0.3, -0.3, -0.3, -0.3, -0.3,  -0.3, -0.5,
    ]
];

const PHASES: [f64; 6] = [
    0.0, // Pawn
    1.0, // Knight phase
    1.0, // Bishop phase
    2.0, // Rook phase
    4.0, // Queen phase
    0.0 // unused
];
const TOTAL_PHASE: f64 = 16.0*PHASES[0] + 4.0*(PHASES[1] + PHASES[2] + PHASES[3]) + 2.0*PHASES[4];

const BISHOP_PAIR_BONUS_MG: f64 = 0.35;
const BISHOP_PAIR_BONUS_EG: f64 = 0.5;

const DOUBLED_PAWNS_PENALTY_MG: f64 = -0.01;
const DOUBLED_PAWNS_PENALTY_EG: f64 = -0.1;

pub fn eval(game: &GameState) -> f64 {
    let mut piece_counts = [[0usize; Piece::N_PIECES]; Side::N_SIDES];
    let mut doubled_pawns = [0usize; Side::N_SIDES];
    let mut phase = TOTAL_PHASE;
    let mut mg_eval = 0.0;
    let mut eg_eval = 0.0;
    const MULTIPLIERS: [f64; 2] = [1.0, -1.0]; // multiply by -1 for black's piece evaluation

    for side in Side::VALUES {
        let side_idx = side as usize;
        let multiplier = MULTIPLIERS[side as usize];

        for piece in Piece::VALUES {
            let piece_idx = piece as usize;
            let mut piece_bb = game.board[(piece, side)];
            while piece_bb != 0 {
                let one_pos = piece_bb.trailing_zeros() as usize;
                let square = u64::BITS as usize - 1 - one_pos;
                let mut corrected_square = square;
                if side == Side::Black {
                    // Piece placement values arrays above are from white's side,
                    // flip the rank for black
                    let flipped_square_rank = BOARD_HEIGHT - 1 - rank(square);
                    let square_file = file(square);
                    corrected_square = BOARD_WIDTH*flipped_square_rank + square_file;
                }

                mg_eval += multiplier*(
                    // intrinsic piece value
                    PIECES_VALUES_MG[piece_idx]
                    // absolute piece placement value
                    + PIECE_PLACEMENT_VALUES_MG[piece_idx][corrected_square]
                );
                eg_eval += multiplier*(
                    PIECES_VALUES_EG[piece_idx]
                    + PIECE_PLACEMENT_VALUES_EG[piece_idx][corrected_square]
                );

                // Count the fraction of squares that a slider covers compared to
                // the maximum amount of possible coverage for a given square
                if piece.is_slider() {
                    let moves_bb = game.slider_moves(piece, square);
                    let all_rays = game.pl_moves.get_comp_rays(piece)[square];
                    let ray_occupied = moves_bb & all_rays;
                    let frac_ray_occupied = (ray_occupied.count_ones() as f64)/(all_rays.count_ones() as f64);
                    
                    mg_eval += multiplier*frac_ray_occupied*0.7;
                    eg_eval += multiplier*frac_ray_occupied*0.8;
                }

                piece_counts[side_idx][piece_idx] += 1;

                phase -= PHASES[piece_idx];

                // clear square bit
                piece_bb &= piece_bb - (1 << one_pos);
            }
        }

        // Bishop pair bonus
        if piece_counts[side_idx][Piece::Bishop as usize] >= 2 {
            mg_eval += multiplier*BISHOP_PAIR_BONUS_MG;
            eg_eval += multiplier*BISHOP_PAIR_BONUS_EG;
        }

        // Doubled pawns
        for file in FILE_MASKS {
            let pawns_on_file = (file & game.board[(Piece::Pawn, side)]).count_ones() as usize;
            if pawns_on_file > 1 {
                doubled_pawns[side_idx] += pawns_on_file - 1;
            }
        }

        mg_eval += multiplier*(doubled_pawns[side as usize] as f64)*DOUBLED_PAWNS_PENALTY_MG;
        eg_eval += multiplier*(doubled_pawns[side as usize] as f64)*DOUBLED_PAWNS_PENALTY_EG;
    }

    phase = (phase*256.0 + (TOTAL_PHASE / 2.0))/TOTAL_PHASE;
    (mg_eval*(256.0 - phase) + eg_eval*phase)/256.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flip() {
        let square = 63 - 27 as usize;

        assert_eq!(PIECE_PLACEMENT_VALUES_MG[Piece::Pawn as usize][square], 0.25);


        let square = 63 - 35 as usize;

        let flipped_square_rank = BOARD_HEIGHT - 1 - rank(square);
        let square_file = file(square);
        let corrected_square = BOARD_WIDTH*flipped_square_rank + square_file;

        assert_eq!(PIECE_PLACEMENT_VALUES_MG[Piece::Pawn as usize][corrected_square], 0.25);

        let square = 63 - 36 as usize;

        let flipped_square_rank = BOARD_HEIGHT - 1 - rank(square);
        let square_file = file(square);
        let corrected_square = BOARD_WIDTH*flipped_square_rank + square_file;

        assert_eq!(PIECE_PLACEMENT_VALUES_MG[Piece::Pawn as usize][corrected_square], 0.25);
    }
}
