# pawndropper

A humble chess engine with simple negamax and quiescence search with some search heuristics.

## Installation
Simply clone the repository and build using Cargo.

## Usage
```
Usage: pawndropper [OPTIONS]

Options:
  -c, --cpu-side <CPU_SIDE>  [default: black]
  -d, --depth <DEPTH>        [default: 6]
  -h, --help                 Print help
  -V, --version              Print version
```

By default, the engine will play as black and run with a search depth of 6. Moves are made through an interactive terminal user interface:
```
[2023-11-28T20:43:55Z INFO  pawndropper::move_bitboards] Initialising pseudo-legal moves and ray masks
[2023-11-28T20:43:55Z INFO  pawndropper::magic] Initialising pre-calculated magics and populating blocker move tables
8  ♖ ♘ ♗ ♕ ♔ ♗ ♘ ♖
7  ♙ ♙ ♙ ♙ ♙ ♙ ♙ ♙
6  . . . . . . . .
5  . . . . . . . .
4  . . . . . . . .
3  . . . . . . . .
2  ♟︎ ♟︎ ♟︎ ♟︎ ♟︎ ♟︎ ♟︎ ♟︎
1  ♜ ♞ ♝ ♛ ♚ ♝ ♞ ♜

   a b c d e f g h

[2023-11-28T20:43:55Z INFO  pawndropper] [White] Legal moves: [h3 h4 g3 g4 f3 f4 e3 e4 d3 d4 c3 c4 b3 b4 a3 a4 Nh3 Nf3 Nc3 Na3 ]
move 1> ...your move here
```
