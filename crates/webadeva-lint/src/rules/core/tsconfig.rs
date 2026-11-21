//! Which files a tsconfig includes, following TypeScript's own rules
//! (`getParsedCommandLineOfConfigFile`): JSONC, `extends` chains (relative and
//! package paths), `files` / `include` / `exclude` with TypeScript's wildcard
//! semantics, the default include and exclude, extension priority, and
//! solution-style `references`.
//!
//! Instead of enumerating every file like TypeScript, a file is tested against
//! the parsed specs, which gives the same answer for files that exist.
//! Parsed configs are cached for the whole process (rules run on many threads).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;

// ---------------------------------------------------------------------------
// Paths

/// TypeScript's `getNormalizedAbsolutePath`: resolve `.` and `..` lexically.
pub fn normalize(path: &Path) -> PathBuf {
    let mut parts: Vec<&str> = Vec::new();
    let s = path.to_string_lossy();
    let s = s.replace('\\', "/");
    for comp in s.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    let mut out = String::from("/");
    out.push_str(&parts.join("/"));
    PathBuf::from(out)
}

fn join(base: &Path, rel: &str) -> PathBuf {
    if rel.starts_with('/') {
        normalize(Path::new(rel))
    } else {
        normalize(&base.join(rel))
    }
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

// ---------------------------------------------------------------------------
// JSONC

fn strip_trailing_commas(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut in_str = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 1;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if c == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if !(j < chars.len() && (chars[j] == '}' || chars[j] == ']')) {
                out.push(c);
            }
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// Reads a tsconfig-style JSONC file. `None` when unreadable or invalid.
pub fn read_jsonc(path: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
    let mut stripped = String::new();
    std::io::Read::read_to_string(
        &mut json_comments::StripComments::new(raw.as_bytes()),
        &mut stripped,
    )
    .ok()?;
    serde_json::from_str(&strip_trailing_commas(&stripped)).ok()
}

// ---------------------------------------------------------------------------
// TypeScript wildcard specs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Usage {
    Files,
    Exclude,
}

const PACKAGE_FOLDERS: &[&str] = &["node_modules", "bower_components", "jspm_packages"];

#[derive(Debug, Clone)]
enum Comp {
    /// `**`
    Recursive,
    /// A path component, possibly with `*` / `?`.
    Glob(String),
}

#[derive(Debug, Clone)]
struct Spec {
    comps: Vec<Comp>,
    usage: Usage,
}

fn has_wildcard_or_special(c: &str) -> bool {
    // `componentPattern !== component`: anything regExpEscape changes.
    c.chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch.is_whitespace() || ch == '/'))
}

impl Spec {
    /// `getSubPatternFromSpec`; `None` for an invalid include spec.
    fn new(spec: &str, base: &Path, usage: Usage) -> Option<Self> {
        let abs = join(base, spec);
        let mut parts: Vec<String> = path_str(&abs)
            .split('/')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let last = parts.last().cloned().unwrap_or_default();
        if usage != Usage::Exclude && last == "**" {
            return None;
        }
        // isImplicitGlob
        if !last.contains(['.', '*', '?']) {
            parts.push("**".into());
            parts.push("*".into());
        }
        let comps = parts
            .into_iter()
            .map(|p| {
                if p == "**" {
                    Comp::Recursive
                } else {
                    Comp::Glob(p)
                }
            })
            .collect();
        Some(Self { comps, usage })
    }

    fn matches(&self, path: &Path) -> bool {
        let s = path_str(path);
        let parts: Vec<&str> = s.split('/').filter(|s| !s.is_empty()).collect();
        match_comps(&self.comps, &parts, self.usage, &s)
    }
}

/// Matches spec components against path components. For `Exclude`, a spec
/// matching a prefix of the path matches (`(?:$|/)` terminator).
fn match_comps(comps: &[Comp], parts: &[&str], usage: Usage, whole: &str) -> bool {
    let Some((first, rest)) = comps.split_first() else {
        return parts.is_empty() || usage == Usage::Exclude;
    };
    match first {
        Comp::Recursive => {
            // Zero or more directories; for includes, none starting with `.`
            // and none of the common package folders.
            for skip in 0..=parts.len() {
                if match_comps(rest, &parts[skip..], usage, whole) {
                    return true;
                }
                if skip < parts.len() && usage != Usage::Exclude {
                    let p = parts[skip];
                    if p.starts_with('.') || PACKAGE_FOLDERS.contains(&p) {
                        return false;
                    }
                }
            }
            false
        }
        Comp::Glob(g) => {
            let Some((part, others)) = parts.split_first() else {
                return false;
            };
            let is_last_path_comp = others.is_empty();
            if usage != Usage::Exclude
                && has_wildcard_or_special(g)
                && PACKAGE_FOLDERS.contains(part)
            {
                return false;
            }
            glob_component(g, part, usage, is_last_path_comp && whole.ends_with(part))
                && match_comps(rest, others, usage, whole)
        }
    }
}

