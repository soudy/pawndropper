use crate::board::Side;
use crate::game::GameState;
use crate::r#move::Move;
use crate::eval::eval;
use crate::r#move::MoveResult;

use rayon::ThreadPool;
use rayon::prelude::*;
use dashmap::DashMap;

#[derive(PartialEq)]
enum TransitionTableFlag {
    Exact,
    Beta,
    Alpha
}

pub struct SearchAsync {
    pool: ThreadPool,
    tt: DashMap<u64, (f64, usize, TransitionTableFlag)>
}

impl SearchAsync {
    const EVAL_CACHE_CAPACITY: usize = 1_000_000;

    pub fn new(n_threads: usize) -> Self {
        Self {
            pool: rayon::ThreadPoolBuilder::new()
                .num_threads(n_threads)
                .build()
                .unwrap(),
            tt: DashMap::with_capacity(Self::EVAL_CACHE_CAPACITY)
        }
    }

    pub fn find_best_legal_move(
        &self,
        game: &mut GameState,
        legal_moves: &Vec<Move>,
        depth: usize,
    ) -> (f64, Move) {
        let move_evals: Vec<(f64, &Move)> = self.pool.install(|| {
            legal_moves.into_par_iter().map(|m| {
                let mut branched_game = game.clone();

                branched_game.update_board_with_move(m);
                let (mut legal_moves_opposite, in_check) = branched_game.get_legal_moves();

                self.order_moves(&mut legal_moves_opposite);

                let move_eval = self.minimax(
                    &mut branched_game,
                    &legal_moves_opposite,
                    depth,
                    1,
                    in_check,
                    f64::MIN,
                    f64::MAX,
                );

                (move_eval, m)
            }).collect()
        });

        let mut best_eval = if game.board.side_to_move == Side::White {
            f64::MIN
        } else {
            f64::MAX
        };
        let mut best_move = legal_moves[0];

        for (eval, m) in move_evals {
            if game.board.side_to_move == Side::White && eval > best_eval {
                best_eval = eval;
                best_move = *m;
            } else if game.board.side_to_move == Side::Black && eval < best_eval {
                best_eval = eval;
                best_move = *m;
            }
        }

        (best_eval, best_move)
    }

    pub fn minimax(
        &self,
        game: &mut GameState,
        legal_moves: &Vec<Move>,
        mut max_depth: usize,
        depth: usize,
        in_check: bool,
        mut alpha: f64,
        mut beta: f64,
    ) -> f64 {
        if in_check {
            max_depth += 1;
        }

        if depth >= max_depth {
            return self.qsearch(game, legal_moves, max_depth + 3, depth, in_check, alpha, beta);
        }

        // let tt_entry = self.tt.get(&game.pos_hash);
        // if let Some(eval) = tt_entry  {
            // let tt_eval = eval.0;
            // let tt_depth = eval.1;
            // let tt_flag = &eval.2;

            // let use_tt_entry = tt_depth > depth &&
                // (*tt_flag == TransitionTableFlag::Exact
                 // || (*tt_flag == TransitionTableFlag::Beta && tt_eval >= beta)
                 // || (*tt_flag == TransitionTableFlag::Alpha && tt_eval <= alpha));

            // if use_tt_entry {
                // return tt_eval;
            // }
        // }

        // Needed for undoing moves
        let castling_right_long = game.board.castling_right_long;
        let castling_right_short = game.board.castling_right_short;
        let half_move_of_last_capture = game.half_move_of_last_capture;

        let mut best_eval: f64;

        if game.board.side_to_move == Side::White {
            best_eval = f64::MIN;

            for m in legal_moves {
                game.update_board_with_move(m);

                let (mut legal_moves_opposite, in_check) = game.get_legal_moves();
                self.order_moves(&mut legal_moves_opposite);

                let eval = self.minimax(
                    game,
                    &legal_moves_opposite,
                    max_depth,
                    depth + 1,
                    in_check,
                    alpha,
                    beta,
                );

                game.update_board_undo_move(
                    m,
                    &castling_right_long,
                    &castling_right_short,
                    half_move_of_last_capture,
                );

                if eval > best_eval {
                    best_eval = eval;
                }
                
                if eval > alpha {
                    alpha = eval;
                }

                if eval >= beta {
                    break;
                }
            }
        } else {
            best_eval = f64::MAX;

            for m in legal_moves {
                game.update_board_with_move(m);

                let (mut legal_moves_opposite, in_check) = game.get_legal_moves();
                self.order_moves(&mut legal_moves_opposite);

                let eval = self.minimax(
                    game,
                    &legal_moves_opposite,
                    max_depth,
                    depth + 1,
                    in_check,
                    alpha,
                    beta,
                );

                game.update_board_undo_move(
                    m,
                    &castling_right_long,
                    &castling_right_short,
                    half_move_of_last_capture,
                );

                if eval < best_eval {
                    best_eval = eval;
                }

                if eval < beta {
                    beta = eval;
                }

                if eval <= alpha {
                    break;
                }
            }
        };

        // let flag = if best_eval >= beta {
            // TransitionTableFlag::Beta
        // } else if best_eval > alpha {
            // TransitionTableFlag::Exact
        // } else {
            // TransitionTableFlag::Alpha
        // };
        // self.tt.insert(game.pos_hash, (alpha, depth, flag));

        best_eval
    }

