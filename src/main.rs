mod formatter;
mod lexer;

use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
    process,
};

use formatter::{FormatConfig, Formatter};

const HELP: &str = r#"gscfmt — A formatter for GSC

USAGE:
    gscfmt [OPTIONS] [FILE...]
    gscfmt --stdin

OPTIONS:
    -w, --write           Format files in-place
    -c, --check           Check formatting; exit 1 if any file needs changes
    -d, --diff            Print a unified diff to stdout
        --stdin           Read from stdin, write to stdout
        --indent <STR>    Indentation string (default: 4 spaces)
        --tabs            Use a tab character for indentation
        --max-blank <N>   Max consecutive blank lines preserved (default: 1)
    -h, --help            Show this help message
    -V, --version         Show version

EXAMPLES:
    gscfmt script.gsc                    # print formatted output to stdout
    gscfmt --write *.gsc                 # format in-place
    gscfmt --check script.gsc            # CI: exit 1 if unformatted
    gscfmt --diff script.gsc             # see what would change
    gscfmt --tabs --write script.gsc     # use tabs for indentation
    gscfmt --stdin < script.gsc          # pipe mode
"#;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Default)]
struct Args {
    files: Vec<String>,
    write: bool,
    check: bool,
    diff: bool,
    stdin: bool,
    indent: Option<String>,
    tabs: bool,
    max_blank: Option<usize>,
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut a = Args::default();
    let mut iter = raw.iter().peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("gscfmt {VERSION}");
                process::exit(0);
            }
            "-w" | "--write" => a.write = true,
            "-c" | "--check" => a.check = true,
            "-d" | "--diff" => a.diff = true,
            "--stdin" => a.stdin = true,
            "--tabs" => a.tabs = true,
            "--indent" => {
                let val = iter.next().ok_or("--indent requires a value")?;
                a.indent = Some(val.clone());
            }
            "--max-blank" => {
                let val = iter.next().ok_or("--max-blank requires a value")?;
                let n: usize = val
                    .parse()
                    .map_err(|_| format!("--max-blank: invalid number '{val}'"))?;
                a.max_blank = Some(n);
            }
            s if s.starts_with('-') => return Err(format!("unknown option: {s}")),
            file => a.files.push(file.to_string()),
        }
    }

    Ok(a)
}

fn unified_diff(original: &str, formatted: &str, label: &str) -> String {
    if original == formatted {
        return String::new();
    }
    let ol: Vec<&str> = original.lines().collect();
    let fl: Vec<&str> = formatted.lines().collect();
    let mut out = format!("--- {label} (original)\n+++ {label} (formatted)\n");
    out.push_str(&format!("@@ -1,{} +1,{} @@\n", ol.len(), fl.len()));
    for l in &ol {
        out.push('-');
        out.push_str(l);
        out.push('\n');
    }
    for l in &fl {
        out.push('+');
        out.push_str(l);
        out.push('\n');
    }
    out
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("gscfmt: {e}");
            eprintln!("Run `gscfmt --help` for usage.");
            process::exit(2);
        }
    };

    let indent = if args.tabs {
        "\t".to_string()
    } else {
        args.indent.clone().unwrap_or_else(|| "    ".to_string())
    };

    let cfg = FormatConfig {
        indent,
        max_blank_lines: args.max_blank.unwrap_or(1),
    };
    let fmt = Formatter::new(cfg);

    // stdin mode
    if args.stdin {
        let mut src = String::new();
        io::stdin()
            .read_to_string(&mut src)
            .expect("failed to read stdin");
        let result = fmt.format(&src);
        io::stdout()
            .write_all(result.as_bytes())
            .expect("failed to write stdout");
        return;
    }

    if args.files.is_empty() {
        print!("{HELP}");
        process::exit(0);
    }

    let mut exit_code = 0i32;

    for path_str in &args.files {
        let path = Path::new(path_str);

        let original = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("gscfmt: {path_str}: {e}");
                exit_code = 1;
                continue;
            }
        };

        let formatted = fmt.format(&original);
        let changed = original.replace("\r\n", "\n") != formatted;

        if args.diff {
            let d = unified_diff(&original, &formatted, path_str);
            if !d.is_empty() {
                print!("{d}");
                exit_code = 1;
            }
        } else if args.check {
            if changed {
                eprintln!("gscfmt: {path_str}: not formatted");
                exit_code = 1;
            } else {
                println!("gscfmt: {path_str}: OK");
            }
        } else if args.write {
            if changed {
                match fs::write(path, formatted.as_bytes()) {
                    Ok(_) => println!("gscfmt: {path_str}: reformatted"),
                    Err(e) => {
                        eprintln!("gscfmt: {path_str}: write error: {e}");
                        exit_code = 1;
                    }
                }
            } else {
                println!("gscfmt: {path_str}: unchanged");
            }
        } else {
            io::stdout()
                .write_all(formatted.as_bytes())
                .expect("failed to write stdout");
        }
    }

    process::exit(exit_code);
}