/// One component: `*` = any run of non-`/`, `?` = one char; for includes a
/// leading `*` or `?` cannot match a leading `.`, and `*` never matches the
/// `.` of a trailing `.min.js`.
fn glob_component(glob: &str, text: &str, usage: Usage, at_end: bool) -> bool {
    let g: Vec<char> = glob.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let include = usage != Usage::Exclude;
    let tail_is_min_js = |i: usize| -> bool {
        at_end && t[i] == '.' && t[i + 1..].iter().collect::<String>() == "min.js"
    };
    // memoized recursion
    fn go(
        g: &[char],
        t: &[char],
        gi: usize,
        ti: usize,
        include: bool,
        min: &dyn Fn(usize) -> bool,
        memo: &mut HashMap<(usize, usize), bool>,
    ) -> bool {
        if let Some(&r) = memo.get(&(gi, ti)) {
            return r;
        }
        let r = if gi == g.len() {
            ti == t.len()
        } else {
            match g[gi] {
                '*' => {
                    let leading = include && gi == 0;
                    let mut ok = go(g, t, gi + 1, ti, include, min, memo);
                    let mut k = ti;
                    while !ok && k < t.len() {
                        if leading && k == ti && t[k] == '.' {
                            break;
                        }
                        if include && min(k) {
                            break;
                        }
                        k += 1;
                        ok = go(g, t, gi + 1, k, include, min, memo);
                    }
                    ok
                }
                '?' => {
                    ti < t.len()
                        && !(include && gi == 0 && t[ti] == '.')
                        && go(g, t, gi + 1, ti + 1, include, min, memo)
                }
                c => ti < t.len() && t[ti] == c && go(g, t, gi + 1, ti + 1, include, min, memo),
            }
        };
        memo.insert((gi, ti), r);
        r
    }
    let mut memo = HashMap::new();
    go(&g, &t, 0, 0, include, &tail_is_min_js, &mut memo)
}

/// `/(?:^|\/)\*\*\/?$/`
fn invalid_trailing_recursion(spec: &str) -> bool {
    let s = spec.strip_suffix('/').unwrap_or(spec);
    s == "**" || s.ends_with("/**")
}

/// `invalidDotDotAfterRecursiveWildcard`
fn invalid_dot_dot_after_recursive(s: &str) -> bool {
    let wildcard = if s.starts_with("**/") {
        Some(0)
    } else {
        s.find("/**/")
    };
    let Some(wildcard) = wildcard else {
        return false;
    };
    let last_dot = if s.ends_with("/..") {
        Some(s.len())
    } else {
        s.rfind("/../")
    };
    last_dot.is_some_and(|d| d > wildcard)
}

// ---------------------------------------------------------------------------
// tsconfig parsing

#[derive(Debug, Default)]
pub struct Project {
    literal: HashSet<PathBuf>,
    includes: Vec<Spec>,
    excludes: Vec<Spec>,
    references: Vec<PathBuf>,
}

/// The parts of a parsed config that matter here (after `extends`).
#[derive(Debug, Default, Clone)]
struct Raw {
    files: Option<Value>,
    include: Option<Value>,
    exclude: Option<Value>,
    /// `outDir` / `declarationDir`, already absolute (or `${configDir}`-relative).
    out_dir: Option<String>,
    declaration_dir: Option<String>,
}

const CONFIG_DIR: &str = "${configDir}";

fn starts_with_config_dir(s: &str) -> bool {
    s.len() >= CONFIG_DIR.len() && s[..CONFIG_DIR.len()].eq_ignore_ascii_case(CONFIG_DIR)
}

/// `getSubstitutedPathWithConfigDirTemplate`
fn substitute_config_dir(s: &str, base: &Path) -> String {
    let replaced = s.replacen(&s[..CONFIG_DIR.len()], "./", 1);
    path_str(&join(base, &replaced))
}

