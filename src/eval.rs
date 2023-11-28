use crate::game::GameState;
use crate::board::{Piece, Side, N_SQUARES, BOARD_WIDTH, BOARD_HEIGHT};
use crate::move_bitboards::{rank, file, FILE_MASKS};

// Middlegame evaluations
const PIECES_VALUES_MG: [i32; Piece::N_PIECES] = [
    95, // Pawn
    310, // Knight
    330, // Bishop
    490, // Rook
    920, // Queen
    0, // King
];
pub const PIECE_PLACEMENT_VALUES_MG: [[i32; N_SQUARES]; Piece::N_PIECES] = [
    // Pawn
    [
        0,    0,   0,  0,  0,  0,  0,  0,
        40,    40,   40,  40,  50,  40,  40,  40,
        20,    10,   27, 35, 40,  27,  10,  20,
        10,  5,  25,  30,  30, 25,  5, 10,
        5,    10,   10, 25, 35,  5,  5,  5,
        5, 5,  5,  20,  30,  5, 5, 5,
        5,   -10,   10, -30, -30,  10,  10,  5,
        0,    0,   0,  0,  0,  0, 0,  0,
    ],
    // Knight
    [
        -40,  -20,  -10,  -10,  -10, -10, -20, -40,
        -20,  -10,    -5, -5,  -5, -5, -10, -20,
        -10,  0,   15,   25,   25,  15,  0, -10,
        -10,  5,   25,   32,   32,  25,  5, -10,
        -10,  0,   15,   15,   15,  15,  0, -10,
        -10,  5,   10,   15,   15,  10,  5, -10,
        -20,  -10,   0,  5,  5,  0, -10, -20,
        -40,  -20,  -30,  -30,  -30, -30, -20, -40,
    ],
    // Bishop
    [
        -20,    0,    0,    0,    0,    0,    0,  -20,
        -15,    0,    0,    0,    0,    0,    0,  -15,
        -10,    0,    5,    5,    5,    5,    0,  -10,
        -10,   10,   10,   30,   30,   10,   10,  -10,
          5,    10,   10,   25,   25,   10,    10,    5,
          5,    10,    10,   10,   10,    10,    10,    5,
        -10,    5,    5,   10,   10,    5,    5,  -10,
        -20,  -10,  -10,  -10,  -10,  -10,  -10,  -20
    ],
    // Rook
    [
        0,   0,   0,   0,   0,   0,   0,   0,
        15,  20,  20,  20,  20,  20,  20,  15,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,  10,  10,  10,   0,   0
    ],
    // Queen
    [
        -30,  -20,  -10,  -10,  -10,  -10,  -20,  -30,
        -20,  -10,   -5,   -5,   -5,   -5,  -10,  -20,
        -10,   -5,   10,   10,   10,   10,   -5,  -10,
        -10,   -5,   10,   20,   20,   10,   -5,  -10,
        -10,   -5,   10,   20,   20,   10,   -5,  -10,
        -10,   -5,   -5,   -5,   -5,   -5,   -5,  -10,
        -20,  -10,   -5,   -5,   -5,   -5,  -10,  -20,
        -30,  -20,  -10,  -10,  -10,  -10,  -20,  -30
    ],
    // King
    [
        -30, -40, -40, -50, -50, -40, -40, -30,
        -30, -40, -40, -50, -50, -40, -40, -30,
        -30, -40, -40, -50, -50, -40, -40, -30,
        -30, -40, -40, -50, -50, -40, -40, -30,
        -20, -30, -30, -40, -40, -30, -30, -20,
        -10, -20, -20, -30, -30, -20, -20, -10,
        15, 15,  0, -10, -10,  0,  15, 15,
        20,   40,  5,  -5,  -5,  5,   40, 20,
    ]
];

