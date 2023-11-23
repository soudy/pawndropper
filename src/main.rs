mod board;
mod move_bitboards;
mod r#move;
mod game;
mod eval;
mod search;
mod magic;
mod zobrist;
mod cli;

use crate::board::{Board, Piece, Side};
use crate::game::GameState;
use crate::magic::MagicBitboard;
use crate::move_bitboards::MoveBitboards;
use crate::r#move::{Move, MoveType, MoveResult};
use crate::search::SearchAsync;

use std::collections::HashMap;

use rand::seq::SliceRandom;

use log::info;
use env_logger::Env;

use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

use std::time::Instant;

fn print_legal_moves(side: Side, moves: &Vec<Move>) {
    let mut moves_str = "".to_owned();
    for m in moves {
        moves_str.push_str(&m.to_algebraic_with_state(moves));
        moves_str.push_str(" ");
    }
    info!("[{:?}] Legal moves: [{}]", side, moves_str);
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        Env::default().default_filter_or("pawndropper=info")
    ).init();

    // Parse CLI args
    use clap::Parser;
    let args = cli::Args::parse();

    // Initialise engine states (search thread pool, pseudo-legal moves, etc.)
    let searcher = SearchAsync::new(args.n_threads);
    let pseudo_legal_moves = MoveBitboards::init_legal_moves();
    let magics = MagicBitboard::init_precomputed(&pseudo_legal_moves);
    //
    // Uncomment to (re)generate magics
    //MagicBitboard::init(&pseudo_legal_moves).print_magics();
    let mut game = GameState::new(&pseudo_legal_moves, &magics);

    let mut move_res: Option<MoveResult>;
    let (mut legal_moves, _) = game.get_legal_moves();

    let cpu_side = Side::from_str(&args.cpu_side);

    // If the computer is white, make a white move before going in readline loop
    if cpu_side == Side::White {
        // Only respectable moves, of course
        let considered_moves = [
            // e4
            Move {
                from_square: 11,
                to_square: 27,
                move_type: MoveType::Quiet,
                piece: Piece::Pawn,
                side: Side::White,
            },
            // d4
            Move {
                from_square: 12,
                to_square: 28,
                move_type: MoveType::Quiet,
                piece: Piece::Pawn,
                side: Side::White,
            }
        ];

        let m = considered_moves.choose(&mut rand::thread_rng()).unwrap();

        (_, legal_moves) = game.make_move(&m);

        println!("{}", game.board.to_ascii(cpu_side.opposite()));

        println!("1. {}", m.to_algebraic_with_state(&legal_moves));
    } else {
        println!("{}", game.board.to_ascii(cpu_side.opposite()));
    }

    // Readline instance for user input
    let mut rl = DefaultEditor::new()?;

    loop {
        let mut move_map = HashMap::<String, &Move>::new();

        for m in &legal_moves {
            let algebraic_notation = m.to_algebraic_with_state(&legal_moves);
            move_map.insert(algebraic_notation, m);
        }

        print_legal_moves(game.board.side_to_move, &legal_moves);

        let rl_str = if game.board.side_to_move == Side::White {
            format!("move {}> ", game.move_number)
        } else {
            format!("move ..{}> ", game.move_number)
        };
        let readline = rl.readline(&rl_str);
        match readline {
            Ok(line) => {
                match move_map.get(&line) {
                    Some(m) => {
                        // User move
                        (move_res, legal_moves) = game.make_move(m);

                        println!("{}", game.board.to_ascii(cpu_side.opposite()));

                        match move_res {
                            Some(MoveResult::Checkmate) => {
                                println!("Checkmate --- computer loses");
                                break;
                            },
                            Some(MoveResult::Draw(reason)) => {
                                println!("Draw: {:?}", reason);
                                break;
                            },
                            _ => {},
                        }

                        // Computer move
                        let start = Instant::now();
                        let (best_eval, best_move) =
                            searcher.find_best_legal_move(&mut game, &legal_moves, args.depth);
                        let duration = start.elapsed();

                        info!("Search took {:?}", duration);

                        print_legal_moves(game.board.side_to_move, &legal_moves);

                        let move_str = if game.board.side_to_move == Side::White {
                            format!("{}. {}", game.move_number, best_move.to_algebraic_with_state(&legal_moves))
                        } else {
                            format!("{}. ..{}", game.move_number, best_move.to_algebraic_with_state(&legal_moves))
                        };
                        println!("{}", move_str);

                        (move_res, legal_moves) = game.make_move(&best_move);

                        println!("{}", game.board.to_ascii(cpu_side.opposite()));
                        info!("Eval: {:.3}", best_eval);

                        let moves_since_capture =
                            (game.half_move_number - game.half_move_of_last_capture) / 2;
                        info!("Moves since last capture: {}", moves_since_capture);

                        match move_res {
                            Some(MoveResult::Checkmate) => {
                                println!("Checkmate --- computer wins");
                                break;
                            },
                            Some(MoveResult::Draw(reason)) => {
                                println!("Draw: {:?}", reason);
                                break;
                            },
                            _ => {},
                        }
                    },
                    _ => println!("Invalid or illegal move '{}'", line)
                }
            },
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }

    Ok(())
}
