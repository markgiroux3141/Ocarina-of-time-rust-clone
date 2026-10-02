//! Minimal reader for static C initializers in decomp source, so tables like Player's
//! display-list groups are taken from `z_player_lib.c` rather than copied by hand.
//!
//! Handles `type name[..][..] = { ... };` with nested braces, identifiers, numbers and
//! comments. [`read_c`] gives the source as gc-eu-mq-dbg builds it: its preprocessor
//! conditionals evaluated ([`preprocess`]), `FRAMERATE_CONST` resolved; otherwise the first
//! definition wins, which for the decomp is the non-`AVOID_UB` (as-shipped) variant.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// The defines a gc-eu-mq-dbg build has: the Makefile's (`-DPLATFORM_GC=1 -DOOT_VERSION=GC_EU_MQ_DBG
/// -DOOT_REVISION=15 -DOOT_REGION=REGION_EU -DDEBUG_FEATURES=1 ...`), the version and region
/// numbers of `include/versions.h` and `include/region.h`, and what `versions.h` derives (PAL,
/// not the 50 Hz N64 PAL, Master Quest, the debug ROM's assets).
pub const VERSION_DEFINES: &[(&str, i64)] = &[
    ("PLATFORM_N64", 0),
    ("PLATFORM_GC", 1),
    ("PLATFORM_IQUE", 0),
    ("NTSC_1_0", 1),
    ("NTSC_1_1", 2),
    ("PAL_1_0", 3),
    ("NTSC_1_2", 4),
    ("PAL_1_1", 5),
    ("GC_JP", 6),
    ("GC_JP_MQ", 7),
    ("GC_US", 8),
    ("GC_US_MQ", 9),
    ("GC_EU_DBG_2", 10),
    ("GC_EU_MQ_DBG", 11),
    ("GC_EU_DBG", 12),
    ("GC_EU", 13),
    ("GC_EU_MQ", 14),
    ("GC_JP_CE", 15),
    ("IQUE_CN", 16),
    ("OOT_VERSION", 11),
    ("OOT_REVISION", 15),
    ("REGION_NULL", 0),
    ("REGION_JP", 1),
    ("REGION_US", 2),
    ("REGION_EU", 3),
    ("REGION_CN", 4),
    ("OOT_REGION", 3),
    ("LIBULTRA_VERSION_D", 4),
    ("LIBULTRA_VERSION_E", 5),
    ("LIBULTRA_VERSION_F", 6),
    ("LIBULTRA_VERSION_G", 7),
    ("LIBULTRA_VERSION_H", 8),
    ("LIBULTRA_VERSION_I", 9),
    ("LIBULTRA_VERSION_J", 10),
    ("LIBULTRA_VERSION_K", 11),
    ("LIBULTRA_VERSION_L", 12),
    ("LIBULTRA_VERSION", 12),
    ("LIBULTRA_PATCH", 0),
    ("DEBUG_FEATURES", 1),
    ("F3DEX_GBI_2", 1),
    ("F3DEX_GBI_PL", 1),
    ("GBI_DOWHILE", 1),
    ("GBI_DEBUG", 1),
    ("_LANGUAGE_C", 1),
    ("__sgi", 1),
    ("_MIPS_SZLONG", 32),
    ("OOT_NTSC", 0),
    ("OOT_PAL", 1),
    ("OOT_PAL_N64", 0),
    ("OOT_MQ", 1),
    ("DEBUG_ASSETS", 1),
];

/// A decomp C file as gc-eu-mq-dbg compiles it, for the readers here: comments stripped, the
/// preprocessor's conditionals evaluated ([`preprocess`]), and `FRAMERATE_CONST(a, b)` (the
/// 60 Hz value unless `OOT_PAL_N64`, `include/versions.h`) as `(a)`. Lines are kept.
pub fn read_c(decomp: &Path, rel: &str) -> Result<String> {
    let p = decomp.join(rel);
    let text = std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
    Ok(prepare(&text))
}

/// [`read_c`] on source already read.
pub fn prepare(text: &str) -> String {
    framerate_const(&preprocess(&strip_comments(text)))
}