// Endgame evaluations
const PIECES_VALUES_EG: [i32; Piece::N_PIECES] = [
    110, // Pawn
    260, // Knight
    280, // Bishop
    490, // Rook
    890, // Queen
    0, // King
];
pub const PIECE_PLACEMENT_VALUES_EG: [[i32; N_SQUARES]; Piece::N_PIECES] = [
    // Pawn
    [
        0,    0,   0,  0,  0,  0,  0,  0,
        100,    100,   100,  100,  110,  100,  100,  100,
        70,    70,   80, 80, 85,  80,  70,  70,
        30,  15,  30,  40,  40, 30,  15, 30,
        5,    10,   10, 25, 35,  10,  5,  5,
        5, 5,  5,  20,  30,  5, 5, 5,
        5,   -10,   10, -30, -30,  10,  10,  5,
        0,    0,   0,  0,  0,  0, 0,  0,
    ],
    // Knight
    [
        -20,  -5,  -1,  -1,  -1, -1, -5, -20,
        -10,  -10,    -5, -5,  -5, -5, -10, -21,
        -5,  0,   10,   20,   20,  10,  0, -5,
        -5,  5,   20,   22,   22,  20,  5, -5,
        -5,  0,   15,   15,   15,  15,  0, -5,
        -5,  5,   10,   15,   15,  10,  5, -5,
        -10,  -10,   0,  5,  5,  0, -10, -10,
        -20,  -5,  -10,  -10,  -10, -10, -5, -20,
    ],
    // Bishop
    [
        -10,    0,    0,    0,    0,    0,    0,  -10,
        -5,    0,    0,    0,    0,    0,    0,  -5,
        -5,    0,    5,    5,    5,    5,    0,  -5,
        -5,   5,   5,   10,   10,   5,   5,  -5,
          5,    5,   5,   15,   15,   5,    5,    5,
          5,    5,    5,   7,   7,    5,    5,    5,
        -10,    5,    5,   7,   7,    5,    5,  -10,
        -10,  -5,  -5,  -5,  -5,  -5,  -5,  -10
    ],
    // Rook
    [
        10,   10,   10,   10,   10,   10,   10,   10,
        15,  20,  20,  20,  20,  20,  20,  15,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,  0,  0,  0,   0,   0
    ],
    // Queen
    [
        -30,  -20,  -10,  -10,  -10,  -10,  -20,  -30,
        -20,  -10,   -5,   -5,   -5,   -5,  -10,  -20,
        -10,   -5,   10,   10,   10,   10,   -5,  -10,
        -10,   -5,   10,   20,   20,   10,   -5,  -10,
        -10,   -5,   10,   20,   20,   10,   -5,  -10,
        -10,   -5,   -5,   -5,   -5,   -5,   -5,  -10,
        -20,  -10,   -5,   -5,   -5,   -5,  -10,  -20,
        -30,  -20,  -10,  -10,  -10,  -10,  -20,  -30
    ],
    // King
    [
        -50, -40, -30, -20, -20, -30, -40, -50,
        -30, -20, -10, 0, 0, -10, -20, -30,
        -30, -10, 20, 30, 30, 20, -10, -30,
        -30, -10, 30, 40, 40, 30, -10, -30,
        -30, -10, 30, 40, 40, 30, -10, -30,
        -30, -10, 20, 30, 30, 20, -10, -30,
        -30, -30,  0, 0, 0,  0,  -30, -30,
        -50, -30, -30, -30, -30, -30,  -30, -50,
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

const BISHOP_PAIR_BONUS_MG: i32 = 35;
const BISHOP_PAIR_BONUS_EG: i32 = 50;

const DOUBLED_PAWNS_PENALTY_MG: i32 = -2;
const DOUBLED_PAWNS_PENALTY_EG: i32 = -10;

pub fn eval(game: &GameState) -> f64 {
    let mut piece_counts = [[0i32; Piece::N_PIECES]; Side::N_SIDES];
    let mut doubled_pawns = [0i32; Side::N_SIDES];
    let mut phase = TOTAL_PHASE;
    let mut mg_eval = 0i32;
    let mut eg_eval = 0i32;
    let mut side_mg_eval;
    let mut side_eg_eval;
    const MULTIPLIERS: [i32; 2] = [1, -1]; // multiply by -1 for black's piece evaluation

    for side in Side::VALUES {
        let side_idx = side as usize;
        let multiplier = MULTIPLIERS[side as usize];

        side_mg_eval = 0;
        side_eg_eval = 0;

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

                side_mg_eval += 
                    // intrinsic piece value
                    PIECES_VALUES_MG[piece_idx]
                    // absolute piece placement value
                    + PIECE_PLACEMENT_VALUES_MG[piece_idx][corrected_square];
                side_eg_eval += 
                    PIECES_VALUES_EG[piece_idx]
                    + PIECE_PLACEMENT_VALUES_EG[piece_idx][corrected_square];

                // Count the fraction of squares that a knight or slider piece 
                // covers compared to the maximum amount of possible coverage
                // for a given square
                let frac_ray_occupied = if piece.is_slider() {
                    let moves_bb = game.slider_moves(piece, square);
                    let all_rays = game.pl_moves.get_comp_rays(piece)[square];
                    let ray_occupied = moves_bb & all_rays;
                    ((ray_occupied.count_ones() as f64)/(all_rays.count_ones() as f64)) as i32 * 100
                } else if piece == Piece::Knight {
                    let moves_bb = game.pl_moves[(piece, side, square)];
                    (moves_bb.count_ones() as f64 / 8.0) as i32 * 100
                } else {
                    0
                };

                
                side_mg_eval += frac_ray_occupied/4;
                side_eg_eval += frac_ray_occupied/10;

                piece_counts[side_idx][piece_idx] += 1;

                phase -= PHASES[piece_idx];

                // clear square bit
                piece_bb &= piece_bb - (1 << one_pos);
            }
        }

        // Bishop pair bonus
        if piece_counts[side_idx][Piece::Bishop as usize] >= 2 {
            side_mg_eval += BISHOP_PAIR_BONUS_MG;
            side_eg_eval += BISHOP_PAIR_BONUS_EG;
        }

        // Doubled pawns
        for file in FILE_MASKS {
            let pawns_on_file = (file & game.board[(Piece::Pawn, side)]).count_ones() as i32;
            if pawns_on_file > 1 {
                doubled_pawns[side_idx] += pawns_on_file - 1;
            }
        }

        side_mg_eval += doubled_pawns[side as usize]*DOUBLED_PAWNS_PENALTY_MG;
        side_eg_eval += doubled_pawns[side as usize]*DOUBLED_PAWNS_PENALTY_EG;

        mg_eval += multiplier*side_mg_eval;
        eg_eval += multiplier*side_eg_eval;
    }

    phase = (phase*256.0 + (TOTAL_PHASE / 2.0))/TOTAL_PHASE;
    (((mg_eval as f64)*(256.0 - phase) + (eg_eval as f64)*phase)/256.0)/100.0
}
