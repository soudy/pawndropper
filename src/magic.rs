use crate::board::{N_SQUARES, Piece};
use crate::move_bitboards::MoveBitboards;

use rand::Rng;

use log::info;

pub struct MagicBitboard {
    pub rook_magics: [u64; N_SQUARES],
    pub bishop_magics: [u64; N_SQUARES],

    // These are too big, need to be on heap instead
    // https://github.com/rust-lang/rust/issues/53827
    pub rook_table: Vec<[u64; 1 << 12]>,
    pub bishop_table: Vec<[u64; 1 << 9]>
}

impl MagicBitboard {
    const ROOK_SQUARE_BITS: [usize; N_SQUARES] = [
        12, 11, 11, 11, 11, 11, 11, 12,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        11, 10, 10, 10, 10, 10, 10, 11,
        12, 11, 11, 11, 11, 11, 11, 12
    ];
    const BISHOP_SQUARE_BITS: [usize; N_SQUARES] = [
        6, 5, 5, 5, 5, 5, 5, 6,
        5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 7, 7, 7, 7, 5, 5,
        5, 5, 7, 9, 9, 7, 5, 5,
        5, 5, 7, 9, 9, 7, 5, 5,
        5, 5, 7, 7, 7, 7, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5,
        6, 5, 5, 5, 5, 5, 5, 6
    ];

    // Pre-computed magics, save computation on start-up
    const PRECOMP_ROOK_MAGICS: [u64; N_SQUARES] = [
        36037800344256544, 18014699425783808, 612507141653135490, 180149585766776960, 9367522452253967440, 1224983496725373696, 4683814535262110208, 144126200376230433, 2379167241225650193, 141012542431234, 576601558536429568, 5584604309795934208, 9278822955159418880, 563018773611544, 422508817940608, 3518438291013888, 3518986966810624, 8092968805298341970, 141287512875016, 108227678435282944, 2307112395455072257, 4756083781126980608, 324351532416402961, 1441154079790273092, 36169674093903872, 234398289005380224, 35186520621184, 180231948173050242, 2319406606129299712, 2759018022366611468, 285885924901376, 5718018812302337, 143211397906528, 2534531085123584, 425030022340608, 1556910620823552, 72202731728668672, 147070718289380353, 11817463048833073442, 869335480108323072, 648845452124520448, 9385607195078443012, 1971012052779028, 10971912467835912224, 146648739894788640, 1153484506234224656, 6352332849544429576, 144115473842569220, 2342435873439629824, 2377971041252245632, 292734388104336512, 1153211777825112192, 292879184332062976, 9288682888497408, 649098922857499648, 9223381951992562176, 9268478951647367425, 126241601142923521, 2328431583863832713, 87965239871493, 4630544876231200811, 18296199905281537, 13873057185728823427, 4543598766867458
    ];
    const PRECOMP_BISHOP_MAGICS: [u64; N_SQUARES] = [
        325407081031270657, 153135587946152352, 1235121311739805696, 9875273726504208384, 3127829380923981954, 9578954033758208, 432917397851340816, 577025912086463012, 10394312372512104608, 8967900660225, 1134722105352192, 4613829965284352, 9108553007122, 153123642000016384, 18024947088098304, 4510197839299778, 9010772709740806, 3118560251544576, 7066147987151400980, 1306189061865619456, 9391412647615794200, 1162491692473975808, 81223124108189698, 4683884633430163841, 9027283058955008, 1139094215723008, 16142046755679635472, 290271338700832, 13837037193442377744, 56297196651299329, 151183352660992, 9251521218163642624, 4516813648957504, 4611829126906515584, 6341420299485971460, 10088345192190443616, 4649968831463493888, 466198301114624, 36596146243700744, 5428301810237504, 2326391920001028096, 6918098171613683713, 9223653614978368000, 4684871574359140608, 142979629319170, 698093160976355393, 20363264588251280, 2306970577765664260, 1129766788796545, 285875775012864, 1297126861244203392, 162129587692773461, 703962592387072, 9512732779751161888, 2542075245760720, 2254001020207109, 563500800823312, 2305849614890436096, 288934339579219008, 206162760192, 2342022547243024640, 289365243984347648, 72274233530597888, 1443651139441673472
    ];

    pub fn init(pl_moves: &MoveBitboards) -> Self {
        let mut magic_bb = Self {
            rook_magics: [0u64; N_SQUARES],
            bishop_magics: [0u64; N_SQUARES],

            rook_table: vec![[0u64; 1 << 12]; N_SQUARES],
            bishop_table: vec![[0u64; 1 << 9]; N_SQUARES]
        };

        info!("Computing magics and calculating blocker moves");

        magic_bb.compute_magics(pl_moves, Piece::Bishop);
        magic_bb.compute_magics(pl_moves, Piece::Rook);

        info!("Populating blocker move tables");

        magic_bb.init_move_table(pl_moves, Piece::Bishop);
        magic_bb.init_move_table(pl_moves, Piece::Rook);

        magic_bb
    }

