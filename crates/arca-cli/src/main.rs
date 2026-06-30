use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::ExitCode as ProcessExitCode;

use anyhow::{Context, Result};
use arca_core::{
    ArcaError, CompressOptions, Encryption, ExitCode, ExtractOptions, FormatKind, Password,
    TestOptions, compress, extract, format, list, test,
};
use clap::{Args, Parser, Subcommand};
use zeroize::Zeroize;

#[derive(Debug, Parser)]
#[command(
    name = "arca",
    version,
    about = "Cross-platform archive utility",
    after_help = "Examples:\n  arca compress docs -o docs.zip\n  arca compress photo.jpg -o photo.jpg.gz\n  arca extract docs.zip -o docs-out\n  arca list docs.zip --json\n  arca test docs.zip"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Create an archive")]
    Compress(CompressArgs),
    #[command(about = "Extract an archive")]
    Extract(ExtractArgs),
    #[command(about = "List archive entries")]
    List(ListArgs),
    #[command(about = "Verify archive integrity")]
    Test(TestArgs),
}

#[derive(Debug, Args)]
struct CompressArgs {
    #[arg(
        required = true,
        value_name = "INPUT",
        help = "Files or directories to add to the archive"
    )]
    inputs: Vec<PathBuf>,

    #[arg(
        short,
        long,
        value_name = "ARCHIVE",
        help = "Output archive path; suffix selects the format"
    )]
    output: Option<PathBuf>,

    #[arg(long, help = "Replace an existing output archive")]
    overwrite: bool,

    #[arg(
        long,
        value_name = "0..9",
        value_parser = clap::value_parser!(u8).range(0..=9),
        help = "Compression level where 0 is fastest and 9 is smallest"
    )]
    level: Option<u8>,

    #[arg(
        long,
        default_value_t = 1,
        value_name = "N",
        value_parser = parse_jobs,
        help = "Parallel jobs for ZIP creation; tar writers are serialized"
    )]
    jobs: usize,

    #[arg(
        long,
        value_name = "GLOB",
        help = "Exclude paths matching this glob; may be repeated"
    )]
    exclude: Vec<String>,

    #[arg(long, help = "Prompt for a ZIP password")]
    password: bool,

    #[arg(long, help = "Read the ZIP password from the first stdin line")]
    password_stdin: bool,

    #[arg(
        long,
        help = "Use weak legacy ZipCrypto instead of AES-256 ZIP encryption"
    )]
    zipcrypto: bool,

    #[arg(
        long,
        help = "When a directory is written to .gz/.bz2/.xz, create a tar-based archive instead"
    )]
    auto_tar: bool,

    #[arg(long, help = "Print only errors")]
    quiet: bool,
}

#[derive(Debug, Args)]
struct ExtractArgs {
    #[arg(help = "Archive to extract")]
    archive: PathBuf,

    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "Destination directory, or file path for single-file compressed streams"
    )]
    output: Option<PathBuf>,

    #[arg(long, help = "Replace an existing destination")]
    overwrite: bool,

    #[arg(
        long,
        default_value_t = default_jobs(),
        value_name = "N",
        value_parser = parse_jobs,
        help = "Parallel jobs for ZIP extraction and testing"
    )]
    jobs: usize,

    #[arg(long, help = "Prompt for a ZIP password")]
    password: bool,

    #[arg(long, help = "Read the ZIP password from the first stdin line")]
    password_stdin: bool,

    #[arg(long, help = "Print only errors")]
    quiet: bool,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(help = "Archive to inspect")]
    archive: PathBuf,

    #[arg(long, help = "Print machine-readable JSON")]
    json: bool,

    #[arg(long, help = "Suppress the table output")]
    quiet: bool,
}

#[derive(Debug, Args)]
struct TestArgs {
    #[arg(help = "Archive to verify")]
    archive: PathBuf,

    #[arg(
        long,
        default_value_t = default_jobs(),
        value_name = "N",
        value_parser = parse_jobs,
        help = "Parallel jobs for ZIP integrity testing"
    )]
    jobs: usize,

    #[arg(long, help = "Prompt for a ZIP password")]
    password: bool,

    #[arg(long, help = "Read the ZIP password from the first stdin line")]
    password_stdin: bool,

    #[arg(long, help = "Print only errors")]
    quiet: bool,
}