/// `FRAMERATE_CONST(value60Hz, value50Hz)` as `(value60Hz)`.
fn framerate_const(src: &str) -> String {
    const M: &str = "FRAMERATE_CONST(";
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(at) = rest.find(M) {
        let (before, after) = rest.split_at(at);
        // Not the macro's own #define.
        if before.trim_end().ends_with("#define") {
            out.push_str(&rest[..at + M.len()]);
            rest = &rest[at + M.len()..];
            continue;
        }
        out.push_str(before);
        let args = &after[M.len()..];
        let (mut depth, mut comma, mut end) = (0i32, None, None);
        for (i, c) in args.char_indices() {
            match c {
                '(' => depth += 1,
                ')' if depth == 0 => {
                    end = Some(i);
                    break;
                }
                ')' => depth -= 1,
                ',' if depth == 0 && comma.is_none() => comma = Some(i),
                _ => {}
            }
        }
        match (comma, end) {
            (Some(c), Some(e)) => {
                out.push('(');
                out.push_str(args[..c].trim());
                out.push(')');
                rest = &args[e + 1..];
            }
            _ => {
                out.push_str(M);
                rest = args;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Drops the lines of the `#if` / `#ifdef` / `#ifndef` / `#elif` / `#else` branches a
/// gc-eu-mq-dbg build doesn't take ([`VERSION_DEFINES`]), and the directives; the line count
/// stays. Expect comments stripped first.
pub fn preprocess(src: &str) -> String {
    let defines: HashMap<&str, i64> = VERSION_DEFINES.iter().copied().collect();
    // (taking this branch, a branch was taken, the enclosing block is live)
    let mut stack: Vec<(bool, bool, bool)> = Vec::new();
    let live = |stack: &[(bool, bool, bool)]| stack.iter().all(|s| s.0);
    let mut out = String::with_capacity(src.len());
    for line in src.split_inclusive('\n') {
        let t = line.trim_start();
        let directive = t.strip_prefix('#').map(|d| d.trim_start());
        let word = directive.map(|d| d.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').next().unwrap_or(""));
        let handled = match (directive, word) {
            (Some(d), Some(w @ ("if" | "ifdef" | "ifndef"))) => {
                let rest = d[w.len()..].trim();
                let c = match w {
                    "if" => cond(rest, &defines),
                    "ifdef" => defines.contains_key(rest.split_whitespace().next().unwrap_or("")),
                    _ => !defines.contains_key(rest.split_whitespace().next().unwrap_or("")),
                };
                let parent = live(&stack);
                stack.push((c, c, parent));
                true
            }
            (Some(d), Some("elif")) => {
                if let Some((_, taken, parent)) = stack.pop() {
                    let c = !taken && cond(d["elif".len()..].trim(), &defines);
                    stack.push((c, taken || c, parent));
                }
                true
            }
            (Some(_), Some("else")) => {
                if let Some((_, taken, parent)) = stack.pop() {
                    stack.push((!taken, true, parent));
                }
                true
            }
            (Some(_), Some("endif")) => {
                stack.pop();
                true
            }
            _ => false,
        };
        if handled || !live(&stack) {
            if line.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push_str(line);
        }
    }
    out
}

/// A `#if` expression: integers, names (`defines`, else 0), `defined(X)`, `!`, `&&`, `||`,
/// comparisons, `+ - * / % & | << >>`, parentheses.
fn cond(expr: &str, defines: &HashMap<&str, i64>) -> bool {
    #[derive(Debug, Clone, PartialEq)]
    enum T {
        N(i64),
        Op(&'static str),
    }
    let b = expr.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    const OPS: [&str; 18] = ["&&", "||", "==", "!=", "<=", ">=", "<<", ">>", "<", ">", "!", "+", "-", "*", "/", "%", "&", "|"];
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            toks.push(T::N(parse_int(&expr[s..i]).unwrap_or(0)));
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let name = &expr[s..i];
            if name == "defined" {
                let rest = expr[i..].trim_start();
                let (inner, used) = if let Some(r) = rest.strip_prefix('(') {
                    let close = r.find(')').unwrap_or(r.len());
                    (r[..close].trim(), expr.len() - rest.len() + close + 2)
                } else {
                    let n = rest.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').next().unwrap_or("");
                    (n, expr.len() - rest.len() + n.len())
                };
                toks.push(T::N(defines.contains_key(inner) as i64));
                i = used.min(b.len());
            } else {
                toks.push(T::N(*defines.get(name).unwrap_or(&0)));
            }
        } else if c == b'(' {
            toks.push(T::Op("("));
            i += 1;
        } else if c == b')' {
            toks.push(T::Op(")"));
            i += 1;
        } else if let Some(op) = OPS.iter().find(|o| expr[i..].starts_with(**o)) {
            toks.push(T::Op(op));
            i += op.len();
        } else {
            i += 1;
        }
    }
    // Precedence climbing.
    fn prec(op: &str) -> Option<u8> {
        Some(match op {
            "||" => 1,
            "&&" => 2,
            "|" => 3,
            "&" => 4,
            "==" | "!=" => 5,
            "<" | ">" | "<=" | ">=" => 6,
            "<<" | ">>" => 7,
            "+" | "-" => 8,
            "*" | "/" | "%" => 9,
            _ => return None,
        })
    }
    fn unary(t: &[T], i: &mut usize) -> i64 {
        match t.get(*i) {
            Some(T::Op("!")) => {
                *i += 1;
                (unary(t, i) == 0) as i64
            }
            Some(T::Op("-")) => {
                *i += 1;
                -unary(t, i)
            }
            Some(T::Op("(")) => {
                *i += 1;
                let v = binary(t, i, 0);
                if t.get(*i) == Some(&T::Op(")")) {
                    *i += 1;
                }
                v
            }
            Some(T::N(n)) => {
                *i += 1;
                *n
            }
            _ => {
                *i += 1;
                0
            }
        }
    }
    fn binary(t: &[T], i: &mut usize, min: u8) -> i64 {
        let mut l = unary(t, i);
        while let Some(T::Op(op)) = t.get(*i) {
            let Some(p) = prec(op) else { break };
            if p < min {
                break;
            }
            *i += 1;
            let r = binary(t, i, p + 1);
            l = match *op {
                "||" => (l != 0 || r != 0) as i64,
                "&&" => (l != 0 && r != 0) as i64,
                "|" => l | r,
                "&" => l & r,
                "==" => (l == r) as i64,
                "!=" => (l != r) as i64,
                "<" => (l < r) as i64,
                ">" => (l > r) as i64,
                "<=" => (l <= r) as i64,
                ">=" => (l >= r) as i64,
                "<<" => l << r,
                ">>" => l >> r,
                "+" => l + r,
                "-" => l - r,
                "*" => l * r,
                "/" => l.checked_div(r).unwrap_or(0),
                _ => l.checked_rem(r).unwrap_or(0),
            };
        }
        l
    }
    let mut i = 0;
    binary(&toks, &mut i, 0) != 0
}

#[derive(Debug, Clone, PartialEq)]
pub enum Init {
    Atom(String),
    List(Vec<Init>),
}

impl Init {
    pub fn atom(&self) -> Option<&str> {
        match self {
            Init::Atom(s) => Some(s),
            Init::List(_) => None,
        }
    }
    pub fn list(&self) -> &[Init] {
        match self {
            Init::List(v) => v,
            Init::Atom(_) => &[],
        }
    }
    /// All atoms in order, ignoring nesting.
    pub fn flatten(&self) -> Vec<String> {
        match self {
            Init::Atom(s) => vec![s.clone()],
            Init::List(v) => v.iter().flat_map(|i| i.flatten()).collect(),
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        let mut s = self.atom()?.trim();
        // `(1000)`, as FRAMERATE_CONST leaves it.
        while let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            s = inner.trim();
        }
        match s {
            "false" => return Some(0),
            "true" => return Some(1),
            _ => {}
        }
        let (neg, s) = match s.strip_prefix('-') {
            Some(r) => (true, r.trim()),
            None => (false, s),
        };
        let v = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            Some(h) => i64::from_str_radix(h, 16).ok(),
            None => s.parse().ok(),
        }?;
        Some(if neg { -v } else { v })
    }
    /// Evaluates a constant arithmetic expression such as `70.0f * (11.0f / 17.0f)`.
    pub fn as_f32(&self) -> Option<f32> {
        eval_expr(self.atom()?)
    }
}

/// The macros and enum members of some decomp sources, to evaluate the integer constant
/// expressions tables are written in (`CAM_INTERFACE_FIELD(CAM_LETTERBOX_NONE,
/// CAM_HUD_VISIBILITY_ALL, NORMAL1_FLAG_1 | NORMAL1_FLAG_0)`, `ACTOR_FLAG_ATTENTION_ENABLED |
/// ACTOR_FLAG_FRIENDLY`): object-like and function-like `#define`s (continued lines joined) and
/// the members of every enum, expanded and then evaluated as C does (casts dropped).
#[derive(Default, Clone)]
pub struct Macros {
    objects: HashMap<String, String>,
    functions: HashMap<String, (Vec<String>, String)>,
    values: HashMap<String, i64>,
}

impl Macros {
    /// From sources already [`prepare`]d (comments stripped, the build's branches).
    pub fn new(sources: &[&str]) -> Macros {
        let mut m = Macros::default();
        // `stdbool.h`'s, as the decomp's headers use them.
        m.values.insert("false".into(), 0);
        m.values.insert("true".into(), 1);
        for src in sources {
            m.add(src);
        }
        m
    }

    /// Reads `rel` files of the decomp ([`read_c`]) into a table.
    pub fn read(decomp: &Path, rels: &[&str]) -> Result<Macros> {
        let texts = rels.iter().map(|r| read_c(decomp, r)).collect::<Result<Vec<_>>>()?;
        Ok(Macros::new(&texts.iter().map(String::as_str).collect::<Vec<_>>()))
    }

    pub fn add(&mut self, src: &str) {
        let joined = src.replace("\\\r\n", " ").replace("\\\n", " ");
        for line in joined.lines() {
            let Some(rest) = line.trim_start().strip_prefix('#').map(str::trim_start).and_then(|r| r.strip_prefix("define")) else { continue };
            if !rest.starts_with([' ', '\t']) {
                continue;
            }
            let rest = rest.trim_start();
            let name_end = rest.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(rest.len());
            let name = &rest[..name_end];
            if name.is_empty() {
                continue;
            }
            let after = &rest[name_end..];
            if let Some(params) = after.strip_prefix('(') {
                let Some(close) = params.find(')') else { continue };
                let ps = params[..close].split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
                self.functions.entry(name.to_string()).or_insert((ps, params[close + 1..].trim().to_string()));
            } else {
                self.objects.entry(name.to_string()).or_insert(after.trim().to_string());
            }
        }
        // Enum members, with their values.
        let clean = strip_comments(src);
        let mut from = 0;
        while let Some(rel) = clean[from..].find("enum") {
            let at = from + rel;
            from = at + 4;
            let before_ok = at == 0 || !clean.as_bytes()[at - 1].is_ascii_alphanumeric() && clean.as_bytes()[at - 1] != b'_';
            let rest = &clean[from..];
            let Some(open) = rest.find('{') else { break };
            if !before_ok || rest[..open].contains([';', '(', ')', '=']) {
                continue;
            }
            let Some(close) = rest[open..].find('}') else { break };
            let body = &rest[open + 1..open + close];
            let mut next = 0i64;
            for item in body.split(',') {
                let item = item.trim();
                if item.is_empty() || item.starts_with('#') {
                    continue;
                }
                let (name, val) = match item.split_once('=') {
                    Some((n, e)) => (n.trim(), self.eval(e).unwrap_or(next)),
                    None => (item, next),
                };
                if name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
                    self.values.entry(name.to_string()).or_insert(val);
                }
                next = val + 1;
            }
            from += open + close;
        }
    }

    /// An enum built by `#include`ing a table of `DEFINE_*` rows (`NA_BGM` from
    /// `tables/sequence_table.h`): each row's argument `arg` is its member, valued by its row.
    pub fn add_table(&mut self, table: &str, arg: usize) {
        for (i, (_, args)) in define_rows_nested(table).into_iter().enumerate() {
            if let Some(name) = args.get(arg).filter(|n| n.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')) {
                self.values.entry(name.clone()).or_insert(i as i64);
            }
        }
    }

    /// Names `name` `value` (one the sources don't define, such as a sound's id).
    pub fn set(&mut self, name: &str, value: i64) {
        self.values.insert(name.to_string(), value);
    }

    /// `name`'s value: an enum member, or an object-like macro that evaluates.
    pub fn get(&self, name: &str) -> Option<i64> {
        self.values.get(name).copied().or_else(|| self.objects.get(name).and_then(|b| self.eval(b)))
    }

    /// Expands the macros in `expr` (to a depth of 32).
    pub fn expand(&self, expr: &str) -> String {
        let mut s = expr.to_string();
        for _ in 0..32 {
            let e = self.expand_once(&s);
            if e == s {
                break;
            }
            s = e;
        }
        s
    }

    fn expand_once(&self, s: &str) -> String {
        let b = s.as_bytes();
        let mut out = String::with_capacity(s.len());
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            if c.is_ascii_alphabetic() || c == b'_' {
                let st = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let name = &s[st..i];
                if let Some((params, body)) = self.functions.get(name) {
                    let mut j = i;
                    while j < b.len() && b[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < b.len() && b[j] == b'(' {
                        let (args, end) = split_args(&s[j + 1..]);
                        let mut body = body.clone();
                        for (p, a) in params.iter().zip(&args) {
                            body = replace_ident(&body, p, &format!("({a})"));
                        }
                        out.push('(');
                        out.push_str(&body);
                        out.push(')');
                        i = j + 1 + end + 1;
                        continue;
                    }
                }
                if let Some(body) = self.objects.get(name).filter(|b| !b.is_empty()) {
                    out.push('(');
                    out.push_str(body);
                    out.push(')');
                } else {
                    out.push_str(name);
                }
                continue;
            }
            if c.is_ascii_digit() {
                // Keep a number whole (0x10, 1e3f): its letters aren't names.
                let st = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'.') {
                    i += 1;
                }
                out.push_str(&s[st..i]);
                continue;
            }
            out.push(c as char);
            i += 1;
        }
        out
    }

    /// An integer constant expression, after expansion; None if a name is left over.
    pub fn eval(&self, expr: &str) -> Option<i64> {
        let e = self.expand(expr);
        let v = int_expr(&e, &self.values);
        v
    }
}

/// Arguments of a call whose `(` was just passed: (args, index of the closing `)`).
fn split_args(s: &str) -> (Vec<String>, usize) {
    let mut args = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for (i, c) in s.char_indices() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' if depth == 0 => {
                if !cur.trim().is_empty() || !args.is_empty() {
                    args.push(cur.trim().to_string());
                }
                return (args, i);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => args.push(std::mem::take(&mut cur).trim().to_string()),
            _ => cur.push(c),
        }
    }
    (args, s.len())
}

