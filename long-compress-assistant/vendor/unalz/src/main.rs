//! `unalz`: command-line ALZ archive extractor.

use std::io::{self, Read, Write};
use std::path::Path;
use std::process;

use unalz::archive::{
    ATTR_ARCHIVE, ATTR_DIRECTORY, ATTR_HIDDEN, ATTR_READONLY, ATTR_SYSTEM, AlzArchive,
    archive_totals,
};
use unalz::dostime::dos_datetime_to_string;
use unalz::extract;

const USAGE: &str = "\
Usage: unalz [OPTION]... ARCHIVE [FILE]...
  or:  unalz [OPTION]... - [FILE]...

Extract files from an ALZ archive, or list its contents. With no FILE
operand every file is extracted; otherwise only the named FILEs are.
A single '-' in place of ARCHIVE reads the archive from standard input.

  -l, --list            list the archive contents, do not extract
  -d, --output-dir=DIR  extract into DIR instead of the current directory
  -p, --pipe            write extracted files to standard output
  -P, --password=PW     decrypt the archive with password PW
  -q, --quiet           suppress informational messages
  -h, --help            display this help and exit
  -V, --version         display version information and exit

Report bugs at <https://github.com/alkegi/unalz/issues>.";

struct Cli {
    list: bool,
    pipe: bool,
    quiet: bool,
    dest_dir: Option<String>,
    password: Option<String>,
    archive: String,
    files: Vec<String>,
}

fn version() -> ! {
    println!("unalz {}", env!("CARGO_PKG_VERSION"));
    process::exit(0);
}

fn parse_args() -> Result<Cli, String> {
    let mut list = false;
    let mut pipe = false;
    let mut quiet = false;
    let mut dest_dir = None;
    let mut password = None;
    let mut positional: Vec<String> = Vec::new();
    let mut options_done = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if options_done || arg == "-" || !arg.starts_with('-') {
            positional.push(arg);
        } else if arg == "--" {
            options_done = true;
        } else if let Some(long) = arg.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (long, None),
            };
            let mut value = |what: &str| {
                inline.clone().map_or_else(
                    || args.next().ok_or(format!("--{name} requires {what}")),
                    Ok,
                )
            };
            match name {
                "list" => list = true,
                "pipe" => pipe = true,
                "quiet" => quiet = true,
                "output-dir" => dest_dir = Some(value("a directory")?),
                "password" => password = Some(value("a password")?),
                "help" => {
                    println!("{USAGE}");
                    process::exit(0);
                }
                "version" => version(),
                _ => return Err(format!("unknown option: --{name}")),
            }
        } else {
            // Short-option cluster: -lq, -dDIR, -d DIR, -P PW, ...
            let chars: Vec<char> = arg[1..].chars().collect();
            let mut i = 0;
            while i < chars.len() {
                match chars[i] {
                    'l' => list = true,
                    'p' => pipe = true,
                    'q' => quiet = true,
                    'h' => {
                        println!("{USAGE}");
                        process::exit(0);
                    }
                    'V' => version(),
                    opt @ ('d' | 'P') => {
                        let rest: String = chars[i + 1..].iter().collect();
                        let val = if rest.is_empty() {
                            args.next().ok_or(format!("-{opt} requires an argument"))?
                        } else {
                            rest
                        };
                        if opt == 'd' {
                            dest_dir = Some(val);
                        } else {
                            password = Some(val);
                        }
                        break;
                    }
                    c => return Err(format!("unknown option: -{c}")),
                }
                i += 1;
            }
        }
    }

    if positional.is_empty() {
        return Err("no archive specified".into());
    }
    let archive = positional.remove(0);
    Ok(Cli {
        list,
        pipe,
        quiet,
        dest_dir,
        password,
        archive,
        files: positional,
    })
}

