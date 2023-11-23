use std::collections::HashMap;

use crate::board::{Board, Piece, Side, BOARD_HEIGHT, BOARD_WIDTH, N_SQUARES};
use crate::magic::MagicBitboard;
use crate::move_bitboards::{file, rank, MoveBitboards, RayDirection};
use crate::r#move::{Move, MoveType, MoveResult, DrawReason};
use crate::zobrist::ZobristHasher;

/// GameState holds the state of a game on a turn.
///
/// * `pl_moves`: Pseudo-legal move bitboards for all pieces and all squares
/// * `magics`: Magic bitboards used for looking up slider rays w.r.t blockers
/// * `board`: Pieces bitboards of current turn
/// * `occupation_boards`: Sum (OR) of all piece bitboards of each side
/// * `comp_occupation_board`: Sum (OR) of both occupation boards
/// * `side_to_move`: What side to move
/// * `castling_right_long`: Long castling rights for both sides
/// * `castling_right_short`: Short castling rights for both sides
/// * `en_passant_square`: Tracks possibl en passant capture square
/// * `half_move_number`: Number of half moves, i.e. the sum of black and white moves
/// * `move_number`: Current move number, i.e. the number of black and white moves
/// * `half_move_of_last_capture`: Half move number at which last capture occured
#[derive(Clone)]
pub struct GameState<'a> {
    pub pl_moves: &'a MoveBitboards,
    magics: &'a MagicBitboard,

    pub board: Board,
    occupation_boards: [u64; Side::N_SIDES],
    comp_occupation_board: u64,

    pub half_move_number: usize,
    pub move_number: usize,
    pub half_move_of_last_capture: usize,
    pub threefold_repetition: bool,

    pub pos_hash: u64,
    position_occurance_counter: HashMap<u64, usize>,
    zobrist_hasher: ZobristHasher
}

impl<'a> GameState<'a> {
    const POSITION_OCCURANCE_CAPACITY: usize = 100;
    const MOVES_RESERVE_SIZE: usize = 64;

    pub fn new(pl_moves: &'a MoveBitboards, magics: &'a MagicBitboard) -> Self {
        let mut gs = Self {
            pl_moves,
            magics,

            board: Board::default(),
            occupation_boards: [0u64; Side::N_SIDES],
            comp_occupation_board: 0u64,
            half_move_number: 1,
            move_number: 1,
            half_move_of_last_capture: 0,
            threefold_repetition: false,

            pos_hash: 0u64,
            position_occurance_counter: HashMap::with_capacity(Self::POSITION_OCCURANCE_CAPACITY),
            zobrist_hasher: ZobristHasher::new(),
        };

        gs.update_occupation_boards();

        gs
    }

    pub fn from_board(pl_moves: &'a MoveBitboards, magics: &'a MagicBitboard, board: Board) -> Self {
        let mut gs = Self {
            pl_moves,
            magics,

            board,
            occupation_boards: [0u64; Side::N_SIDES],
            comp_occupation_board: 0u64,

            half_move_number: 1,
            move_number: 1,
            half_move_of_last_capture: 0,
            threefold_repetition: false,

            pos_hash: 0u64,
            position_occurance_counter: HashMap::with_capacity(Self::POSITION_OCCURANCE_CAPACITY),
            zobrist_hasher: ZobristHasher::new(),
        };

        gs.update_occupation_boards();

        gs
    }

    pub fn make_move(&mut self, m: &Move) -> (Option<MoveResult>, Vec<Move>) {
        self.update_board_with_move(m);

        // Moves for other side, to be returned and passed to caller of this function
        // after checking the result of the previous move (e.g. check, checkmate, draw)
        let (legal_moves_opposite, in_check) = self.get_legal_moves();
        let move_result = self.get_move_result(&legal_moves_opposite, in_check);

        (move_result, legal_moves_opposite)
    }

    pub fn update_board_with_move(&mut self, m: &Move) {
        self.board.make_move(m);

        self.update_occupation_boards();

        // Track move number
        self.half_move_number += 1;
        if self.half_move_number % 2 != 0 {
            self.move_number += 1;
        }

        // Track last capture for fifty move rule
        if m.is_capture() {
            self.half_move_of_last_capture = self.half_move_number;
        }

        // Other side to move
        self.board.side_to_move = self.board.side_to_move.opposite();

        // Update position occurance counter to track three-fold repetition
        self.pos_hash = self.zobrist_hasher.hash(&self.board);
        let n_occurances = self.position_occurance_counter.entry(self.pos_hash)
            .and_modify(|c| *c += 1)
            .or_insert(1);

        if *n_occurances == 3 {
            self.threefold_repetition = true;
        }
    }

    pub fn update_board_undo_move(
        &mut self,
        m: &Move,
        castling_right_long: &[bool; Side::N_SIDES],
        castling_right_short: &[bool; Side::N_SIDES],
        half_move_of_last_capture: usize
    ) {
        self.board.undo_move(m, castling_right_long, castling_right_short);

        self.update_occupation_boards();

        // Restore move number
        self.half_move_number -= 1;
        if self.half_move_number % 2 == 0 {
            self.move_number -= 1;
        }

        // Restore last capture for fifty move rule
        self.half_move_of_last_capture = half_move_of_last_capture;

        // Recalculate hash for previous position after decreasing position counter
        // for the position resulting from the played move
        self.position_occurance_counter.entry(self.pos_hash)
            .and_modify(|c| *c -= 1 );
        self.pos_hash = self.zobrist_hasher.hash(&self.board);

        self.threefold_repetition = false;
    }