/// The top-level, comma-separated arguments of `NAME(a, F(b, c), d)`, or None.
pub fn call_args(s: &str) -> Option<(&str, Vec<String>)> {
    let s = s.trim();
    let open = s.find('(')?;
    let name = s[..open].trim();
    let (args, end) = split_args(&s[open + 1..]);
    (open + 1 + end == s.len() - 1).then_some((name, args))
}

fn replace_ident(s: &str, name: &str, with: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if (b[i].is_ascii_alphabetic() || b[i] == b'_') && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')) {
            let st = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push_str(if &s[st..i] == name { with } else { &s[st..i] });
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

/// C's integer arithmetic on an expanded expression: numbers (with u/l suffixes), the names in
/// `values`, casts to the decomp's integer types (dropped), `~ ! - +` unary, `* / % + - << >>
/// < <= > >= == != & ^ | && ||`, `?:`.
fn int_expr(e: &str, values: &HashMap<String, i64>) -> Option<i64> {
    #[derive(Debug, Clone, PartialEq)]
    enum T {
        N(i64),
        Op(&'static str),
    }
    const OPS: [&str; 24] = ["<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "(", ")", "~", "!", "*", "/", "%", "+", "-", "<", ">", "&", "^", "|", "?", ":"];
    const TYPES: [&str; 14] = ["s8", "u8", "s16", "u16", "s32", "u32", "s64", "u64", "int", "unsigned", "signed", "char", "short", "long"];
    let b = e.as_bytes();
    let mut t = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() {
            let st = i;
            while i < b.len() && b[i].is_ascii_alphanumeric() {
                i += 1;
            }
            t.push(T::N(parse_int(&e[st..i])?));
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let st = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let name = &e[st..i];
            if TYPES.contains(&name) {
                // `(u8)x`: drop the cast's parentheses and type.
                if t.last() == Some(&T::Op("(")) {
                    let mut j = i;
                    while j < b.len() && (b[j].is_ascii_whitespace() || b[j].is_ascii_alphanumeric()) {
                        j += 1;
                    }
                    if b.get(j) == Some(&b')') {
                        t.pop();
                        i = j + 1;
                        continue;
                    }
                }
                continue;
            }
            t.push(T::N(*values.get(name)?));
        } else if let Some(op) = OPS.iter().find(|o| e[i..].starts_with(**o)) {
            t.push(T::Op(op));
            i += op.len();
        } else {
            return None;
        }
    }
    fn prec(op: &str) -> Option<u8> {
        Some(match op {
            "?" => 1,
            "||" => 2,
            "&&" => 3,
            "|" => 4,
            "^" => 5,
            "&" => 6,
            "==" | "!=" => 7,
            "<" | ">" | "<=" | ">=" => 8,
            "<<" | ">>" => 9,
            "+" | "-" => 10,
            "*" | "/" | "%" => 11,
            _ => return None,
        })
    }
    fn unary(t: &[T], i: &mut usize) -> Option<i64> {
        match t.get(*i)? {
            T::Op("-") => {
                *i += 1;
                Some(-unary(t, i)?)
            }
            T::Op("+") => {
                *i += 1;
                unary(t, i)
            }
            T::Op("~") => {
                *i += 1;
                Some(!unary(t, i)?)
            }
            T::Op("!") => {
                *i += 1;
                Some((unary(t, i)? == 0) as i64)
            }
            T::Op("(") => {
                *i += 1;
                let v = binary(t, i, 0)?;
                (t.get(*i) == Some(&T::Op(")"))).then(|| *i += 1)?;
                Some(v)
            }
            T::N(n) => {
                *i += 1;
                Some(*n)
            }
            _ => None,
        }
    }
    fn binary(t: &[T], i: &mut usize, min: u8) -> Option<i64> {
        let mut l = unary(t, i)?;
        while let Some(T::Op(op)) = t.get(*i) {
            let Some(p) = prec(op) else { break };
            if p < min {
                break;
            }
            *i += 1;
            if *op == "?" {
                let a = binary(t, i, 0)?;
                (t.get(*i) == Some(&T::Op(":"))).then(|| *i += 1)?;
                let b = binary(t, i, p)?;
                l = if l != 0 { a } else { b };
                continue;
            }
            let r = binary(t, i, p + 1)?;
            l = match *op {
                "||" => (l != 0 || r != 0) as i64,
                "&&" => (l != 0 && r != 0) as i64,
                "|" => l | r,
                "^" => l ^ r,
                "&" => l & r,
                "==" => (l == r) as i64,
                "!=" => (l != r) as i64,
                "<" => (l < r) as i64,
                ">" => (l > r) as i64,
                "<=" => (l <= r) as i64,
                ">=" => (l >= r) as i64,
                "<<" => l.checked_shl(r as u32)?,
                ">>" => l.checked_shr(r as u32)?,
                "+" => l + r,
                "-" => l - r,
                "*" => l * r,
                "/" => l.checked_div(r)?,
                _ => l.checked_rem(r)?,
            };
        }
        Some(l)
    }
    let mut i = 0;
    let v = binary(&t, &mut i, 0)?;
    (i == t.len()).then_some(v)
}

/// Evaluates `+ - * /`, parentheses, unary minus and C numeric literals (`1.5f`, `0x10`).
/// Arithmetic is done in f32 like the compiler folds float constants.
pub fn eval_expr(src: &str) -> Option<f32> {
    struct P<'a> {
        b: &'a [u8],
        i: usize,
    }
    impl P<'_> {
        fn ws(&mut self) {
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
        }
        fn expr(&mut self) -> Option<f32> {
            let mut v = self.term()?;
            loop {
                self.ws();
                match self.b.get(self.i) {
                    Some(b'+') => {
                        self.i += 1;
                        v += self.term()?;
                    }
                    Some(b'-') => {
                        self.i += 1;
                        v -= self.term()?;
                    }
                    _ => return Some(v),
                }
            }
        }
        fn term(&mut self) -> Option<f32> {
            let mut v = self.factor()?;
            loop {
                self.ws();
                match self.b.get(self.i) {
                    Some(b'*') => {
                        self.i += 1;
                        v *= self.factor()?;
                    }
                    Some(b'/') => {
                        self.i += 1;
                        v /= self.factor()?;
                    }
                    _ => return Some(v),
                }
            }
        }
        fn factor(&mut self) -> Option<f32> {
            self.ws();
            match self.b.get(self.i)? {
                b'-' => {
                    self.i += 1;
                    Some(-self.factor()?)
                }
                b'(' => {
                    self.i += 1;
                    let v = self.expr()?;
                    self.ws();
                    (self.b.get(self.i) == Some(&b')')).then(|| self.i += 1)?;
                    Some(v)
                }
                _ => {
                    let start = self.i;
                    while self.i < self.b.len() && (self.b[self.i].is_ascii_alphanumeric() || self.b[self.i] == b'.') {
                        self.i += 1;
                    }
                    let t = std::str::from_utf8(&self.b[start..self.i]).ok()?;
                    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                        return i64::from_str_radix(h, 16).ok().map(|v| v as f32);
                    }
                    t.trim_end_matches(['f', 'F']).parse::<f32>().ok()
                }
            }
        }
    }
    let mut p = P { b: src.trim().as_bytes(), i: 0 };
    let v = p.expr()?;
    p.ws();
    (p.i == p.b.len()).then_some(v)
}

