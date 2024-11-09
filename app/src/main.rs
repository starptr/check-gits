mod cli {
    use clap::Parser;
    use std::path::PathBuf;

    #[derive(Parser, Debug)]
    #[command(version, about, long_about = None)]

    pub struct Args {
        /// The directory where the repositories are stored
        pub repos_directory: Option<PathBuf>,
    }

    pub fn get_args() -> Args {
        Args::parse()
    }
}

fn main() {
    let args = cli::get_args();

}