    pub fn get_move_result(&self, legal_moves_opposite: &Vec<Move>, in_check: bool) -> Option<MoveResult> {
        let has_legal_moves = legal_moves_opposite.len() != 0;

        let moves_since_last_capture =
            (self.half_move_number - self.half_move_of_last_capture) / 2; // floor division

        // TODO: check for draw by:
        // - insufficient material:
        //   - king vs king
        //   - king and bishop vs king
        //   - king and knight vs king
        //   - king and bishop vs king and bishop (same color bishop)
        if moves_since_last_capture == 50 {
            Some(MoveResult::Draw(DrawReason::FiftyMoveRule))
        } else if self.threefold_repetition {
            Some(MoveResult::Draw(DrawReason::ThreeFoldRepetition))
        } else if in_check {
            if !has_legal_moves {
                Some(MoveResult::Checkmate)
            } else {
                Some(MoveResult::Check)
            }
        } else if !has_legal_moves {
            Some(MoveResult::Draw(DrawReason::Stalemate))
        } else {
            None
        }
    }

    /// Get a list of legal moves for the side who's to play.
    pub fn get_legal_moves(&self) -> (Vec<Move>, bool) {
        let mut move_list: Vec<Move> = vec![];
        move_list.reserve(Self::MOVES_RESERVE_SIZE);

        // Get information about enemy pieces to determine checks and pins
        let (enemy_attack_bb, checkers, pin_masks) = self.enemy_attacks();
        let n_checkers = checkers.len();
        let in_check = n_checkers > 0;

        if !in_check {
            // Not in check, generate moves as usual
            for piece in Piece::VALUES {
                self.get_legal_moves_for_piece(piece, &enemy_attack_bb, &pin_masks, &mut move_list);
            }

            // Castling only legal when not in check
            self.get_castling_moves(enemy_attack_bb, &mut move_list);
        } else {
            // In check, handle check-related moves (e.g. moving out of check,
            // capturing checking piece, blocking rays, etc.)

            if n_checkers == 1 {
                // Single check, filter generated moves for moves that get out of check
                // In the case of double check, only king moves are legal and we do not consider
                // other moves
                let (checker_piece, checker_square, checker_bb) = checkers[0];
                let mut king_ray_mask = 0xffffffffffffffffu64;

                let move_mask = if checker_piece.is_slider() {
                    // Slider check, look for capture of checking piece, and moves that
                    // block the checker's ray.

                    // Get the ray on which the check is
                    let (checking_ray, _) = self.get_checking_ray(checker_square);
                    king_ray_mask = !checking_ray | checker_bb;
                    checker_bb & checking_ray | (1 << checker_square)
                } else {
                    // Non-sliders, i.e. pawns and knights, just check for captures
                    1 << checker_square
                };

                self.get_legal_moves_for_piece_with_mask(
                    Piece::King,
                    &enemy_attack_bb,
                    &pin_masks,
                    &mut move_list,
                    &king_ray_mask
                );
                for piece in Piece::ALL_BUT_KING {
                    self.get_legal_moves_for_piece_with_mask(
                        piece,
                        &enemy_attack_bb,
                        &pin_masks,
                        &mut move_list,
                        &move_mask,
                    );
                }
            } else {
                // Double check, only king moves
                let mut king_ray_mask = 0u64;

                for (checker_piece, checker_square, checker_bb) in checkers {
                    if checker_piece.is_slider() {
                        let (checking_ray, _) = self.get_checking_ray(checker_square);
                        king_ray_mask |= checking_ray | checker_bb;
                    }
                }
                king_ray_mask = !king_ray_mask;

                self.get_legal_moves_for_piece_with_mask(
                    Piece::King,
                    &enemy_attack_bb,
                    &pin_masks,
                    &mut move_list,
                    &king_ray_mask
                );
            }
        }

        (move_list, in_check)
    }

    fn get_legal_moves_for_piece(
        &self,
        piece: Piece,
        enemy_attack_bb: &u64,
        pins: &[u64; N_SQUARES],
        move_list: &mut Vec<Move>,
    ) {
        self.get_legal_moves_for_piece_with_mask(
            piece,
            enemy_attack_bb,
            pins,
            move_list,
            &0xffffffffffffffff,
        )
    }

    fn get_legal_moves_for_piece_with_mask(
        &self,
        piece: Piece,
        enemy_attack_bb: &u64,
        pin_masks: &[u64; N_SQUARES],
        move_list: &mut Vec<Move>,
        mask: &u64,
    ) {
        let mut piece_bb = self.board[(piece, self.board.side_to_move)];
        while piece_bb != 0 {
            let square = piece_bb.trailing_zeros() as usize;

            let mut moves_bb = if piece.is_slider() {
                self.slider_moves(piece, square)
            } else {
                self.pl_moves[(piece, self.board.side_to_move, square)]
            };

            if piece == Piece::Pawn {
                self.pawn_moves(square, &mut moves_bb);
            } else if piece == Piece::King {
                // King cannot move into check
                moves_bb &= !enemy_attack_bb;
            }

            moves_bb &= mask & pin_masks[square];

            self.remove_friendly_moves(&mut moves_bb);

            self.generate_moves_from_bb(piece, square, moves_bb, move_list);

            // clear square bit
            piece_bb &= piece_bb - (1 << square);
        }
    }