/// Removes `//` and `/* */` comments, keeping line structure.
pub fn strip_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else if b[i] == b'"' {
            // Keep string literals intact (they may contain "//").
            let start = i;
            i += 1;
            while i < b.len() && b[i] != b'"' {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            i += 1;
            out.push_str(&src[start..i.min(b.len())]);
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Finds the initializer of the first definition of `name` in comment-stripped source.
pub fn find_initializer(src: &str, name: &str) -> Result<Init> {
    let b = src.as_bytes();
    let mut from = 0;
    while let Some(rel) = src[from..].find(name) {
        let at = from + rel;
        from = at + name.len();
        if (at > 0 && is_ident(b[at - 1])) || b.get(from).is_some_and(|&c| is_ident(c)) {
            continue;
        }
        // Skip array dimensions, then require `=` and `{`.
        let mut i = from;
        loop {
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if b.get(i) == Some(&b'[') {
                while i < b.len() && b[i] != b']' {
                    i += 1;
                }
                i += 1;
            } else {
                break;
            }
        }
        if b.get(i) != Some(&b'=') {
            continue;
        }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if b.get(i) != Some(&b'{') {
            continue;
        }
        let (init, _) = parse_list(src, i)?;
        return Ok(init);
    }
    bail!("no initializer for {name}")
}

/// Parses `{ ... }` starting at `start` (which must be `{`); returns the list and the index after `}`.
fn parse_list(src: &str, start: usize) -> Result<(Init, usize)> {
    let b = src.as_bytes();
    let mut items = Vec::new();
    let mut i = start + 1;
    let mut atom = String::new();
    let flush = |atom: &mut String, items: &mut Vec<Init>| {
        let t = atom.trim();
        if !t.is_empty() {
            items.push(Init::Atom(t.to_string()));
        }
        atom.clear();
    };
    // Commas inside parentheses (macro arguments such as `CLOCK_TIME(4, 0)`) don't split atoms.
    let mut depth = 0usize;
    while i < b.len() {
        match b[i] {
            b'(' => {
                depth += 1;
                atom.push('(');
            }
            b')' => {
                depth = depth.saturating_sub(1);
                atom.push(')');
            }
            b',' if depth > 0 => atom.push(','),
            b'{' => {
                flush(&mut atom, &mut items);
                let (sub, next) = parse_list(src, i)?;
                items.push(sub);
                i = next;
                continue;
            }
            b'}' => {
                flush(&mut atom, &mut items);
                return Ok((Init::List(items), i + 1));
            }
            b',' => flush(&mut atom, &mut items),
            c => atom.push(c as char),
        }
        i += 1;
    }
    bail!("unterminated initializer")
}

/// Reads `/* 0xNN */ PREFIX_NAME,` style enum members with the given prefix, in order.
pub fn enum_members(src: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines().filter(|l| l.trim_start().starts_with("/* 0x")) {
        let clean = strip_comments(line);
        let t = clean.trim().trim_end_matches(',');
        if t.starts_with(prefix) && t.bytes().all(is_ident) && !out.iter().any(|o: &String| o == t) {
            out.push(t.to_string());
        }
    }
    out
}


// Table helpers (moved from oot_extract in spike 04 so the runtime scene loader can use them).

/// `DEFINE_X(a, b, ...)` rows in table order.
pub fn define_rows(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut l = line.trim();
        if l.starts_with("/*") {
            match l.find("*/") {
                Some(e) => l = l[e + 2..].trim(),
                None => continue,
            }
        }
        if !l.starts_with("DEFINE_") {
            continue;
        }
        let (Some(a), Some(b)) = (l.find('('), l.rfind(')')) else { continue };
        if b < a {
            continue;
        }
        let args = l[a + 1..b].split(',').map(|s| s.trim().trim_matches('"').to_string()).collect();
        out.push((l[..a].trim().to_string(), args));
    }
    out
}

/// `DEFINE_X(a, F(b, c), ...)` rows in table order, splitting arguments only at top-level
/// commas (so macro arguments such as `TRANS_TYPE_CIRCLE(TCA_NORMAL, TCC_BLACK, TCS_FAST)`
/// stay whole).
pub fn define_rows_nested(text: &str) -> Vec<(String, Vec<String>)> {
    let clean = strip_comments(text);
    let mut out = Vec::new();
    for line in clean.lines() {
        let l = line.trim();
        if !l.starts_with("DEFINE_") {
            continue;
        }
        let (Some(a), Some(b)) = (l.find('('), l.rfind(')')) else { continue };
        if b < a {
            continue;
        }
        let mut args = Vec::new();
        let mut depth = 0;
        let mut cur = String::new();
        for c in l[a + 1..b].chars() {
            match c {
                '(' => {
                    depth += 1;
                    cur.push(c);
                }
                ')' => {
                    depth -= 1;
                    cur.push(c);
                }
                ',' if depth == 0 => args.push(std::mem::take(&mut cur).trim().trim_matches('"').to_string()),
                _ => cur.push(c),
            }
        }
        args.push(cur.trim().trim_matches('"').to_string());
        out.push((l[..a].trim().to_string(), args));
    }
    out
}

pub fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim().trim_end_matches(['u', 'U', 'l', 'L']);
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(h, 16).ok()
    } else {
        s.parse().ok()
    }
}