    pub fn init_precomputed(pl_moves: &MoveBitboards) -> Self {
        let mut magic_bb = Self {
            rook_magics: Self::PRECOMP_ROOK_MAGICS,
            bishop_magics: Self::PRECOMP_BISHOP_MAGICS,

            rook_table: vec![[0u64; 1 << 12]; N_SQUARES],
            bishop_table: vec![[0u64; 1 << 9]; N_SQUARES]
        };

        info!("Initialising pre-calculated magics and populating blocker move tables");

        magic_bb.init_move_table(pl_moves, Piece::Bishop);
        magic_bb.init_move_table(pl_moves, Piece::Rook);

        magic_bb
    }

    pub fn get_rook_moves(&self, square: usize, blockers: u64) -> u64 {
        let magic = self.rook_magics[square];
        let moves = &self.rook_table[square];
        moves[self.magic_index(magic, blockers, Self::ROOK_SQUARE_BITS[square])]
    }

    pub fn get_bishop_moves(&self, square: usize, blockers: u64) -> u64 {
        let magic = self.bishop_magics[square];
        let moves = &self.bishop_table[square];
        moves[self.magic_index(magic, blockers, Self::BISHOP_SQUARE_BITS[square])]
    }

    pub fn print_magics(&mut self) {
        println!("ROOK MAGICS");
        println!("----------------------");
        for magic in &self.rook_magics {
            print!("{}, ", magic);
        }

        println!("BISHOP MAGICS");
        println!("----------------------");
        for magic in &self.bishop_magics {
            print!("{}, ", magic);
        }
    }

    fn init_move_table(&mut self, pl_moves: &MoveBitboards, piece: Piece) {
        let masks;
        let square_bits;
        let magics;
        if piece == Piece::Bishop {
            masks = &pl_moves.bishop_masks;
            square_bits = &Self::BISHOP_SQUARE_BITS;
            magics = &self.bishop_magics;
        } else {
            // Rook
            masks = &pl_moves.rook_masks;
            square_bits = &Self::ROOK_SQUARE_BITS;
            magics = &self.rook_magics;
        }

        for square in 0..N_SQUARES {
            for blocker_idx in 0..(1 << square_bits[square]) {
                let blockers = self.get_blocker_from_idx(blocker_idx, masks[square]);
                let index = self.magic_index(magics[square], blockers, square_bits[square]);
                if piece == Piece::Bishop {
                    self.bishop_table[square][index] = pl_moves.get_bishop_rays(square, blockers);
                } else {
                    self.rook_table[square][index] = pl_moves.get_rook_rays(square, blockers);
                }
            }
        }
    }

    fn compute_magics(&mut self, pl_moves: &MoveBitboards, piece: Piece) {
        let mut rng = rand::thread_rng();

        let masks;
        let square_bits;
        let table_size;
        if piece == Piece::Bishop {
            masks = &pl_moves.bishop_masks;
            square_bits = &Self::BISHOP_SQUARE_BITS;
            table_size = 9;
        } else {
            // Rook
            masks = &pl_moves.rook_masks;
            square_bits = &Self::ROOK_SQUARE_BITS;
            table_size = 12;
        }

        let mut occ_table;

        // Then find magics
        for square in 0..N_SQUARES {
            'retry: loop {
                let magic = rng.gen::<u64>() & rng.gen::<u64>() & rng.gen::<u64>();

                occ_table = vec![false; 1 << table_size];

                for blocker_idx in 0..(1 << square_bits[square]) {
                    let blockers = self.get_blocker_from_idx(blocker_idx, masks[square]);
                    let index = self.magic_index(magic, blockers, square_bits[square]);

                    if !occ_table[index] {
                        occ_table[index] = true;
                    } else {
                        // hash collision, retry
                        continue 'retry;
                    }
                }

                // Found perfect magic for this square, save to table
                if piece == Piece::Bishop {
                    self.bishop_magics[square] = magic;
                } else {
                    self.rook_magics[square] = magic;
                }

                // Next square
                break;
            }
        }
    }

    fn magic_index(&self, magic: u64, blockers: u64, bits: usize) -> usize {
        ((blockers.wrapping_mul(magic)) >> (64 - bits)) as usize
    }

    fn get_blocker_from_idx(&self, idx: usize, mut mask: u64) -> u64 {
        let mut blockers = 0u64;
        let mut i = 0;

        while mask != 0 {
            let pos = mask.trailing_zeros() as usize;

            if idx & (1 << i) != 0 {
                blockers |= 1 << pos;
            }

            mask &= mask - (1 << pos);
            i += 1;
        }

        blockers
    }
}