    /// Generate moves from a move bitboard for a single piece on a single
    /// square.
    ///
    /// * `piece`: Piece type
    /// * `square`: Square of the piece
    /// * `moves_bb`: Moves bitboard
    /// * `moves_list`: Moves vector to push moves to
    fn generate_moves_from_bb(
        &self,
        piece: Piece,
        square: usize,
        mut moves_bb: u64,
        move_list: &mut Vec<Move>,
    ) {
        while moves_bb != 0 {
            // Pop least significant 1 bits in moves bitboard to generate moves
            let target_square = moves_bb.trailing_zeros() as usize;

            // We've removed potential "friendly captures", so any overlapping
            // piece with the target square is an enemy's piece
            let is_capture = self.comp_occupation_board & (1 << target_square) != 0;

            // Determine the captured piece
            let mut captured_piece = Piece::Pawn;
            if is_capture {
                for piece in Piece::ALL_BUT_KING {
                    if (1 << target_square) & self.board[(piece, self.board.side_to_move.opposite())] != 0 {
                        captured_piece = piece;
                        break;
                    }
                }
            }

            let is_pawn = piece == Piece::Pawn;
            let pawn_promotion_possible = target_square >= (BOARD_WIDTH * (BOARD_HEIGHT - 1))
                || target_square <= (BOARD_WIDTH) - 1;
            if is_pawn && pawn_promotion_possible  {
                // Pawn move to 8th or 1st rank = multiple possible promotion moves
                self.generate_promotion_moves(
                    piece,
                    square,
                    target_square,
                    is_capture,
                    captured_piece,
                    move_list,
                );
            } else {
                let move_type = if is_pawn && target_square == self.board.en_passant_square {
                    MoveType::EnPassantCapture(captured_piece)
                } else if is_capture {
                    MoveType::Capture(captured_piece)
                } else {
                    MoveType::Quiet
                };

                move_list.push(Move {
                    from_square: square,
                    to_square: target_square,
                    move_type: move_type,
                    piece: piece,
                    side: self.board.side_to_move,
                });
            }

            // clear move bit
            moves_bb &= moves_bb - (1 << target_square);
        }
    }

    fn generate_promotion_moves(
        &self,
        piece: Piece,
        square: usize,
        target_square: usize,
        is_capture: bool,
        captured_piece: Piece,
        move_list: &mut Vec<Move>,
    ) {
        for promotion_piece in Piece::PROMOTION_PIECES {
            let move_type = if is_capture {
                MoveType::CapturePromotion(captured_piece, promotion_piece)
            } else {
                MoveType::Promotion(promotion_piece)
            };
            move_list.push(Move {
                from_square: square,
                to_square: target_square,
                move_type: move_type,
                piece: piece,
                side: self.board.side_to_move,
            });
        }
    }

    /// Update legal pawn moves as pawns movement and captures are different from
    /// other non-sliding pieces.
    ///
    /// * `square`: Square of the pawn
    /// * `moves_bb`: Moves bitboard
    fn pawn_moves(&self, square: usize, moves_bb: &mut u64) {
        // Check for blockers
        let pawn_blockers = *moves_bb & self.comp_occupation_board;
        if pawn_blockers != 0 {
            let nearest_blocker_square = if self.board.side_to_move == Side::White {
                pawn_blockers.trailing_zeros() as usize
            } else {
                (u64::BITS - 1 - pawn_blockers.leading_zeros()) as usize
            };
            *moves_bb &=
                !self.pl_moves[(Piece::Pawn, self.board.side_to_move, nearest_blocker_square)];
            // Pawns can't capture forward, so the blocker square is also illegal
            *moves_bb &= !(1 << nearest_blocker_square);
        }

        // Check for captures
        let capture_squares = &self.pl_moves.pawn_capture_moves[self.board.side_to_move as usize][square];
        *moves_bb |= capture_squares
            & (self.occupation_boards[self.board.side_to_move.opposite() as usize] | 1 << self.board.en_passant_square);
    }

    /// Update legal slider moves, removing moves that are blocked by other pieces
    /// (friendly or enemy).
    ///
    /// * `piece`: Type of piece
    /// * `square`: Square of the slider piece
    pub fn slider_moves(&self, piece: Piece, square: usize) -> u64 {
        match piece {
            Piece::Rook => {
                let blocker_mask =
                    self.comp_occupation_board & self.pl_moves.get_piece_blocker_mask(piece, square);

                self.magics.get_rook_moves(square, blocker_mask)
            }
            Piece::Bishop => {
                let blocker_mask =
                    self.comp_occupation_board & self.pl_moves.get_piece_blocker_mask(piece, square);
                self.magics.get_bishop_moves(square, blocker_mask)
            },
            Piece::Queen => {
                let blocker_mask_bishop =
                    self.comp_occupation_board & self.pl_moves.get_piece_blocker_mask(Piece::Bishop, square);
                let blocker_mask_rook =
                    self.comp_occupation_board & self.pl_moves.get_piece_blocker_mask(Piece::Rook, square);
                self.magics.get_rook_moves(square, blocker_mask_rook)
                    | self.magics.get_bishop_moves(square, blocker_mask_bishop)
            }
            _ => unreachable!(),
        }
    }

    /// This function removes moves that are illegal because the target square
    /// is occupied by a friendly piece. Aditionally, in the case of pawns
    /// (who have capture moves different from movement moves), squares occupied
    /// by enemy pieces also become illegal.
    ///
    /// * `moves_bb`: Moves bitboard
    #[inline]
    fn remove_friendly_moves(&self, moves_bb: &mut u64) {
        *moves_bb &= !self.occupation_boards[self.board.side_to_move as usize];
    }