    fn qsearch(
        &self,
        game: &mut GameState,
        legal_moves: &Vec<Move>,
        max_depth: usize,
        depth: usize,
        in_check: bool,
        mut alpha: f64,
        beta: f64,
    ) -> f64 {
        // Continue searching until the position is quiet, i.e. positions where
        // there are no winning tactical moves to be made.
        // This avoids the horizon effect
        let mut stand_pat = eval(game);

        let move_result = game.get_move_result(legal_moves, in_check);

        if let Some(MoveResult::Checkmate) = move_result {
            if game.board.side_to_move == Side::White {
                return -2000.0;
            } else {
                return 2000.0;
            }
        } else if let Some(MoveResult::Draw(_)) = move_result {
            return 0.0;
        }

        if depth >= max_depth {
            return stand_pat;
        }

        if !in_check && stand_pat >= beta {
            return beta;
        }

        if alpha < stand_pat {
            alpha = stand_pat;
        }

        let old_alpha = alpha;

        let castling_right_long = game.board.castling_right_long;
        let castling_right_short = game.board.castling_right_short;
        let half_move_of_last_capture = game.half_move_of_last_capture;

        for m in legal_moves {
            if !in_check && !(m.is_capture() || m.is_promotion()) {
                // Only check for unstabilizing moves such as captures and promotions
                // when not in check. When in check, consider all moves
                continue;
            }

            game.update_board_with_move(m);

            let (mut legal_moves_opposite, in_check) = game.get_legal_moves();
            self.order_moves(&mut legal_moves_opposite);

            let eval = -self.qsearch(
                game,
                &legal_moves_opposite,
                max_depth,
                depth + 1,
                in_check,
                -alpha,
                -beta,
            );

            game.update_board_undo_move(
                m,
                &castling_right_long,
                &castling_right_short,
                half_move_of_last_capture,
            );

            if eval > stand_pat {
                stand_pat = eval;
            }

            if eval >= beta {
                break;
            }

            if eval > alpha {
                alpha = eval;
            }
        }

        // let flag = if stand_pat >= beta {
            // TransitionTableFlag::Beta
        // } else if stand_pat > old_alpha {
            // TransitionTableFlag::Exact
        // } else {
            // TransitionTableFlag::Alpha
        // };
        // self.tt.insert(game.pos_hash, (alpha, depth, flag));

        alpha
    }

    fn order_moves(&self, moves: &mut Vec<Move>) {
        self.pool.install(|| {
            moves.par_sort_unstable_by(|a, b| {
                // check captures first
                b.prio().cmp(&a.prio())
            });
        });
        // moves.sort_by(|a, b| {
            // // check moves with lowest pieces first
            // (a.piece as usize).cmp(&(b.piece as usize))
        // });
    }
}
