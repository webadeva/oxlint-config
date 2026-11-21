//! Parity-test harness: runs the rules over files the way ESLint would,
//! including disable directives and the 10-pass `--fix` loop, so tests can
//! compare its output with ESLint's. Not shipped: users get the rules inside
//! `oxlint`, through the package's JS plugin and Node add-on.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use webadeva_lint::diagnostic::{Format, Severity, render};
use webadeva_lint::{config, linter, rules};

const HELP: &str = "\
Usage: webadeva-lint [OPTIONS] [PATH]...

Test harness for the @webadeva/oxlint-config native rules. Reads the rules to
run from `settings.webadeva.rules` in .oxlintrc.json (ESLint-style values).

Options:
  -c, --config <FILE>      Config file (default: .oxlintrc.json in the cwd)
  -f, --format <FORMAT>    default | unix | json | github
      --fix                Apply fixes like `eslint --fix` (up to 10 passes)
      --quiet              Report errors only
      --rules              List the available rules
      --print-config <F>   Print the resolved rules and settings for a file
  -h, --help               Show this help
";

struct Args {
    config: Option<PathBuf>,
    format: Format,
    quiet: bool,
    fix: bool,
    list_rules: bool,
    print_config: Option<String>,
    paths: Vec<PathBuf>,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut args = Args {
        config: None,
        format: Format::Default,
        quiet: false,
        fix: false,
        list_rules: false,
        print_config: None,
        paths: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| -> Result<String, String> {
            inline
                .clone()
                .or_else(|| it.next())
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(None);
            }
            "-c" | "--config" => args.config = Some(PathBuf::from(value("--config")?)),
            "-f" | "--format" => {
                let v = value("--format")?;
                args.format = Format::parse(&v).ok_or_else(|| format!("unknown format {v:?}"))?;
            }
            "--quiet" => args.quiet = true,
            "--fix" => args.fix = true,
            "--rules" => args.list_rules = true,
            "--print-config" => args.print_config = Some(value("--print-config")?),
            f if f.starts_with('-') && f.len() > 1 => return Err(format!("unknown option {f}")),
            _ => args.paths.push(PathBuf::from(arg)),
        }
    }
    if args.paths.is_empty() {
        args.paths.push(PathBuf::from("."));
    }
    Ok(Some(args))
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(Some(a)) => a,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("webadeva-lint: {e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    let all_rules = rules::all();
    if args.list_rules {
        let mut names: Vec<_> = all_rules.iter().map(|r| r.name()).collect();
        names.sort_unstable();
        for n in names {
            println!("{n}");
        }
        return ExitCode::SUCCESS;
    }

    let root = std::env::current_dir().expect("cwd");
    let config = match config::load(args.config.as_deref(), &root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("webadeva-lint: {e}");
            return ExitCode::from(2);
        }
    };
    if let Some(file) = &args.print_config {
        let (rules, settings) = config.resolve(file);
        let json = serde_json::json!({ "rules": rules, "settings": settings });
        println!("{}", serde_json::to_string_pretty(&json).expect("json"));
        return ExitCode::SUCCESS;
    }
    let linter = match linter::Linter::new(config, &all_rules) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };

    let files = linter.collect_files(&args.paths);
    let mut diagnostics: Vec<_> = linter
        .lint_files(&files, args.fix)
        .into_iter()
        .flat_map(|r| r.diagnostics)
        .collect();
    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    if args.quiet {
        diagnostics.retain(|d| d.severity == Severity::Error);
    }
    let out = render(args.format, &diagnostics, files.len());
    let _ = std::io::stdout().lock().write_all(out.as_bytes());
    if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