    /// Generate a bitboard with all attacking trajectories of the opponent's
    /// pieces and find all pieces that are currently checking the king as well as
    /// determining pinned pieces and the ray along which they're pinned.
    fn enemy_attacks(&self) -> (u64, Vec<(Piece, usize, u64)>, [u64; N_SQUARES]) {
        let mut checkers: Vec<(Piece, usize, u64)> = vec![];
        let mut pin_masks = [0xffffffffffffffffu64; N_SQUARES];
        let mut attacks_bb = 0u64;
        let king_pos = self.board[(Piece::King, self.board.side_to_move)];
        let opposite_side = self.board.side_to_move.opposite();

        for piece in Piece::VALUES {
            let mut piece_bb = self.board[(piece, opposite_side)];
            while piece_bb != 0 {
                let square = piece_bb.trailing_zeros() as usize;
                let is_slider = piece.is_slider();

                let moves_bb = if is_slider {
                    self.slider_moves(piece, square)
                } else if piece == Piece::Pawn {
                    self.pl_moves.pawn_capture_moves[opposite_side as usize][square]
                } else {
                    self.pl_moves[(piece, opposite_side, square)]
                };

                // We do not care about friendly moves here, so no need to filter

                if moves_bb & king_pos != 0 {
                    checkers.push((piece, square, moves_bb));
                } else if is_slider {
                    // Piece isn't checking king, check if it's pinning pieces
                    self.enemy_attacks_piece_pins(piece, square, king_pos, &mut pin_masks);
                }

                attacks_bb |= moves_bb;

                // clear square bit
                piece_bb &= piece_bb - (1 << square);
            }
        }

        (attacks_bb, checkers, pin_masks)
    }

    fn enemy_attacks_piece_pins(
        &self,
        piece: Piece,
        square: usize,
        king_pos: u64,
        pin_masks: &mut [u64; N_SQUARES],
    ) {
        let comp_ray = self.pl_moves.get_comp_rays(piece)[square];
        let king_aligned = comp_ray & king_pos;

        if king_aligned != 0 {
            // Slider is aligned with king but not checking ---
            // check and track pinned pieces
            let (pin_ray, ray_direction) = self.get_checking_ray(square);

            let king_square = king_pos.trailing_zeros() as usize;
            let enemy_piece_king_ray = pin_ray
                & !(self.pl_moves.rays[ray_direction as usize][king_square]);
            let own_blocking_pieces = enemy_piece_king_ray
              & self.occupation_boards[self.board.side_to_move.opposite() as usize];

            if own_blocking_pieces == 0 {
                // No friendly pieces in the way of a pin, check for
                // number of enemy pieces on ray. When there's only a
                // single enemy piece on the ray, that piece is pinned.
                let enemy_blocking_pieces = enemy_piece_king_ray &
                  & self.occupation_boards[self.board.side_to_move as usize]
                  & !king_pos;
                let n_blockers = enemy_blocking_pieces.count_ones();

                if n_blockers == 1 {
                    // Single piece between enemy slider and king, it's pinned
                    let pinned_square = enemy_blocking_pieces.trailing_zeros() as usize;
                    pin_masks[pinned_square] = pin_ray | (1 << square);
                }
            }
        }
    }

    fn get_checking_ray(&self, checker_square: usize) -> (u64, RayDirection) {
        let king_square = self.board[(Piece::King, self.board.side_to_move)].trailing_zeros() as usize;
        let king_file = file(king_square);
        let king_rank = rank(king_square);

        let checker_file = file(checker_square);
        let checker_rank = rank(checker_square);

        let direction = if king_file == checker_file {
            if king_rank < checker_rank {
                RayDirection::South
            } else {
                RayDirection::North
            }
        } else if king_file > checker_file {
            if king_rank == checker_rank {
                RayDirection::West
            } else if king_rank < checker_rank {
                RayDirection::SouthWest
            } else {
                RayDirection::NorthWest
            }
        } else {
            if king_rank == checker_rank {
                RayDirection::East
            } else if king_rank < checker_rank {
                RayDirection::SouthEast
            } else {
                RayDirection::NorthEast
            }
        };

        (self.pl_moves.rays[direction as usize][checker_square], direction)
    }

    const SHORT_CASTLE_MASKS: [u64; Side::N_SIDES] =
        [0b00000110, 0b00000110 << (BOARD_WIDTH - 1) * BOARD_WIDTH];
    const LONG_CASTLE_MASKS: [u64; Side::N_SIDES] = [
        0b01110000,
        0b01110000 << (BOARD_WIDTH - 1) * BOARD_WIDTH,
    ];
    const KING_STARTING_POS: [u64; Side::N_SIDES] = [
        1 << 3,
        1 << (BOARD_HEIGHT*(BOARD_HEIGHT - 1) + 3),
    ];

    /// Determines legal castling moves, adding to `move_list` if legal.
    ///
    /// * `enemy_attack_bb`: Bitboard of all squares that enemy pieces cover (attack)
    /// * `move_list`: Moves vector to push castling moves to
    fn get_castling_moves(&self, enemy_attack_bb: u64, move_list: &mut Vec<Move>) {
        if self.board[(Piece::King, self.board.side_to_move)]
            & Self::KING_STARTING_POS[self.board.side_to_move as usize] == 0 {
            // King not on its starting square, don't look further for castling
            // legality
            return;
        }

        if self.board.castling_right_short[self.board.side_to_move as usize] {
            let no_check_in_path = (Self::SHORT_CASTLE_MASKS[self.board.side_to_move as usize]
                & (enemy_attack_bb | self.occupation_boards[self.board.side_to_move as usize])) == 0;
            let rook_in_place = (1 << Board::ROOK_SHORT_SQUARES[self.board.side_to_move as usize])
                & self.board[(Piece::Rook, self.board.side_to_move)] != 0;

            if no_check_in_path && rook_in_place {
                move_list.push(Move {
                    from_square: 0, // unused
                    to_square: 0,   // unused
                    move_type: MoveType::CastleShort,
                    piece: Piece::King,
                    side: self.board.side_to_move,
                });
            }
        }

        if self.board.castling_right_long[self.board.side_to_move as usize] {
            let no_check_in_path = (Self::LONG_CASTLE_MASKS[self.board.side_to_move as usize]
                & (enemy_attack_bb | self.occupation_boards[self.board.side_to_move as usize])) == 0;
            let rook_in_place = (1 << Board::ROOK_LONG_SQUARES[self.board.side_to_move as usize])
                & self.board[(Piece::Rook, self.board.side_to_move)] != 0;

            if no_check_in_path && rook_in_place {
                move_list.push(Move {
                    from_square: 0, // unused
                    to_square: 0,   // unused
                    move_type: MoveType::CastleLong,
                    piece: Piece::King,
                    side: self.board.side_to_move,
                });
            }
        }
    }

