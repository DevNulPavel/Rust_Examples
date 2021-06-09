use std::path::PathBuf;
use structopt::StructOpt;

/// App parameters
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct AppArguments {
    /// pvrgz atlas images directory
    #[structopt(long, parse(from_os_str))]
    pub atlases_images_directory: PathBuf,

    /// pvrgz atlas' json directory
    #[structopt(long, parse(from_os_str))]
    pub alternative_atlases_json_directory: Option<PathBuf>,

    /// pvrgz cache path
    #[structopt(long, parse(from_os_str))]
    pub cache_path: PathBuf,

    /// Target webp quality
    #[structopt(long)]
    pub target_webp_quality: u8,

    /// Minimum pvrgz size for convert
    #[structopt(long)]
    pub minimum_pvrgz_size: u64,

    /// Verbose
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8
}
