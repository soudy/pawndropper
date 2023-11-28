use std::collections::HashMap;

use crate::board::Side;
use crate::game::GameState;
use crate::eval::eval;
use crate::r#move::{Move, MoveResult, NULL_MOVE};

#[derive(PartialEq)]
enum TransitionTableFlag {
    Exact,
    Beta,
    Alpha
}

pub const MAX_KILLER_MOVES: usize = 2;
pub const MAX_GAME_PLY: usize = 1024;

pub struct SearchAsync {
    tt: HashMap<u128, (f64, usize, TransitionTableFlag)>,
    killer_list: [[Move; MAX_GAME_PLY]; MAX_KILLER_MOVES],
    best_move: Move,
    pv_list: Vec<Move>
}

impl SearchAsync {
    const TRANSITION_TABLE_CAPACITY: usize = 1_000_000;

    pub fn new() -> Self {
        Self {
            tt: HashMap::with_capacity(Self::TRANSITION_TABLE_CAPACITY),
            killer_list: [[NULL_MOVE; MAX_GAME_PLY]; MAX_KILLER_MOVES],
            best_move: NULL_MOVE,
            pv_list: vec![]
        }
    }

    pub fn find_best_legal_move(
        &mut self,
        game: &mut GameState,
        depth: usize,
    ) -> (f64, Move, Vec<Move>) {
        let (mut legal_moves_opposite, in_check) = game.get_legal_moves();
        self.order_moves(&mut legal_moves_opposite, 1);

        let alpha = f64::MIN;
        let beta = f64::MAX;
        let mult = if game.board.side_to_move == Side::White {
            1.0
        } else {
            -1.0
        };

        let mut root_pv: Vec<Move> = vec![];

        let eval = self.negamax(
            game,
            &legal_moves_opposite,
            depth,
            1,
            in_check,
            alpha,
            beta,
            &mut root_pv
        );
        (mult*eval, self.best_move, root_pv)
    }

    pub fn negamax(
        &mut self,
        game: &mut GameState,
        legal_moves: &Vec<Move>,
        mut max_depth: usize,
        ply: usize,
        in_check: bool,
        mut alpha: f64,
        beta: f64,
        pv: &mut Vec<Move>
    ) -> f64 {
        if in_check || legal_moves.len() == 1 {
            max_depth += 1;
        }

        if ply >= max_depth {
            return self.qsearch(
                game,
                legal_moves,
                max_depth + 15,
                ply,
                in_check,
                alpha,
                beta,
                pv
            );
        }

        if legal_moves.len() == 0 {
            let mult = if game.board.side_to_move == Side::White {
                1.0
            } else {
                -1.0
            };
            let move_result = game.get_move_result(legal_moves, in_check);

            match move_result {
                Some(MoveResult::Checkmate) => {
                    return mult*(2000.0 + ply as f64)
                }
                Some(MoveResult::Draw(_)) => return 0.0,
                _ => (),
            }
        }

        let tt_entry = self.tt.get(&game.pos_hash);
        if let Some(eval) = tt_entry  {
            let tt_eval = eval.0;
            let tt_depth = eval.1;
            let tt_flag = &eval.2;

            let use_tt_entry = tt_depth > ply &&
                (*tt_flag == TransitionTableFlag::Exact
                 || (*tt_flag == TransitionTableFlag::Beta && tt_eval >= beta)
                 || (*tt_flag == TransitionTableFlag::Alpha && tt_eval <= alpha));

            if use_tt_entry {
                return tt_eval;
            }
        }

        // Needed for undoing moves
        let pos_hash = game.pos_hash;
        let castling_right_long = game.board.castling_right_long;
        let castling_right_short = game.board.castling_right_short;
        let half_move_of_last_capture = game.half_move_of_last_capture;

        let mut best_eval: f64;

        best_eval = f64::MIN;

        for m in legal_moves {
            let mut node_pv: Vec<Move> = vec![];

            game.update_board_with_move(m);

            let (mut legal_moves_opposite, in_check) = game.get_legal_moves();
            self.order_moves(&mut legal_moves_opposite, ply);

            let eval = -self.negamax(
                game,
                &legal_moves_opposite,
                max_depth,
                ply + 1,
                in_check,
                -beta,
                -alpha,
                &mut node_pv
            );

            game.update_board_undo_move(
                m,
                pos_hash,
                &castling_right_long,
                &castling_right_short,
                half_move_of_last_capture,
            );

            if eval > best_eval {
                best_eval = eval;
                if ply == 1 {
                    self.best_move = *m;
                }
            }

            if eval >= beta {
                self.store_killer(m, ply);
                return beta;
            }
            
            if eval > alpha {
                alpha = eval;

                pv.clear();
                pv.push(*m);
                pv.append(&mut node_pv);
            }
        }

        let flag = if best_eval >= beta {
            TransitionTableFlag::Beta
        } else if best_eval > alpha {
            TransitionTableFlag::Exact
        } else {
            TransitionTableFlag::Alpha
        };
        self.tt.insert(game.pos_hash, (alpha, ply, flag));

        alpha
    }