fn main() {
    let cli = match parse_args() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            process::exit(2);
        }
    };

    let quiet = cli.quiet || cli.pipe;

    let mut archive = if cli.archive == "-" {
        let mut data = Vec::new();
        if let Err(e) = std::io::stdin().read_to_end(&mut data) {
            eprintln!("err: {e}");
            process::exit(1);
        }
        match AlzArchive::from_bytes(data) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("file open error : stdin");
                eprintln!("err: {e}");
                process::exit(1);
            }
        }
    } else {
        match AlzArchive::open(&cli.archive) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("file open error : {}", cli.archive);
                eprintln!("err: {e}");
                process::exit(1);
            }
        }
    };

    if cli.list {
        // A closed pipe (`unalz -l big.alz | head`) is the reader's normal way
        // of saying "enough", not a failure.
        if let Err(e) = list_archive(&archive, &cli.archive)
            && e.kind() != io::ErrorKind::BrokenPipe
        {
            eprintln!("err: {e}");
            process::exit(2);
        }
        return;
    }

    let password = if archive.is_encrypted {
        if let Some(ref pwd) = cli.password {
            Some(pwd.clone())
        } else if cli.archive == "-" {
            eprintln!("err: encrypted archive from stdin requires --password");
            process::exit(1);
        } else {
            match rpassword::prompt_password("Enter Password : ") {
                Ok(pwd) => Some(pwd),
                // No TTY
                Err(_) => {
                    eprintln!("err: encrypted archive, password required (use --password)");
                    process::exit(1);
                }
            }
        }
    } else {
        cli.password.clone()
    };

    let dest_dir = cli.dest_dir.as_deref().unwrap_or(".");
    let dest_path = Path::new(dest_dir);

    if !quiet {
        eprintln!("\nExtract {} to {}", cli.archive, dest_dir);
    }

    let result = if cli.files.is_empty() {
        extract::extract_all(
            &mut archive,
            dest_path,
            password.as_deref(),
            cli.pipe,
            quiet,
        )
    } else {
        extract::extract_files(
            &mut archive,
            dest_path,
            &cli.files,
            password.as_deref(),
            cli.pipe,
            quiet,
        )
    };

    match result {
        // 0 = all extracted, 1 = partial (truncation warning already printed),
        // 2 = fatal.
        Ok(true) => {
            if !quiet {
                eprintln!("\ndone.");
            }
        }
        Ok(false) => {
            if !quiet {
                eprintln!("\ndone (with warnings).");
            }
            process::exit(1);
        }
        // A reader closing the pipe (`unalz -p a.alz f | head`) is a normal stop.
        Err(ref e) if cli.pipe && e.is_broken_pipe() => {}
        Err(e) => {
            eprintln!("\nextract failed: {e}");
            process::exit(2);
        }
    }
}

fn list_archive(archive: &AlzArchive, source: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    writeln!(out, "\nListing archive: {source}")?;
    writeln!(out)?;
    writeln!(
        out,
        "Attr  Uncomp Size    Comp Size Method  Date & Time & File Name"
    )?;
    writeln!(
        out,
        "----- ------------ ------------ ------- ------------------------------------------------"
    )?;

    for entry in &archive.entries {
        let a = entry.file_attribute;
        let attr = format!(
            "{}{}{}{}{}",
            if a & ATTR_ARCHIVE != 0 { "A" } else { "_" },
            if a & ATTR_DIRECTORY != 0 { "D" } else { "_" },
            if a & ATTR_READONLY != 0 { "R" } else { "_" },
            if a & ATTR_HIDDEN != 0 { "H" } else { "_" },
            if a & ATTR_SYSTEM != 0 { "S" } else { "_" },
        );

        let datetime = dos_datetime_to_string(entry.file_time_date);
        let encrypted = if entry.is_encrypted() { "*" } else { "" };

        writeln!(
            out,
            "{attr} {:>12} {:>12} {:<7} {datetime}  {}{encrypted}",
            entry.uncompressed_size,
            entry.compressed_size,
            entry.compression_method,
            entry.file_name,
        )?;
    }

    let (total_uncompressed, total_compressed, file_count) = archive_totals(&archive.entries);

    writeln!(
        out,
        "----- ------------ ------------ ------- ------------------------------------------------"
    )?;
    let plural = if file_count <= 1 { "" } else { "s" };
    writeln!(
        out,
        "      {total_uncompressed:>12} {total_compressed:>12}         Total {file_count} file{plural}"
    )
}