/// Values of the `typedef enum` that contains `member`.
pub fn parse_enum(text: &str, member: &str) -> HashMap<i64, String> {
    let clean = strip_comments(text);
    let mut out = HashMap::new();
    let mut search = 0;
    while let Some(rel) = clean[search..].find(member) {
        let pos = search + rel;
        search = pos + member.len();
        let before_ok = pos == 0 || !clean.as_bytes()[pos - 1].is_ascii_alphanumeric() && clean.as_bytes()[pos - 1] != b'_';
        let after = clean.as_bytes().get(pos + member.len()).copied().unwrap_or(b' ');
        if !before_ok || after.is_ascii_alphanumeric() || after == b'_' {
            continue;
        }
        let Some(open) = clean[..pos].rfind('{') else { continue };
        if !clean[open.saturating_sub(40)..open].contains("enum") {
            continue;
        }
        let Some(close_rel) = clean[pos..].find('}') else { continue };
        let body = &clean[open + 1..pos + close_rel];
        let mut next = 0i64;
        for item in body.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let (name, val) = match item.split_once('=') {
                Some((n, e)) => (n.trim(), parse_int(e).unwrap_or(next)),
                None => (item, next),
            };
            out.insert(val, name.to_string());
            next = val + 1;
        }
        break;
    }
    out
}