fn file_exists(p: &Path) -> bool {
    p.is_file()
}

/// `getExtendsConfigPath`
fn extends_path(spec: &str, base: &Path) -> Option<PathBuf> {
    let spec = spec.replace('\\', "/");
    if spec.starts_with('/') || spec.starts_with("./") || spec.starts_with("../") {
        let mut p = join(base, &spec);
        if !file_exists(&p) && !spec.ends_with(".json") {
            p = PathBuf::from(format!("{}.json", path_str(&p)));
            if !file_exists(&p) {
                return None;
            }
        }
        return Some(p);
    }
    if spec.is_empty() {
        return None;
    }
    resolve_package_config(&spec, base)
}

/// Node-style lookup of a tsconfig in a package (`nodeNextJsonConfigResolver`).
fn resolve_package_config(spec: &str, base: &Path) -> Option<PathBuf> {
    let mut segs = spec.splitn(if spec.starts_with('@') { 3 } else { 2 }, '/');
    let name = if spec.starts_with('@') {
        format!("{}/{}", segs.next()?, segs.next()?)
    } else {
        segs.next()?.to_string()
    };
    let subpath = segs.next().unwrap_or("");
    let mut dir = Some(base.to_path_buf());
    while let Some(d) = dir {
        let pkg = d.join("node_modules").join(&name);
        if pkg.is_dir() {
            let manifest = std::fs::read_to_string(pkg.join("package.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok());
            if let Some(exports) = manifest.as_ref().and_then(|m| m.get("exports")) {
                let key = if subpath.is_empty() {
                    ".".to_string()
                } else {
                    format!("./{subpath}")
                };
                if let Some(target) = resolve_exports(exports, &key) {
                    let p = join(&pkg, &target);
                    if file_exists(&p) {
                        return Some(p);
                    }
                }
                return None;
            }
            if subpath.is_empty() {
                let field = manifest
                    .as_ref()
                    .and_then(|m| m.get("tsconfig"))
                    .and_then(Value::as_str)
                    .map(|s| join(&pkg, s));
                if let Some(p) = field.filter(|p| file_exists(p)) {
                    return Some(p);
                }
                let p = pkg.join("tsconfig.json");
                return file_exists(&p).then_some(p);
            }
            let p = join(&pkg, subpath);
            if file_exists(&p) {
                return Some(p);
            }
            let with_json = PathBuf::from(format!("{}.json", path_str(&p)));
            if file_exists(&with_json) {
                return Some(with_json);
            }
            let in_dir = p.join("tsconfig.json");
            if file_exists(&in_dir) {
                return Some(in_dir);
            }
            return None;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    None
}

fn resolve_exports(exports: &Value, key: &str) -> Option<String> {
    const CONDITIONS: &[&str] = &["node", "require", "types", "default"];
    fn target(v: &Value, star: Option<&str>) -> Option<String> {
        match v {
            Value::String(s) => Some(match star {
                Some(m) => s.replace('*', m),
                None => s.clone(),
            }),
            Value::Object(m) => m
                .iter()
                .filter(|(k, _)| CONDITIONS.contains(&k.as_str()))
                .find_map(|(_, v)| target(v, star)),
            Value::Array(a) => a.iter().find_map(|v| target(v, star)),
            _ => None,
        }
    }
    match exports {
        Value::Object(m) if m.keys().any(|k| k.starts_with('.')) => {
            if let Some(v) = m.get(key) {
                return target(v, None);
            }
            // Subpath patterns: the longest matching prefix wins.
            let mut best: Option<(&String, &Value)> = None;
            for (k, v) in m {
                if let Some((pre, post)) = k.split_once('*')
                    && key.starts_with(pre)
                    && key.ends_with(post)
                    && key.len() >= pre.len() + post.len()
                    && best.is_none_or(|(b, _)| k.len() > b.len())
                {
                    best = Some((k, v));
                }
            }
            let (k, v) = best?;
            let (pre, post) = k.split_once('*')?;
            target(v, Some(&key[pre.len()..key.len() - post.len()]))
        }
        other if key == "." => target(other, None),
        _ => None,
    }
}

/// `parseConfig`: own raw values with `extends` applied. `stack` guards
/// against cycles.
fn parse_raw(path: &Path, stack: &mut Vec<PathBuf>) -> Option<Raw> {
    let json = read_jsonc(path)?;
    let Value::Object(obj) = json else {
        return Some(Raw::default());
    };
    let base = path.parent().unwrap_or(Path::new("/")).to_path_buf();
    let mut own = Raw {
        files: obj.get("files").cloned(),
        include: obj.get("include").cloned(),
        exclude: obj.get("exclude").cloned(),
        ..Raw::default()
    };
    let opts = obj.get("compilerOptions").and_then(Value::as_object);
    let dir_opt = |name: &str| -> Option<String> {
        let v = opts?.get(name)?.as_str()?;
        Some(if starts_with_config_dir(v) {
            v.to_string()
        } else {
            path_str(&join(&base, v))
        })
    };
    own.out_dir = dir_opt("outDir");
    own.declaration_dir = dir_opt("declarationDir");
    let explicit_out = opts.is_some_and(|o| o.contains_key("outDir"));
    let explicit_decl = opts.is_some_and(|o| o.contains_key("declarationDir"));

    if stack.contains(&path.to_path_buf()) {
        return Some(own);
    }
    let extends: Vec<String> = match obj.get("extends") {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    };
    if !extends.is_empty() {
        stack.push(path.to_path_buf());
        let mut result = Raw::default();
        for e in extends {
            let Some(ext_path) = extends_path(&e, &base) else {
                continue;
            };
            let Some(ext) = parse_raw(&ext_path, stack) else {
                continue;
            };
            let ext_dir = ext_path.parent().unwrap_or(Path::new("/")).to_path_buf();
            // Inherited specs are relative to the extended config's directory.
            let rebase = |v: &Value| -> Value {
                match v {
                    Value::Array(items) => Value::Array(
                        items
                            .iter()
                            .map(|it| match it.as_str() {
                                Some(s) if starts_with_config_dir(s) || s.starts_with('/') => {
                                    it.clone()
                                }
                                Some(s) => Value::String(path_str(&join(&ext_dir, s))),
                                None => it.clone(),
                            })
                            .collect(),
                    ),
                    other => other.clone(),
                }
            };
            let truthy = |v: &Option<Value>| matches!(v, Some(v) if !(v.is_null() || v == &Value::Bool(false) || v == &Value::from(0) || v == &Value::from("")));
            if !truthy(&own.include) && truthy(&ext.include) {
                result.include = ext.include.as_ref().map(rebase);
            }
            if !truthy(&own.exclude) && truthy(&ext.exclude) {
                result.exclude = ext.exclude.as_ref().map(rebase);
            }
            if !truthy(&own.files) && truthy(&ext.files) {
                result.files = ext.files.as_ref().map(rebase);
            }
            if ext.out_dir.is_some() {
                result.out_dir = ext.out_dir;
            }
            if ext.declaration_dir.is_some() {
                result.declaration_dir = ext.declaration_dir;
            }
        }
        stack.pop();
        if result.include.is_some() {
            own.include = result.include;
        }
        if result.exclude.is_some() {
            own.exclude = result.exclude;
        }
        if result.files.is_some() {
            own.files = result.files;
        }
        if !explicit_out {
            own.out_dir = result.out_dir;
        }
        if !explicit_decl {
            own.declaration_dir = result.declaration_dir;
        }
    }
    Some(own)
}

fn string_list(v: &Option<Value>) -> Option<Vec<String>> {
    // `toPropValue`: only an array counts; `null` is "no prop".
    match v {
        Some(Value::Array(a)) => Some(
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect(),
        ),
        _ => None,
    }
}

fn load_project(config: &Path) -> Project {
    let mut stack = Vec::new();
    let Some(raw) = parse_raw(config, &mut stack) else {
        return Project::default();
    };
    let base = config.parent().unwrap_or(Path::new("/")).to_path_buf();
    let sub = |s: String| {
        if starts_with_config_dir(&s) {
            substitute_config_dir(&s, &base)
        } else {
            s
        }
    };

    let files = string_list(&raw.files);
    let mut include = string_list(&raw.include);
    let exclude_prop = matches!(&raw.exclude, Some(v) if !v.is_null());
    let mut exclude = string_list(&raw.exclude);
    if !exclude_prop {
        let dirs: Vec<String> = [raw.out_dir.clone(), raw.declaration_dir.clone()]
            .into_iter()
            .flatten()
            .collect();
        if !dirs.is_empty() {
            exclude = Some(dirs);
        }
    }
    if files.is_none() && include.is_none() {
        include = Some(vec!["**/*".into()]);
    }
    let mut project = Project::default();
    for f in files.unwrap_or_default() {
        project.literal.insert(join(&base, &sub(f)));
    }
    for s in include.unwrap_or_default() {
        if invalid_trailing_recursion(&s) || invalid_dot_dot_after_recursive(&s) {
            continue;
        }
        if let Some(spec) = Spec::new(&sub(s), &base, Usage::Files) {
            project.includes.push(spec);
        }
    }
    for s in exclude.unwrap_or_default() {
        if invalid_dot_dot_after_recursive(&s) {
            continue;
        }
        if let Some(spec) = Spec::new(&sub(s), &base, Usage::Exclude) {
            project.excludes.push(spec);
        }
    }
    // `references` are never inherited through `extends`.
    if let Some(Value::Object(obj)) = read_jsonc(config)
        && let Some(Value::Array(refs)) = obj.get("references")
    {
        for r in refs {
            if let Some(p) = r.get("path").and_then(Value::as_str) {
                let p = join(&base, p);
                let p = if path_str(&p).ends_with(".json") {
                    p
                } else {
                    p.join("tsconfig.json")
                };
                project.references.push(p);
            }
        }
    }
    project
}

/// TypeScript's supported extension groups, highest priority first.
const TS_GROUPS: &[&[&str]] = &[
    &[".ts", ".tsx", ".d.ts"],
    &[".cts", ".d.cts"],
    &[".mts", ".d.mts"],
];

/// `changeExtension`, aware of `.d.ts`-style extensions.
fn change_extension(path: &str, ext: &str) -> String {
    for e in [
        ".d.ts", ".d.cts", ".d.mts", ".ts", ".tsx", ".cts", ".mts", ".js", ".jsx", ".mjs", ".cjs",
        ".json",
    ] {
        if let Some(stem) = path.strip_suffix(e) {
            return format!("{stem}{ext}");
        }
    }
    format!("{path}{ext}")
}

impl Project {
    fn wildcard_matches(&self, path: &Path) -> bool {
        self.includes.iter().any(|s| s.matches(path))
            && !self.excludes.iter().any(|s| s.matches(path))
    }

    /// Whether `getFileNamesFromConfigSpecs` would list `path` (an existing file).
    fn contains(&self, path: &Path) -> bool {
        if self.literal.contains(path) {
            return true;
        }
        let s = path_str(path);
        let groups = TS_GROUPS;
        let Some(group) = groups.iter().find(|g| g.iter().any(|e| s.ends_with(e))) else {
            // Only TypeScript files are asked about (allowJs is irrelevant).
            return false;
        };
        if !self.wildcard_matches(path) {
            return false;
        }
        // A sibling with a higher-priority extension wins.
        for ext in group.iter() {
            if s.ends_with(ext) && (*ext != ".ts" || !s.ends_with(".d.ts")) {
                return true;
            }
            let higher = PathBuf::from(change_extension(&s, ext));
            if self.literal.contains(&higher)
                || (higher.is_file() && self.wildcard_matches(&higher))
            {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Caches and the public API

fn projects() -> &'static Mutex<HashMap<PathBuf, Arc<Project>>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<Project>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn nearest_cache() -> &'static Mutex<HashMap<PathBuf, Option<PathBuf>>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<PathBuf>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn project(config: &Path) -> Option<Arc<Project>> {
    if let Some(p) = projects().lock().ok()?.get(config) {
        return Some(p.clone());
    }
    // Parse outside the lock; a duplicate parse on a race is harmless.
    let p = if config.is_file() {
        Arc::new(load_project(config))
    } else {
        Arc::new(Project::default())
    };
    projects()
        .lock()
        .ok()?
        .insert(config.to_path_buf(), p.clone());
    Some(p)
}

/// The closest `tsconfig.json` walking up from `dir`, like the TypeScript
/// project service.
pub fn find_nearest_config(dir: &Path) -> Option<PathBuf> {
    if let Some(found) = nearest_cache().lock().ok()?.get(dir) {
        return found.clone();
    }
    let candidate = dir.join("tsconfig.json");
    let found = if candidate.is_file() {
        Some(candidate)
    } else {
        dir.parent().and_then(find_nearest_config)
    };
    if let Ok(mut c) = nearest_cache().lock() {
        c.insert(dir.to_path_buf(), found.clone());
    }
    found
}

fn in_config(config: &Path, file: &Path, seen: &mut HashSet<PathBuf>) -> bool {
    if !seen.insert(config.to_path_buf()) {
        return false;
    }
    let Some(project) = project(config) else {
        return false;
    };
    if project.contains(file) {
        return true;
    }
    // Solution-style configs list their files through project references.
    project.references.iter().any(|r| in_config(r, file, seen))
}

/// `isInProject`: whether the nearest tsconfig (or one it references)
/// includes `file`.
pub fn is_in_project(file: &Path) -> bool {
    let file = normalize(file);
    let Some(config) = file.parent().and_then(find_nearest_config) else {
        return false;
    };
    in_config(&config, &file, &mut HashSet::new())
}

/// `defaultProjectFiles`: root-level files no tsconfig includes, which
/// typescript-eslint types through its default project.
pub fn default_project_files(root: &Path) -> Arc<HashSet<PathBuf>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<HashSet<PathBuf>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(root).cloned()) {
        return v;
    }
    let with_jsx = has_react_types(root);
    let mut names: Vec<String> = std::fs::read_dir(root)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .filter(|n| is_default_project_name(n, with_jsx))
                .filter(|n| !n.chars().any(|c| "*?[]{}()!+@\\".contains(c)))
                .filter(|n| !is_in_project(&root.join(n)))
                .collect()
        })
        .unwrap_or_default();
    // JS sorts by UTF-16 code units.
    names.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    names.truncate(8);
    let set: HashSet<PathBuf> = names
        .into_iter()
        .map(|n| normalize(&root.join(n)))
        .collect();
    let set = Arc::new(set);
    if let Ok(mut c) = cache.lock() {
        c.insert(root.to_path_buf(), set.clone());
    }
    set
}

/// `/\.[cm]?[jt]s$/`, or `/\.[cm]?[jt]sx?$/` with React's types.
fn is_default_project_name(name: &str, jsx: bool) -> bool {
    let name = if jsx {
        name.strip_suffix('x').unwrap_or(name)
    } else {
        name
    };
    let Some(dot) = name.rfind('.') else {
        return false;
    };
    matches!(
        &name[dot + 1..],
        "js" | "ts" | "cjs" | "mjs" | "cts" | "mts"
    )
}

/// Whether `@types/react/package.json` resolves from `root`.
fn has_react_types(root: &Path) -> bool {
    let mut dir = Some(root);
    while let Some(d) = dir {
        if d.join("node_modules/@types/react/package.json").is_file() {
            return true;
        }
        dir = d.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inc(spec: &str, path: &str) -> bool {
        Spec::new(spec, Path::new("/p"), Usage::Files).is_some_and(|s| s.matches(Path::new(path)))
    }
    fn exc(spec: &str, path: &str) -> bool {
        Spec::new(spec, Path::new("/p"), Usage::Exclude).is_some_and(|s| s.matches(Path::new(path)))
    }

    #[test]
    fn wildcards() {
        assert!(inc("src", "/p/src/a.ts"));
        assert!(inc("src", "/p/src/x/y/a.ts"));
        assert!(!inc("src", "/p/src/.hidden/a.ts"));
        assert!(!inc("src", "/p/src/node_modules/a.ts"));
        assert!(inc("**/*", "/p/a.ts"));
        assert!(!inc("**/*", "/p/.a/a.ts"));
        assert!(inc("src/*.ts", "/p/src/a.ts"));
        assert!(!inc("src/*.ts", "/p/src/x/a.ts"));
        assert!(inc("src/?.ts", "/p/src/a.ts"));
        assert!(!inc("src/?.ts", "/p/src/ab.ts"));
        assert!(inc("./src/../lib/**/*", "/p/lib/a.ts"));
        assert!(inc("src/.hidden/a.ts", "/p/src/.hidden/a.ts"));
        assert!(!inc("**", "/p/a.ts"));
        assert!(!inc("*", "/p/a.min.js"));
        assert!(exc("dist", "/p/dist/a.ts"));
        assert!(exc("**/test", "/p/x/.y/test/a.ts"));
        assert!(exc("src/*.ts", "/p/src/a.ts"));
        assert!(!exc("dist", "/p/distx/a.ts"));
    }
}