fn main() -> ProcessExitCode {
    match run() {
        Ok(()) => ProcessExitCode::from(0),
        Err(error) => {
            let code = error
                .downcast_ref::<ArcaError>()
                .map_or(ExitCode::General, ExitCode::from);
            eprintln!("arca: {error}");
            ProcessExitCode::from(code as u8)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Compress(args) => {
            validate_compress_encryption_flags(&args)?;
            let encryption = read_encryption(args.password, args.password_stdin, args.zipcrypto)?;
            if args.zipcrypto && matches!(encryption, Encryption::None) {
                return Err(ArcaError::Usage(
                    "--zipcrypto requires --password or --password-stdin".into(),
                )
                .into());
            }
            if args.zipcrypto {
                eprintln!(
                    "arca: warning: ZipCrypto is weak and should only be used for compatibility"
                );
            }
            let output = compress(CompressOptions {
                inputs: args.inputs,
                output: args.output,
                overwrite: args.overwrite,
                level: args.level,
                jobs: args.jobs,
                excludes: args.exclude,
                encryption,
                auto_tar: args.auto_tar,
            })?;
            if !args.quiet {
                println!("{}", output.display());
            }
        }
        Command::Extract(args) => {
            validate_archive_password_flags(&args.archive, args.password, args.password_stdin)?;
            let output = extract(ExtractOptions {
                archive: args.archive,
                output: args.output,
                overwrite: args.overwrite,
                jobs: args.jobs,
                password: read_optional_password(args.password, args.password_stdin, false)?,
            })?;
            if !args.quiet {
                println!("{}", output.display());
            }
        }
        Command::List(args) => {
            let entries = list(args.archive)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&entries)?);
            } else if !args.quiet {
                print_list(&entries);
            }
        }
        Command::Test(args) => {
            validate_archive_password_flags(&args.archive, args.password, args.password_stdin)?;
            test(TestOptions {
                archive: args.archive,
                jobs: args.jobs,
                password: read_optional_password(args.password, args.password_stdin, false)?,
            })?;
            if !args.quiet {
                println!("ok");
            }
        }
    }
    Ok(())
}

fn validate_archive_password_flags(archive: &Path, password: bool, stdin: bool) -> Result<()> {
    let format = format::required_format(archive)?;
    if !matches!(format.kind, FormatKind::Zip) && (password || stdin) {
        return Err(
            ArcaError::Unsupported("password options are only supported for .zip".into()).into(),
        );
    }
    Ok(())
}

fn validate_compress_encryption_flags(args: &CompressArgs) -> Result<()> {
    let output = format::normalize_output_path(args.output.clone(), &args.inputs)?;
    let format = format::required_format(&output)?;
    if !matches!(format.kind, FormatKind::Zip)
        && (args.password || args.password_stdin || args.zipcrypto)
    {
        return Err(
            ArcaError::Unsupported("password options are only supported for .zip".into()).into(),
        );
    }
    if args.zipcrypto && !args.password && !args.password_stdin {
        return Err(
            ArcaError::Usage("--zipcrypto requires --password or --password-stdin".into()).into(),
        );
    }
    Ok(())
}

fn read_encryption(password: bool, stdin: bool, zipcrypto: bool) -> Result<Encryption> {
    let Some(password) = read_optional_password(password, stdin, true)? else {
        return Ok(Encryption::None);
    };
    if zipcrypto {
        Ok(Encryption::ZipCrypto(password))
    } else {
        Ok(Encryption::Aes256(password))
    }
}

fn read_optional_password(password: bool, stdin: bool, confirm: bool) -> Result<Option<Password>> {
    if password && stdin {
        return Err(ArcaError::Usage(
            "--password and --password-stdin are mutually exclusive".into(),
        )
        .into());
    }
    if stdin {
        return read_password_stdin().map(Some);
    }
    if password {
        let mut first = rpassword::prompt_password("Password: ")?;
        if confirm {
            let mut second = rpassword::prompt_password("Confirm password: ")?;
            let matches = first == second;
            second.zeroize();
            if !matches {
                first.zeroize();
                return Err(
                    ArcaError::Password("password confirmation did not match".into()).into(),
                );
            }
        }
        return Ok(Some(Password::new(first.into_bytes())));
    }
    Ok(None)
}

fn read_password_stdin() -> Result<Password> {
    let mut input = Vec::new();
    io::stdin()
        .lock()
        .read_until(b'\n', &mut input)
        .context("failed to read password from stdin")?;
    if input.last() == Some(&b'\n') {
        input.pop();
        if input.last() == Some(&b'\r') {
            input.pop();
        }
    }
    Ok(Password::new(input))
}

fn print_list(entries: &[arca_core::ops::ListEntry]) {
    println!("{:>12}  {:>8}  {:<10}  path", "size", "packed", "type");
    for entry in entries {
        let packed = entry
            .compressed_size
            .map(|size| size.to_string())
            .unwrap_or_else(|| "-".into());
        println!(
            "{:>12}  {:>8}  {:<10}  {}",
            entry.uncompressed_size, packed, entry.entry_type, entry.path
        );
    }
}

fn default_jobs() -> usize {
    std::thread::available_parallelism().map_or(1, usize::from)
}

fn parse_jobs(value: &str) -> std::result::Result<usize, String> {
    let jobs = value
        .parse::<usize>()
        .map_err(|_| "jobs must be a positive integer".to_owned())?;
    if jobs == 0 {
        return Err("jobs must be at least 1".to_owned());
    }
    Ok(jobs)
}
