mod unpacker;

use anyhow::Context;
use clap::{AppSettings, ArgGroup, Clap};
use std::path::{Path, PathBuf};
use mila::BinArchive;


fn read_bin_archive_input(input_path: &Path) -> anyhow::Result<BinArchive> {
    let input = std::fs::read(input_path).context("Failed to read input file.")?;
    let input = if let Some(extension) = input_path.extension() {
        if "lz" == extension {
            mila::LZ13CompressionFormat {}
                .decompress(&input)
                .context("Failed to LZ13 decompress input.")
        } else {
            Ok(input)
        }
    } else {
        Ok(input)
    }?;
    let archive = BinArchive::from_bytes(&input)
        .context("Failed to deserialize bin archive.")?;
    Ok(archive)
}

#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "thane98")]
#[clap(setting = AppSettings::ColoredHelp)]
#[clap(group = ArgGroup::new("command").required(true))]
struct Arguments {
    input: String,

    #[clap(long, short)]
    output: Option<String>,

    #[clap(long, short, group = "command", about = "Unpack a bin file")]
    unpack: bool,

    #[clap(long, short, group = "command", about = "Pack a text file")]
    pack: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();

    let input_path = Path::new(&args.input);
    if !input_path.exists() || !input_path.is_file() {
        return Err(anyhow::anyhow!(
            "Input is not a valid file: '{}'",
            input_path.display()
        ));
    }

    if args.unpack {
        let archive = read_bin_archive_input(&input_path)?;
        let text = unpacker::unpack(&archive).context("Failed to unpack archive.")?;

        let path = if let Some(path) = args.output {
            let mut buf = PathBuf::new();
            buf.push(path);
            buf
        } else {
            let mut filename = input_path
                .file_name()
                .context("Could not get filename from input")?
                .to_owned();
            filename.push(".txt");
            let mut buf = PathBuf::new();
            buf.push(filename);
            buf
        };

        std::fs::write(path, text).context("Failed to save output.")?;
    } else if args.pack {
        let input = std::fs::read_to_string(&args.input)
            .context("Failed to read input file.")?;
        let archive = unpacker::pack(&input).context("Failed to pack input file.")?;

        let path = if let Some(path) = args.output {
            let mut buf = PathBuf::new();
            buf.push(path);
            buf
        } else {
            let mut buf = PathBuf::new();
            buf.push(
                input_path
                    .file_name()
                    .context("Could not get filename from input")?,
            );
            buf.set_extension("");
            buf
        };
        
        let serialized = archive.serialize().context("Failed to serialize bin archive.")?;
        let bytes = if let Some(extension) = path.extension() {
            if "lz" == extension {
                mila::LZ13CompressionFormat{}.compress(&serialized)
                    .context("Failed to compress output.")?
            } else {
                serialized
            }
        } else {
            serialized
        };

        std::fs::write(path, bytes).context("Failed to write output.")?;
    }

    Ok(())
}