pub fn parse_defines(text: &str, prefix: &str) -> HashMap<i64, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("#define ") else { continue };
        let mut it = rest.split_whitespace();
        let (Some(name), Some(val)) = (it.next(), it.next()) else { continue };
        if name.starts_with(prefix)
            && let Some(v) = parse_int(val)
        {
            out.entry(v).or_insert_with(|| name.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_tables() {
        let src = strip_comments(
            "#ifndef X\nvoid* sA[] = { a, b /* c */, // d\n e };\n#else\nvoid* sA[][2] = { {x}, {y} };\n#endif\n\
             u8 sT[2][3] = { { 1, 0x2, 3 }, { 4, 5, 6 } };",
        );
        let a = find_initializer(&src, "sA").unwrap();
        assert_eq!(a.flatten(), vec!["a", "b", "e"]);
        let t = find_initializer(&src, "sT").unwrap();
        assert_eq!(t.list().len(), 2);
        assert_eq!(t.list()[0].list()[1].as_int(), Some(2));
        let m = find_initializer("x sM[] = { { F(1, 2) + 1, 3 } };", "sM").unwrap();
        assert_eq!(m.list()[0].flatten(), vec!["F(1, 2) + 1", "3"]);
    }

    #[test]
    fn evaluates_float_expressions() {
        assert_eq!(eval_expr("11.0f / 17.0f"), Some(11.0 / 17.0));
        assert_eq!(eval_expr("70.0f * (11.0f / 17.0f)"), Some(70.0f32 * (11.0 / 17.0)));
        assert_eq!(eval_expr("-0x10"), Some(-16.0));
        assert_eq!(Init::Atom("-1592".into()).as_int(), Some(-1592));
        assert_eq!(eval_expr("1 +"), None);
    }

    #[test]
    fn splits_define_rows_at_top_level_commas() {
        let rows = define_rows_nested("/* 0x0 */ DEFINE_E(ENTR_A, SCENE_B, 0, false, F(X, Y, Z), T) // c
");
        assert_eq!(rows, vec![("DEFINE_E".to_string(), vec!["ENTR_A", "SCENE_B", "0", "false", "F(X, Y, Z)", "T"].into_iter().map(String::from).collect())]);
    }

    #[test]
    fn evaluates_macro_expressions() {
        let m = Macros::new(&["#define F_0 (1 << 0)\n#define F_1 (1 << 1)\n#define MASK 0xF0\n#define FIELD(a, b, c) \\\n    (((a) & MASK) | (b) | ((c) & 0xFF))\ntypedef enum { L_NONE = 0x00, L_MED = 0x20, L_BIG } L;\n"]);
        assert_eq!(m.eval("FIELD(L_MED, 0x300, F_1 | F_0)"), Some(0x323));
        assert_eq!(m.eval("(u8)-1 + L_BIG"), Some(0x20));
        assert_eq!(m.eval("1 ? 2 : 3"), Some(2));
        assert_eq!(m.eval("UNKNOWN | 1"), None);
        assert_eq!(call_args("CAM_X(-20, 200, F(1, 2) | 3)"), Some(("CAM_X", vec!["-20".to_string(), "200".to_string(), "F(1, 2) | 3".to_string()])));
    }

    #[test]
    fn takes_the_gc_eu_mq_dbg_branches() {
        let src = "a\n#if OOT_VERSION < PAL_1_0\nntsc\n#elif PLATFORM_GC && DEBUG_FEATURES\ngc_dbg\n#else\nother\n#endif\n#if !OOT_MQ\nvanilla\n#endif\n#ifdef AVOID_UB\nub\n#else\nshipped\n#endif\nz";
        let out = preprocess(src);
        assert_eq!(out.lines().filter(|l| !l.is_empty()).collect::<Vec<_>>(), vec!["a", "gc_dbg", "shipped", "z"]);
        assert_eq!(out.lines().count(), src.lines().count());
        assert_eq!(prepare("x = FRAMERATE_CONST(1000, 1200), FRAMERATE_CONST(F(1, 2), 3);"), "x = (1000), (F(1, 2));");
    }

    #[test]
    fn reads_enum_members_in_order() {
        let src = "typedef enum {\n /* 0x00 */ P_A,\n /* 0x01 */ P_B, // note\n /* 0x02 */ P_MAX\n} P;\nx = P_A;";
        assert_eq!(enum_members(src, "P_"), vec!["P_A", "P_B", "P_MAX"]);
    }
}