    fn qsearch(
        &mut self,
        game: &mut GameState,
        legal_moves: &Vec<Move>,
        max_depth: usize,
        ply: usize,
        in_check: bool,
        mut alpha: f64,
        beta: f64,
        pv: &mut Vec<Move>
    ) -> f64 {
        // Continue searching until the position is quiet, i.e. positions where
        // there are no winning tactical moves to be made.
        // This avoids the horizon effect
        let mult = if game.board.side_to_move == Side::White {
            1.0
        } else {
            -1.0
        };
        let stand_pat = mult*eval(game);

        let move_result = game.get_move_result(legal_moves, in_check);

        match move_result {
            Some(MoveResult::Checkmate) => {
                return mult*(2000.0 + ply as f64)
            }
            Some(MoveResult::Draw(_)) => return 0.0,
            _ => (),
        }

        if ply >= max_depth {
            return stand_pat;
        }

        if stand_pat >= beta {
            return beta;
        }

        if alpha < stand_pat {
            alpha = stand_pat;
        }

        let old_alpha = alpha;

        let pos_hash = game.pos_hash;
        let castling_right_long = game.board.castling_right_long;
        let castling_right_short = game.board.castling_right_short;
        let half_move_of_last_capture = game.half_move_of_last_capture;

        for m in legal_moves {
            if !in_check && !(m.is_capture() || m.is_promotion()) {
                // Only check for unstabilizing moves such as captures and promotions
                // when not in check. When in check, consider all moves
                continue;
            }

            let mut node_pv: Vec<Move> = vec![];

            game.update_board_with_move(m);

            let (mut legal_moves_opposite, in_check) = game.get_legal_moves();
            self.order_moves(&mut legal_moves_opposite, ply);

            let eval = -self.qsearch(
                game,
                &legal_moves_opposite,
                max_depth,
                ply + 1,
                in_check,
                -beta,
                -alpha,
                &mut node_pv
            );

            game.update_board_undo_move(
                m,
                pos_hash,
                &castling_right_long,
                &castling_right_short,
                half_move_of_last_capture,
            );

            if eval >= beta {
                return beta;
            }

            if eval > alpha {
                alpha = eval;

                pv.clear();
                pv.push(*m);
                pv.append(&mut node_pv);
            }
        }

        let flag = if stand_pat >= beta {
            TransitionTableFlag::Beta
        } else if stand_pat > old_alpha {
            TransitionTableFlag::Exact
        } else {
            TransitionTableFlag::Alpha
        };
        self.tt.insert(game.pos_hash, (alpha, ply, flag));

        alpha
    }

    fn store_killer(&mut self, m: &Move, ply: usize) {
        let first_killer = &self.killer_list[0][ply];

        if first_killer != m {
            // Shift killer moves one index up
            for i in (1..MAX_KILLER_MOVES).rev() {
                let prev = self.killer_list[i - 1][ply];
                self.killer_list[i][ply] = prev;
            }
        }

        self.killer_list[0][ply] = *m;
    }

    fn order_moves(&self, moves: &mut Vec<Move>, ply: usize) {
        moves.sort_unstable_by(|a, b| {
            b.prio(ply, &self.killer_list).cmp(&a.prio(ply, &self.killer_list))
        });
    }
}
