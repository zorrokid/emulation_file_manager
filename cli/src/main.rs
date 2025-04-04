use clap::Parser;
use file_compress::{read_zip_file, CompressionMethod};

#[derive(Parser, Debug)]
struct Cli {
    /// Input file (zip archive)
    input_file: String,

    /// Output directory, where files are outputted individually using selected compression method
    output_directory: String,

    /// Compression method (zip, zstd or none)
    #[arg(value_enum)]
    compression_method: CompressionMethod,
}

fn main() {
    let args = Cli::parse();
    if let Err(e) = read_zip_file(
        args.input_file.as_str(),
        args.output_directory.as_str(),
        args.compression_method,
    ) {
        eprintln!("Error processing file: {}", e);
    }
}