    fn update_occupation_boards(&mut self) {
        self.comp_occupation_board = 0;

        for side in Side::VALUES {
            self.occupation_boards[side as usize] = self.board.occupation_board(side);

            // Also keep track of a full composite board of both black and
            // white's pieces
            self.comp_occupation_board |= self.occupation_boards[side as usize];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;

    static PSEUDO_LEGAL_MOVES: Lazy<MoveBitboards> = Lazy::new(|| MoveBitboards::init_legal_moves());
    static MAGICS: Lazy<MagicBitboard> = Lazy::new(|| MagicBitboard::init_precomputed(&*PSEUDO_LEGAL_MOVES));

    #[test]
    fn test_legal_moves() {
        let game = GameState::new(&*PSEUDO_LEGAL_MOVES, &*MAGICS);
        let (legal_moves, _) = game.get_legal_moves();

        assert_eq!(legal_moves.len(), 2 * 8 + 2 * 2);

        let game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001110001100000000,
                    // Black pawns
                    0b0000000011101111000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000000000000,
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
        );

        let (legal_moves, _) = game.get_legal_moves();

        assert!(legal_moves.contains(&Move {
            from_square: 4,
            to_square: 60,
            move_type: MoveType::Capture(Piece::Queen),
            piece: Piece::Queen,
            side: Side::White
        }));
    }

    #[test]
    fn test_pins() {
        // RNBQKBNR
        // pppppppp
        // ....Q...
        // ........
        // .......B
        // ........
        // ppppNppp
        // RNBQKBNR
        let game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001111011100000000,
                    // Black pawns
                    0b0000000011111111000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000100001000010,
                    // Black knights
                    0b0100001000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000000000000100100,
                    // Black bishops
                    0b0010010000000000000000000000000000000001000000000000000000000000,
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
                    0b0001000000000000000010000000000000000000000000000000000000000000,
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
        );
        let (legal_moves, _) = game.get_legal_moves();

        for m in &legal_moves {
            // Pieces on squares 10 and 11 are pinned by bishop and queen, respectively,
            // so they should have no legal moves
            assert_ne!(m.from_square, 10);
            assert_ne!(m.from_square, 11);
        }

        // RNBQKBNR
        // pppppppp
        // ....Q...
        // ........
        // .......B
        // ....B...
        // ppppNppp
        // RNBQKBNR
        let game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001111011100000000,
                    // Black pawns
                    0b0000000011111111000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000100001000010,
                    // Black knights
                    0b0100001000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000010000000000000100100,
                    // Black bishops
                    0b0010010000000000000000000000000000000001000000000000000000000000,
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
                    0b0001000000000000000010000000000000000000000000000000000000000000,
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
        );
        let (legal_moves, _) = game.get_legal_moves();

        // Now the knight is not pinned and can move
        assert!(legal_moves.iter().any(|m| m.from_square == 11));

        // Likewise for the bishop
        assert!(legal_moves.iter().any(|m| m.from_square == 19));

