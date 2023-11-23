use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "black")]
    pub cpu_side: String,

    #[arg(short, long, default_value_t = 12)]
    pub n_threads: usize,

    #[arg(short, long, default_value_t = 7)]
    pub depth: usize,
}
