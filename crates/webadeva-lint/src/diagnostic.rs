//! Diagnostics and output formats (`default`, `unix`, `json`, `github`).

use std::fmt::Write as _;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    /// Byte offsets into the source.
    pub start: u32,
    pub end: u32,
    /// Autofix applied by `--fix`.
    pub fix: Option<crate::fix::Fix>,
}

/// A diagnostic resolved to a file and 1-based line/column.
#[derive(Debug, Clone, Serialize)]
pub struct Located {
    pub file: String,
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub line: usize,
    pub column: usize,
    #[serde(rename = "endLine")]
    pub end_line: usize,
    #[serde(rename = "endColumn")]
    pub end_column: usize,
}

/// Maps byte offsets to 1-based line and (UTF-16, like ESLint) column.
pub struct LineIndex<'a> {
    source: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut starts = vec![0];
        let bytes = source.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => starts.push(i + 1),
                b'\r' => {
                    if bytes.get(i + 1) == Some(&b'\n') {
                        i += 1;
                    }
                    starts.push(i + 1);
                }
                _ => {}
            }
            i += 1;
        }
        // U+2028 / U+2029 are line terminators in JS too.
        for (idx, ch) in source.char_indices() {
            if ch == '\u{2028}' || ch == '\u{2029}' {
                starts.push(idx + ch.len_utf8());
            }
        }
        starts.sort_unstable();
        Self { source, starts }
    }

    /// Number of lines (an empty trailing line after a final newline counts,
    /// like ESLint's `sourceCode.lines`).
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// Text of a 1-based line, without its terminator.
    pub fn line_text(&self, line: usize) -> &'a str {
        let start = self.starts[line - 1];
        let end = self.starts.get(line).copied().unwrap_or(self.source.len());
        self.source[start..end].trim_end_matches(['\n', '\r', '\u{2028}', '\u{2029}'])
    }

    /// 1-based line of a byte offset.
    pub fn line_of(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    }

    /// 1-based (line, column) of a byte offset; column counts UTF-16 units.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.source.len());
        let line = self.line_of(offset);
        let start = self.starts[line - 1];
        let col = self.source[start..offset].encode_utf16().count() + 1;
        (line, col)
    }
}

pub fn locate(file: &str, index: &LineIndex<'_>, d: &Diagnostic) -> Located {
    let (line, column) = index.position(d.start as usize);
    let (end_line, end_column) = index.position(d.end as usize);
    Located {
        file: file.to_string(),
        rule: d.rule.to_string(),
        severity: d.severity,
        message: d.message.clone(),
        line,
        column,
        end_line,
        end_column,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Default,
    Unix,
    Json,
    Github,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "default" | "stylish" => Self::Default,
            "unix" => Self::Unix,
            "json" => Self::Json,
            "github" => Self::Github,
            _ => return None,
        })
    }
}

pub fn render(format: Format, diags: &[Located], files_linted: usize) -> String {
    let mut out = String::new();
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diags.len() - errors;
    match format {
        Format::Json => {
            #[derive(Serialize)]
            struct Report<'a> {
                diagnostics: &'a [Located],
                #[serde(rename = "numberOfFiles")]
                number_of_files: usize,
                errors: usize,
                warnings: usize,
            }
            out = serde_json::to_string(&Report {
                diagnostics: diags,
                number_of_files: files_linted,
                errors,
                warnings,
            })
            .expect("serializable");
            out.push('\n');
        }
        Format::Unix => {
            for d in diags {
                let sev = match d.severity {
                    Severity::Error => "Error",
                    Severity::Warning => "Warning",
                };
                let _ = writeln!(
                    out,
                    "{}:{}:{}: {} [{}/{}]",
                    d.file, d.line, d.column, d.message, sev, d.rule
                );
            }
            if !diags.is_empty() {
                let _ = writeln!(out, "\n{} problems", diags.len());
            }
        }
        Format::Github => {
            for d in diags {
                let level = match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                };
                let _ = writeln!(
                    out,
                    "::{level} file={},line={},col={},endLine={},endColumn={},title={}::{}",
                    d.file, d.line, d.column, d.end_line, d.end_column, d.rule, d.message
                );
            }
        }
        Format::Default => {
            let mut current: Option<&str> = None;
            for d in diags {
                if current != Some(d.file.as_str()) {
                    if current.is_some() {
                        out.push('\n');
                    }
                    let _ = writeln!(out, "{}", d.file);
                    current = Some(d.file.as_str());
                }
                let sev = match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                };
                let _ = writeln!(
                    out,
                    "  {:>4}:{:<4} {:<7}  {}  {}",
                    d.line, d.column, sev, d.message, d.rule
                );
            }
            let _ = writeln!(
                out,
                "{}Found {} warning{} and {} error{} in {} file{}.",
                if diags.is_empty() { "" } else { "\n" },
                warnings,
                if warnings == 1 { "" } else { "s" },
                errors,
                if errors == 1 { "" } else { "s" },
                files_linted,
                if files_linted == 1 { "" } else { "s" },
            );
        }
    }
    out
}