        // R.B.QB.R
        // pppppKpp
        // ......N.
        // .......B
        // ........
        // ........
        // pppppppp
        // RNBQKBNR
        let mut game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001111111100000000,
                    // Black pawns
                    0b0000000011111011000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000001000010,
                    // Black knights
                    0b0000000000000000000000100000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000100000000000000000000000000100100,
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
                    0b0000100000000000000000000000000000000000000000000000000000000000,
                ],
                king: [
                    // White king
                    0b0000000000000000000000000000000000000000000000000000000000001000,
                    // Black king
                    0b0000000000000100000000000000000000000000000000000000000000000000,
                ],

                side_to_move: Side::Black,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );
        let (legal_moves, _) = game.get_legal_moves();

        for m in &legal_moves {
            // Black's only knight is pinned, so it should have no moves
            assert_ne!(m.piece, Piece::Knight);
        }

        // R.B.QB.R
        // pppppKpp
        // ........
        // ........
        // .B......
        // ........
        // pppBpppp
        // RNBQKBNR
        let game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001110111100000000,
                    // Black pawns
                    0b0000000011111011000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000001000010,
                    // Black knights
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000001000000100100,
                    // Black bishops
                    0b0010010000000000000000000000000001000000000000000000000000000000,
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
                    0b0000100000000000000000000000000000000000000000000000000000000000,
                ],
                king: [
                    // White king
                    0b0000000000000000000000000000000000000000000000000000000000001000,
                    // Black king
                    0b0000000000000100000000000000000000000000000000000000000000000000,
                ],

                side_to_move: Side::White,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );
        let (legal_moves, _) = game.get_legal_moves();
        assert!(legal_moves.contains(&Move {
            from_square: 12,
            to_square: 30,
            move_type: MoveType::Capture(Piece::Bishop),
            piece: Piece::Bishop,
            side: Side::White
        }));
        assert!(legal_moves.contains(&Move {
            from_square: 12,
            to_square: 21,
            move_type: MoveType::Quiet,
            piece: Piece::Bishop,
            side: Side::White
        }));

        // R.B.QB.R
        // p.pppppp
        // ..K....R
        // ...p....
        // ..p.....
        // ........
        // pppppppp
        // RNBQKBNR
        let mut game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000001000000100000000000001111111100000000,
                    // Black pawns
                    0b0000000010111111000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000001000010,
                    // Black knights
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000001000000100100,
                    // Black bishops
                    0b0010010000000000000000000000000000000000000000000000000000000000,
                ],
                rooks: [
                    // White rooks
                    0b0000000000000000000000010000000000000000000000000000000010000001,
                    // Black rooks
                    0b1000000100000000000000000000000000000000000000000000000000000000,
                ],
                queens: [
                    // White queens
                    0b0000000000000000000000000000000000000000000000000000000000010000,
                    // Black queens
                    0b0000100000000000000000000000000000000000000000000000000000000000,
                ],
                king: [
                    // White king
                    0b0000000000000000000000000000000000000000000000000000000000001000,
                    // Black king
                    0b0000000000000000001000000000000000000000000000000000000000000000,
                ],

                side_to_move: Side::Black,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );
        let (legal_moves, _) = game.get_legal_moves();
        let king_moves: Vec<&Move> = legal_moves.iter().filter(|m| m.piece == Piece::King).collect();
        assert!(king_moves.len() == 2);

        // Kc5
        assert_eq!(king_moves[0], &Move {
            from_square: 45,
            to_square: 45 - 8,
            move_type: MoveType::Quiet,
            piece: Piece::King,
            side: Side::Black
        });

        // Kb7
        assert_eq!(king_moves[1], &Move {
            from_square: 45,
            to_square: 45 + 9,
            move_type: MoveType::Quiet,
            piece: Piece::King,
            side: Side::Black
        });
    }

    #[test]
    fn test_checkmate() {
        // RNBQKBNR
        // pppppppp
        // ....Q...
        // ........
        // .......B
        // ........
        // pppp..pp
        // RNBQKBNR
        let game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001111001100000000,
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
                    0b0010010000000000000000000000000000000001000000000000000000000000,
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
                    0b0001000000000000000010000000000000000000000000000000000000000000,
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
        );
        let (legal_moves, in_check) = game.get_legal_moves();

        assert!(in_check);
        assert_eq!(legal_moves.len(), 0);
    }

    #[test]
    fn test_get_out_of_check() {
        // RNBQKBNR
        // pppppppp
        // ........
        // ....Q...
        // ........
        // ........
        // pppp..pp
        // RNBQKBNR
        let game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001111001100000000,
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
                    0b0001000000000000000000000000100000000000000000000000000000000000,
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
        );
        let (legal_moves, in_check) = game.get_legal_moves();

        assert!(in_check);

        // move king out of check
        assert_eq!(legal_moves[0], Move {
            from_square: 3,
            to_square: 10,
            move_type: MoveType::Quiet,
            piece: Piece::King,
            side: Side::White,
        });

        // block with knight
        assert_eq!(legal_moves[1], Move {
            from_square: 1,
            to_square: 11,
            move_type: MoveType::Quiet,
            piece: Piece::Knight,
            side: Side::White,
        });

        // block with bishop
        assert_eq!(legal_moves[2], Move {
            from_square: 2,
            to_square: 11,
            move_type: MoveType::Quiet,
            piece: Piece::Bishop,
            side: Side::White,
        });

        // block with queen
        assert_eq!(legal_moves[3], Move {
            from_square: 4,
            to_square: 11,
            move_type: MoveType::Quiet,
            piece: Piece::Queen,
            side: Side::White,
        });

        // R.B.QB.R
        // p.pppppp
        // ..KR....
        // ........
        // .B......
        // .....B..
        // pppppppp
        // RNBQKBNR
        let mut game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001111111100000000,
                    // Black pawns
                    0b0000000010111111000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000001000010,
                    // Black knights
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000001000001000000100100,
                    // Black bishops
                    0b0010010000000000000000000000000001000000000000000000000000000000,
                ],
                rooks: [
                    // White rooks
                    0b0000000000000000000100000000000000000000000000000000000010000001,
                    // Black rooks
                    0b1000000100000000000000000000000000000000000000000000000000000000,
                ],
                queens: [
                    // White queens
                    0b0000000000000000000000000000000000000000000000000000000000010000,
                    // Black queens
                    0b0000100000000000000000000000000000000000000000000000000000000000,
                ],
                king: [
                    // White king
                    0b0000000000000000000000000000000000000000000000000000000000001000,
                    // Black king
                    0b0000000000000000001000000000000000000000000000000000000000000000,
                ],

                side_to_move: Side::Black,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );
        let (legal_moves, in_check) = game.get_legal_moves();
        assert!(in_check);
        assert!(legal_moves.len() == 3);

        // Kc5
        assert_eq!(legal_moves[0], Move {
            from_square: 45,
            to_square: 37,
            move_type: MoveType::Quiet,
            piece: Piece::King,
            side: Side::Black,
        });

        // Kb5
        assert_eq!(legal_moves[1], Move {
            from_square: 45,
            to_square: 38,
            move_type: MoveType::Quiet,
            piece: Piece::King,
            side: Side::Black,
        });

        // Kxd5
        assert_eq!(legal_moves[2], Move {
            from_square: 45,
            to_square: 44,
            move_type: MoveType::Capture(Piece::Rook),
            piece: Piece::King,
            side: Side::Black,
        });

        // R.B.QB.R
        // p.pppppp
        // ..KR....
        // ........
        // .B......
        // .....BB.
        // pppppppp
        // RNBQKBNR
        let mut game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001111111100000000,
                    // Black pawns
                    0b0000000010111111000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000001000010,
                    // Black knights
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000001100001000000100100,
                    // Black bishops
                    0b0010010000000000000000000000000001000000000000000000000000000000,
                ],
                rooks: [
                    // White rooks
                    0b0000000000000000000100000000000000000000000000000000000010000001,
                    // Black rooks
                    0b1000000100000000000000000000000000000000000000000000000000000000,
                ],
                queens: [
                    // White queens
                    0b0000000000000000000000000000000000000000000000000000000000010000,
                    // Black queens
                    0b0000100000000000000000000000000000000000000000000000000000000000,
                ],
                king: [
                    // White king
                    0b0000000000000000000000000000000000000000000000000000000000001000,
                    // Black king
                    0b0000000000000000001000000000000000000000000000000000000000000000,
                ],

                side_to_move: Side::Black,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );
        let (legal_moves, in_check) = game.get_legal_moves();

        assert!(in_check);

        // Additional bishop now defends rook, so Kxd6 is illegal
        assert!(legal_moves.len() == 2);

        // Kc5
        assert_eq!(legal_moves[0], Move {
            from_square: 45,
            to_square: 37,
            move_type: MoveType::Quiet,
            piece: Piece::King,
            side: Side::Black,
        });

        // Kb5
        assert_eq!(legal_moves[1], Move {
            from_square: 45,
            to_square: 38,
            move_type: MoveType::Quiet,
            piece: Piece::King,
            side: Side::Black,
        });
    }

    #[test]
    fn test_castling() {
        // RNBQKBNR
        // pppppppp
        // ........
        // ........
        // ........
        // ........
        // pppppppp
        // R...K..R
        let game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001011111100000000,
                    // Black pawns
                    0b0000000011111111000000000000000000000000000000000100000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                    // Black knights
                    0b0100001000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000000000000000000,
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
                    0b0000000000000000000000000000000000000000000000000000000000000000,
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
        );
        let (legal_moves, in_check) = game.get_legal_moves();

        assert!(!in_check);

        assert!(legal_moves.contains(&Move {
            from_square: 0,
            to_square: 0,
            move_type: MoveType::CastleShort,
            piece: Piece::King,
            side: Side::White
        }));

        // no long castle, would move through pawn check
        assert!(!legal_moves.contains(&Move {
            from_square: 0,
            to_square: 0,
            move_type: MoveType::CastleLong,
            piece: Piece::King,
            side: Side::White
        }));
    }

    #[test]
    fn test_pawn_promotion() {
        // .B......
        // ..p.....
        // ........
        // ........
        // ........
        // ........
        // ........
        // ........
        let game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000100000000000000000000000000000000000000000000000000000,
                    // Black pawns
                    0,
                ],
                knights: [
                    0, 0,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                    // Black bishops
                    0b0100000000000000000000000000000000000000000000000000000000000000,
                ],
                rooks: [
                    0, 0
                ],
                queens: [
                    0, 0
                ],
                king: [
                    0, 0
                ],

                side_to_move: Side::White,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );
        let (legal_moves, _) = game.get_legal_moves();

        for promotion_piece in Piece::PROMOTION_PIECES {
            let move_type = MoveType::CapturePromotion(Piece::Bishop, promotion_piece);
            assert!(legal_moves.iter().any(|m| m.move_type == move_type));
        }

        for promotion_piece in Piece::PROMOTION_PIECES {
            let move_type = MoveType::Promotion(promotion_piece);
            assert!(legal_moves.iter().any(|m| m.move_type == move_type));
        }
    }

    #[test]
    fn test_undo_move() {
        let mut game = GameState::new(&*PSEUDO_LEGAL_MOVES, &*MAGICS);

        let board_initial = game.board.clone();

        let m = Move {
            from_square: 8,
            to_square: 24,
            move_type: MoveType::Quiet,
            piece: Piece::Pawn,
            side: Side::White,
        };

        game.make_move(&m);
        let castling_right_long = game.board.castling_right_long;
        let castling_right_short = game.board.castling_right_short;
        let half_move_of_last_capture = game.half_move_of_last_capture;

        game.update_board_undo_move(&m, &castling_right_long, &castling_right_short, half_move_of_last_capture);

        assert_eq!(game.half_move_number, 1);
        assert_eq!(game.move_number, 1);
        assert_eq!(game.board, board_initial);

        // .B......
        // ..p.....
        // ........
        // ........
        // ........
        // ........
        // ........
        // ........
        let mut game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000100000000000000000000000000000000000000000000000000000,
                    // Black pawns
                    0,
                ],
                knights: [
                    0, 0,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                    // Black bishops
                    0b0100000000000000000000000000000000000000000000000000000000000000,
                ],
                rooks: [
                    0, 0
                ],
                queens: [
                    0, 0
                ],
                king: [
                    0, 0
                ],

                side_to_move: Side::White,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );

        let board_initial = game.board.clone();

        let m = Move {
            from_square: 53,
            to_square: 62,
            move_type: MoveType::CapturePromotion(Piece::Bishop, Piece::Queen),
            piece: Piece::Pawn,
            side: Side::White,
        };

        let castling_right_long = game.board.castling_right_long;
        let castling_right_short = game.board.castling_right_short;
        let half_move_of_last_capture = game.half_move_of_last_capture;
        game.make_move(&m);

        game.update_board_undo_move(&m, &castling_right_long, &castling_right_short, half_move_of_last_capture);

        assert_eq!(game.board, board_initial);
    }

    #[test]
    fn test_en_passant() {
        // RNBQKBNR
        // pppppppp
        // ........
        // .p......
        // ........
        // ........
        // pppppppp
        // R...K..R
        let mut game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000100000000000000000000001111111100000000,
                    // Black pawns
                    0b0000000011111111000000000000000000000000000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                    // Black knights
                    0b0100001000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000000000000000000,
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
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                    // Black queens
                    0b0001000000000000000000000000000000000000000000000000000000000000,
                ],
                king: [
                    // White king
                    0b0000000000000000000000000000000000000000000000000000000000001000,
                    // Black king
                    0b0000100000000000000000000000000000000000000000000000000000000000,
                ],

                side_to_move: Side::Black,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );

        let m = Move {
            from_square: 53,
            to_square: 37,
            move_type: MoveType::Quiet,
            piece: Piece::Pawn,
            side: Side::Black,
        };
        game.make_move(&m);

        assert_eq!(game.board.en_passant_square, 37 + BOARD_WIDTH);

        let (legal_moves, _) = game.get_legal_moves();

        let ep_move = Move {
            from_square: 38,
            to_square: 45,
            move_type: MoveType::EnPassantCapture(Piece::Pawn),
            piece: Piece::Pawn,
            side: Side::White
        };

        assert!(legal_moves.contains(&ep_move));

        game.make_move(&ep_move);

        assert_eq!(game.board.en_passant_square, 0);

        let board_after_ep = Board {
            pawns: [
                // White pawns
                0b0000000000000000001000000000000000000000000000001111111100000000,
                // Black pawns
                0b0000000011011111000000000000000000000000000000000000000000000000,
            ],
            knights: [
                // White knights
                0b0000000000000000000000000000000000000000000000000000000000000000,
                // Black knights
                0b0100001000000000000000000000000000000000000000000000000000000000,
            ],
            bishops: [
                // White bishops
                0b0000000000000000000000000000000000000000000000000000000000000000,
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
                0b0000000000000000000000000000000000000000000000000000000000000000,
                // Black queens
                0b0001000000000000000000000000000000000000000000000000000000000000,
            ],
            king: [
                // White king
                0b0000000000000000000000000000000000000000000000000000000000001000,
                // Black king
                0b0000100000000000000000000000000000000000000000000000000000000000,
            ],

            side_to_move: Side::Black,

            en_passant_square: 0usize,
            castling_right_long: [true, true],
            castling_right_short: [true, true],
        };

        assert_eq!(game.board, board_after_ep);

        // En-passant from black's side
        let mut game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000001111111100000000,
                    // Black pawns
                    0b0000000011111111000000000000000000000010000000000000000000000000,
                ],
                knights: [
                    // White knights
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                    // Black knights
                    0b0100001000000000000000000000000000000000000000000000000000000000,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000000000000000000,
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
                    0b0000000000000000000000000000000000000000000000000000000000000000,
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
        );
        let m = Move {
            from_square: 8,
            to_square: 24,
            move_type: MoveType::Quiet,
            piece: Piece::Pawn,
            side: Side::White,
        };
        game.make_move(&m);

        let (legal_moves, _) = game.get_legal_moves();

        let ep_move = Move {
            from_square: 25,
            to_square: 16,
            move_type: MoveType::EnPassantCapture(Piece::Pawn),
            piece: Piece::Pawn,
            side: Side::Black
        };

        assert!(legal_moves.contains(&ep_move));

        let board_before_ep = game.board.clone();
        let ep_square_before = game.board.en_passant_square;
        let castling_right_long = game.board.castling_right_long;
        let castling_right_short = game.board.castling_right_short;
        let half_move_of_last_capture = game.half_move_of_last_capture;

        game.make_move(&ep_move);

        assert_eq!(game.board.en_passant_square, 0);

        game.update_board_undo_move(&ep_move, &castling_right_long, &castling_right_short, half_move_of_last_capture);

        assert_eq!(game.board, board_before_ep);
        assert_eq!(game.board.en_passant_square, ep_square_before);
    }

    #[test]
    fn test_threefold_repetition() {
        // .B......
        // ..p.....
        // ........
        // ........
        // ........
        // ........
        // ........
        // ........
        let mut game = GameState::from_board(
            &*PSEUDO_LEGAL_MOVES,
            &*MAGICS,
            Board {
                pawns: [
                    // White pawns
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                    // Black pawns
                    0,
                ],
                knights: [
                    0, 0,
                ],
                bishops: [
                    // White bishops
                    0b0000000000000000000000000000000000000000000000000000000000000000,
                    // Black bishops
                    0b0100000000000000000000000000000000000000000000000000000000000000,
                ],
                rooks: [
                    0, 0
                ],
                queens: [
                    0b0000000000000000000000000000000000000000000000000000000000000100,
                    0b1000000000000000000000000000000000000000000000000000000000000000
                ],
                king: [
                    0, 0
                ],

                side_to_move: Side::White,

                en_passant_square: 0usize,
                castling_right_long: [true, true],
                castling_right_short: [true, true],
            }
        );

        for i in 0..3 {
            // White: Qg1
            game.make_move(&Move {
                from_square: 0,
                to_square: 1,
                move_type: MoveType::Quiet,
                piece: Piece::Queen,
                side: Side::White,
            });

            // Black: Qb8
            game.make_move(&Move {
                from_square: 63,
                to_square: 62,
                move_type: MoveType::Quiet,
                piece: Piece::Queen,
                side: Side::Black,
            });

            // White: Qh1
            game.make_move(&Move {
                from_square: 1,
                to_square: 0,
                move_type: MoveType::Quiet,
                piece: Piece::Queen,
                side: Side::White,
            });

            // Black: Qa8
            game.make_move(&Move {
                from_square: 62,
                to_square: 63,
                move_type: MoveType::Quiet,
                piece: Piece::Queen,
                side: Side::Black,
            });

            if i < 2 {
                let (legal_moves, in_check) = game.get_legal_moves();
                let move_result = game.get_move_result(&legal_moves, in_check);
                assert_eq!(move_result, None);
            }
        }

        let (legal_moves, in_check) = game.get_legal_moves();
        let move_result = game.get_move_result(&legal_moves, in_check);
        assert_eq!(move_result, Some(MoveResult::Draw(DrawReason::ThreeFoldRepetition)));
    }
}
