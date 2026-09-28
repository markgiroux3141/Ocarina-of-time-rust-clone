//! Runs `Scene_DrawConfig*` functions from the decomp source well enough to learn what
//! they bind to segments 8..0xD and which colours they set before the rooms are drawn.
//! Unknown runtime state evaluates to 0 and is reported.
//!
//! Moved here from `oot_extract` (spike 04) so the game can re-run a scene's draw config
//! every frame with the live `gameplayFrames`, which drives the scrolling textures.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
enum K {
    Id,
    Num(f64, bool),
    Str,
    P,
}

#[derive(Clone, Debug)]
struct Tok {
    k: K,
    s: String,
}

fn lex(src: &str) -> Vec<Tok> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line_start = true;
    const P3: [&str; 2] = ["<<=", ">>="];
    const P2: [&str; 19] = ["->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^="];
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            line_start = true;
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'#' && line_start {
            while i < b.len() && b[i] != b'\n' {
                if b[i] == b'\\' && b.get(i + 1) == Some(&b'\n') {
                    i += 1;
                } else if b[i] == b'\\' && b.get(i + 1) == Some(&b'\r') {
                    i += 2;
                }
                i += 1;
            }
            continue;
        }
        line_start = false;
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if c == b'"' || c == b'\'' {
            let q = c;
            let st = i;
            i += 1;
            while i < b.len() && b[i] != q {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            let s = String::from_utf8_lossy(&b[st..i.min(b.len())]).to_string();
            out.push(if q == b'"' { Tok { k: K::Str, s } } else { Tok { k: K::Num(0.0, false), s } });
            continue;
        }
        if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let st = i;
            let (v, f);
            if c == b'0' && matches!(b.get(i + 1), Some(b'x') | Some(b'X')) {
                i += 2;
                let hs = i;
                while i < b.len() && b[i].is_ascii_hexdigit() {
                    i += 1;
                }
                v = i64::from_str_radix(std::str::from_utf8(&b[hs..i]).unwrap_or("0"), 16).unwrap_or(0) as f64;
                f = false;
            } else {
                let mut is_f = false;
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || ((b[i] == b'e' || b[i] == b'E') && i > st)) {
                    if b[i] == b'.' || b[i] == b'e' || b[i] == b'E' {
                        is_f = true;
                        if (b[i] == b'e' || b[i] == b'E') && matches!(b.get(i + 1), Some(b'+') | Some(b'-')) {
                            i += 1;
                        }
                    }
                    i += 1;
                }
                let txt = std::str::from_utf8(&b[st..i]).unwrap_or("0");
                v = txt.trim_end_matches('.').parse::<f64>().unwrap_or(0.0);
                f = is_f;
            }
            let mut fl = f;
            while i < b.len() && matches!(b[i], b'f' | b'F' | b'u' | b'U' | b'l' | b'L') {
                if b[i] == b'f' || b[i] == b'F' {
                    fl = true;
                }
                i += 1;
            }
            out.push(Tok { k: K::Num(v, fl), s: String::from_utf8_lossy(&b[st..i]).to_string() });
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let st = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push(Tok { k: K::Id, s: String::from_utf8_lossy(&b[st..i]).to_string() });
            continue;
        }
        let rest = &src[i..];
        let p = P3.iter().chain(P2.iter()).find(|p| rest.starts_with(**p)).map(|p| p.to_string()).unwrap_or_else(|| (c as char).to_string());
        i += p.len();
        out.push(Tok { k: K::P, s: p });
    }
    out
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num(f64, bool),
    Name(String),
    Str,
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>, String),
    Un(String, Box<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    Cast(String, Box<Expr>),
    Tern(Box<Expr>, Box<Expr>, Box<Expr>),
    PostInc(Box<Expr>, i64),
    PreInc(Box<Expr>, i64),
    Assign(String, Box<Expr>, Box<Expr>),
    Comma(Vec<Expr>),
}

#[derive(Clone, Debug)]
enum Stmt {
    Expr(Expr),
    Decl(String, Vec<(String, Option<Expr>)>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    Block(Vec<Stmt>),
    Skip,
}

const TYPES: [&str; 26] = [
    "u8", "s8", "u16", "s16", "u32", "s32", "u64", "s64", "f32", "f64", "Gfx", "void", "Vec3f", "Vec3s", "Mtx", "Player", "Camera", "int", "char",
    "unsigned", "signed", "float", "double", "const", "static", "volatile",
];

fn is_type(s: &str) -> bool {
    TYPES.contains(&s)
}

struct Parser<'a> {
    t: &'a [Tok],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &str {
        self.t.get(self.i).map(|t| t.s.as_str()).unwrap_or("")
    }
    fn peek_at(&self, k: usize) -> &str {
        self.t.get(self.i + k).map(|t| t.s.as_str()).unwrap_or("")
    }
    fn eof(&self) -> bool {
        self.i >= self.t.len()
    }
    fn next(&mut self) -> Option<&'a Tok> {
        let t = self.t.get(self.i);
        self.i += 1;
        t
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.peek() == s {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn skip_balanced(&mut self, open: &str, close: &str) {
        if !self.eat(open) {
            return;
        }
        let mut depth = 1;
        while !self.eof() && depth > 0 {
            let s = self.peek().to_string();
            if s == open {
                depth += 1;
            } else if s == close {
                depth -= 1;
            }
            self.i += 1;
        }
    }
    fn text(&self, a: usize, b: usize) -> String {
        let mut s = String::new();
        let mut prev_word = false;
        for t in &self.t[a..b.min(self.t.len())] {
            let word = !matches!(t.k, K::P);
            if word && prev_word {
                s.push(' ');
            }
            s.push_str(&t.s);
            if t.s == "," {
                s.push(' ');
            }
            prev_word = word;
        }
        s
    }

    fn stmts_until_eof(&mut self) -> Vec<Stmt> {
        let mut v = Vec::new();
        while !self.eof() {
            let before = self.i;
            v.push(self.stmt());
            if self.i == before {
                self.i += 1;
            }
        }
        v
    }

    fn stmt(&mut self) -> Stmt {
        let s = self.peek().to_string();
        match s.as_str() {
            "{" => {
                self.i += 1;
                let mut v = Vec::new();
                while !self.eof() && self.peek() != "}" {
                    let before = self.i;
                    v.push(self.stmt());
                    if self.i == before {
                        self.i += 1;
                    }
                }
                self.eat("}");
                Stmt::Block(v)
            }
            ";" => {
                self.i += 1;
                Stmt::Skip
            }
            "if" => {
                self.i += 1;
                self.eat("(");
                let c = self.expr();
                self.eat(")");
                let a = self.stmt();
                let b = if self.eat("else") { Some(Box::new(self.stmt())) } else { None };
                Stmt::If(c, Box::new(a), b)
            }
            "switch" | "while" | "for" => {
                self.i += 1;
                self.skip_balanced("(", ")");
                let _ = self.stmt();
                Stmt::Skip
            }
            "do" => {
                self.i += 1;
                let _ = self.stmt();
                self.eat("while");
                self.skip_balanced("(", ")");
                self.eat(";");
                Stmt::Skip
            }
            "case" | "default" => {
                while !self.eof() && self.peek() != ":" {
                    self.i += 1;
                }
                self.eat(":");
                Stmt::Skip
            }
            "return" | "break" | "continue" | "goto" => {
                while !self.eof() && self.peek() != ";" {
                    self.i += 1;
                }
                self.eat(";");
                Stmt::Skip
            }
            _ if is_type(&s) && !(self.peek_at(1) == "(" && s != "static") => self.decl(),
            _ => {
                let e = self.expr();
                self.eat(";");
                Stmt::Expr(e)
            }
        }
    }

    fn decl(&mut self) -> Stmt {
        let mut ty = String::new();
        while is_type(self.peek()) || self.peek() == "*" {
            let s = self.peek().to_string();
            if !matches!(s.as_str(), "static" | "const" | "volatile" | "*" | "unsigned" | "signed") {
                ty = s;
            }
            self.i += 1;
        }
        let mut vars = Vec::new();
        loop {
            while self.eat("*") {}
            let Some(name) = self.next().map(|t| t.s.clone()) else { break };
            while self.peek() == "[" {
                self.skip_balanced("[", "]");
            }
            let init = if self.eat("=") {
                if self.peek() == "{" {
                    self.skip_balanced("{", "}");
                    None
                } else {
                    Some(self.assign())
                }
            } else {
                None
            };
            vars.push((name, init));
            if !self.eat(",") {
                break;
            }
        }
        self.eat(";");
        Stmt::Decl(ty, vars)
    }

    fn expr(&mut self) -> Expr {
        let first = self.assign();
        if self.peek() != "," {
            return first;
        }
        let mut v = vec![first];
        while self.eat(",") {
            v.push(self.assign());
        }
        Expr::Comma(v)
    }

    fn assign(&mut self) -> Expr {
        let lhs = self.tern();
        let op = self.peek().to_string();
        if matches!(op.as_str(), "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=") {
            self.i += 1;
            let rhs = self.assign();
            return Expr::Assign(op, Box::new(lhs), Box::new(rhs));
        }
        lhs
    }

    fn tern(&mut self) -> Expr {
        let c = self.binary(1);
        if self.eat("?") {
            let a = self.expr();
            self.eat(":");
            let b = self.tern();
            return Expr::Tern(Box::new(c), Box::new(a), Box::new(b));
        }
        c
    }

    fn prec(op: &str) -> u8 {
        match op {
            "||" => 1,
            "&&" => 2,
            "|" => 3,
            "^" => 4,
            "&" => 5,
            "==" | "!=" => 6,
            "<" | ">" | "<=" | ">=" => 7,
            "<<" | ">>" => 8,
            "+" | "-" => 9,
            "*" | "/" | "%" => 10,
            _ => 0,
        }
    }

    fn binary(&mut self, min: u8) -> Expr {
        let mut lhs = self.unary();
        loop {
            let op = self.peek().to_string();
            let p = Self::prec(&op);
            if p == 0 || p < min || self.t.get(self.i).is_some_and(|t| t.k != K::P) {
                break;
            }
            self.i += 1;
            let rhs = self.binary(p + 1);
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        lhs
    }

    fn is_cast(&self) -> bool {
        if self.peek() != "(" || !is_type(self.peek_at(1)) {
            return false;
        }
        let mut k = 1;
        while is_type(self.peek_at(k)) || self.peek_at(k) == "*" {
            k += 1;
        }
        self.peek_at(k) == ")"
    }

    fn unary(&mut self) -> Expr {
        let s = self.peek().to_string();
        match s.as_str() {
            "-" | "!" | "~" | "+" | "&" | "*" if self.t.get(self.i).is_some_and(|t| t.k == K::P) => {
                self.i += 1;
                let e = self.unary();
                Expr::Un(s, Box::new(e))
            }
            "++" | "--" => {
                self.i += 1;
                let e = self.unary();
                Expr::PreInc(Box::new(e), if s == "++" { 1 } else { -1 })
            }
            "sizeof" => {
                self.i += 1;
                if self.peek() == "(" {
                    self.skip_balanced("(", ")");
                } else {
                    let _ = self.unary();
                }
                Expr::Num(8.0, false)
            }
            "(" if self.is_cast() => {
                self.i += 1;
                let mut ty = String::new();
                while self.peek() != ")" && !self.eof() {
                    let t = self.peek().to_string();
                    if t != "*" && t != "const" {
                        ty = t;
                    }
                    self.i += 1;
                }
                self.eat(")");
                let e = self.unary();
                Expr::Cast(ty, Box::new(e))
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> Expr {
        let start = self.i;
        let mut e = self.primary();
        loop {
            match self.peek() {
                "(" => {
                    self.i += 1;
                    let mut args = Vec::new();
                    while !self.eof() && self.peek() != ")" {
                        let before = self.i;
                        args.push(self.assign());
                        if !self.eat(",") && self.peek() != ")" && self.i == before {
                            self.i += 1;
                        }
                    }
                    self.eat(")");
                    let name = match &e {
                        Expr::Name(n) => n.clone(),
                        _ => "?".into(),
                    };
                    e = Expr::Call(name, args, self.text(start, self.i));
                }
                "[" => {
                    self.i += 1;
                    let idx = self.expr();
                    self.eat("]");
                    e = Expr::Index(Box::new(e), Box::new(idx));
                }
                "." | "->" => {
                    self.i += 1;
                    let f = self.next().map(|t| t.s.clone()).unwrap_or_default();
                    e = Expr::Member(Box::new(e), f);
                }
                "++" => {
                    self.i += 1;
                    e = Expr::PostInc(Box::new(e), 1);
                }
                "--" => {
                    self.i += 1;
                    e = Expr::PostInc(Box::new(e), -1);
                }
                _ => break,
            }
        }
        e
    }

    fn primary(&mut self) -> Expr {
        let Some(t) = self.next() else { return Expr::Num(0.0, false) };
        match &t.k {
            K::Num(v, f) => Expr::Num(*v, *f),
            K::Id => Expr::Name(t.s.clone()),
            K::Str => {
                while self.t.get(self.i).is_some_and(|t| t.k == K::Str) {
                    self.i += 1;
                }
                Expr::Str
            }
            K::P if t.s == "(" => {
                let e = self.expr();
                self.eat(")");
                e
            }
            K::P => Expr::Num(0.0, false),
        }
    }
}

/// A static array: identifier elements (`void* x[] = { a, b }`) or Gfx macros.
pub struct Array {
    pub names: Vec<String>,
    gfx: Vec<Expr>,
    is_gfx: bool,
}

pub struct Program {
    pub arrays: HashMap<String, Array>,
    funcs: HashMap<String, Vec<Stmt>>,
}

impl Program {
    pub fn parse(src: &str) -> Program {
        let toks = lex(src);
        let mut arrays = HashMap::new();
        let mut funcs = HashMap::new();
        let mut i = 0;
        let mut buf_start = 0;
        while i < toks.len() {
            let s = toks[i].s.as_str();
            if s == ";" {
                i += 1;
                buf_start = i;
                continue;
            }
            if s != "{" {
                i += 1;
                continue;
            }
            // Find the matching brace.
            let mut depth = 0;
            let mut j = i;
            while j < toks.len() {
                match toks[j].s.as_str() {
                    "{" => depth += 1,
                    "}" => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            let head = &toks[buf_start..i];
            let body = &toks[(i + 1).min(toks.len())..j.min(toks.len())];
            if head.last().is_some_and(|t| t.s == "=") {
                // Array initializer: name is the identifier before the first '['.
                let name = head.iter().position(|t| t.s == "[").and_then(|k| k.checked_sub(1)).map(|k| head[k].s.clone());
                let ty = head.iter().find(|t| t.k == K::Id && t.s != "static").map(|t| t.s.clone()).unwrap_or_default();
                let is_ptr = head.iter().any(|t| t.s == "*");
                if let Some(name) = name {
                    if ty == "Gfx" && !is_ptr {
                        let mut p = Parser { t: body, i: 0 };
                        let mut gfx = Vec::new();
                        while !p.eof() {
                            let before = p.i;
                            gfx.push(p.assign());
                            p.eat(",");
                            if p.i == before {
                                p.i += 1;
                            }
                        }
                        arrays.insert(name, Array { names: Vec::new(), gfx, is_gfx: true });
                    } else if ty == "void" && is_ptr {
                        let names = body.iter().filter(|t| t.k == K::Id).map(|t| t.s.clone()).collect();
                        arrays.insert(name, Array { names, gfx: Vec::new(), is_gfx: false });
                    }
                }
                i = j + 1;
                if toks.get(i).is_some_and(|t| t.s == ";") {
                    i += 1;
                }
            } else if head.last().is_some_and(|t| t.s == ")") {
                if let Some(k) = head.iter().position(|t| t.s == "(")
                    && k > 0
                {
                    let name = head[k - 1].s.clone();
                    let mut p = Parser { t: body, i: 0 };
                    funcs.insert(name, p.stmts_until_eof());
                }
                i = j + 1;
            } else {
                i = j + 1;
            }
            buf_start = i;
        }
        Program { arrays, funcs }
    }

    pub fn run(&self, func: &str, scene_id: i64, scene_ids: &HashMap<String, i64>) -> Output {
        self.run_with(func, scene_id, scene_ids, &State::default())
    }

    /// Runs `func` with the given runtime state (frame counter, age, time of day).
    pub fn run_with(&self, func: &str, scene_id: i64, scene_ids: &HashMap<String, i64>, state: &State) -> Output {
        let mut m = Machine {
            prog: self,
            scene_id,
            scene_ids,
            state: state.clone(),
            vars: HashMap::new(),
            dls: Vec::new(),
            opa: RawBuf::default(),
            xlu: RawBuf::default(),
            assumed: BTreeSet::new(),
            errors: Vec::new(),
            depth: 0,
        };
        match self.funcs.get(func) {
            Some(body) => {
                for s in body {
                    m.exec(s);
                }
            }
            None => m.errors.push(format!("function {func} not found in z_scene_table.c")),
        }
        let opa = m.finish(&m.opa);
        let xlu = m.finish(&m.xlu);
        Output { opa, xlu, assumed: m.assumed.into_iter().collect(), errors: m.errors }
    }
}

#[derive(Clone, Debug)]
pub enum Cmd {
    Raw(u32, u32),
    CallSym(String),
}

#[derive(Clone, Debug)]
pub enum Bind {
    Sym(String),
    Dl(Vec<Cmd>),
    Mtx,
    Unknown(String),
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub bind: Bind,
    pub source: String,
    pub variants: Vec<String>,
}

#[derive(Default, Clone, Debug)]
pub struct Buf {
    pub segments: BTreeMap<u8, Binding>,
    pub pre: Vec<Cmd>,
}

fn cmd_json(c: &Cmd) -> Value {
    match c {
        Cmd::Raw(a, b) => json!(format!("{a:08X} {b:08X}")),
        Cmd::CallSym(s) => json!(format!("gSPDisplayList({s})")),
    }
}

impl Buf {
    pub fn to_json(&self) -> Value {
        let segs: BTreeMap<String, Value> = self
            .segments
            .iter()
            .map(|(s, b)| {
                let v = match &b.bind {
                    Bind::Sym(n) => json!({ "kind": "pointer", "symbol": n, "variants": b.variants, "c_expr": b.source }),
                    Bind::Dl(c) => json!({ "kind": "display_list", "commands": c.iter().map(cmd_json).collect::<Vec<_>>(), "c_expr": b.source }),
                    Bind::Mtx => json!({ "kind": "matrix", "note": "runtime matrix; bound to identity", "c_expr": b.source }),
                    Bind::Unknown(w) => json!({ "kind": "unresolved", "why": w, "c_expr": b.source }),
                };
                (format!("0x{s:02X}"), v)
            })
            .collect();
        json!({ "segments": segs, "pre_commands": self.pre.iter().map(cmd_json).collect::<Vec<_>>() })
    }
}

pub struct Output {
    pub opa: Buf,
    pub xlu: Buf,
    pub assumed: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
enum Val {
    Num(f64, bool),
    Name(String),
    Elem(String, String),
    Arr(String),
    Dl(usize, usize),
    Mtx,
}

#[derive(Clone, Debug)]
enum RawBind {
    Sym(String, Vec<String>),
    DlRef(usize, usize),
    Mtx,
    Unknown(String),
}

#[derive(Default)]
struct RawBuf {
    segments: BTreeMap<u8, (RawBind, String)>,
    pre: Vec<Cmd>,
}

enum Target {
    Opa,
    Xlu,
    Dl(usize, usize),
    None,
}

/// The runtime state a draw config can read. `Default` is a fresh child-day load at frame
/// 0 (noon), which is what the extractor documents.
#[derive(Clone, Debug)]
pub struct State {
    /// `play->gameplayFrames`: incremented once per game frame (`Play_Update`).
    pub gameplay_frames: u32,
    pub child: bool,
    /// `gSaveContext.nightFlag`.
    pub night: bool,
    /// `gSaveContext.sceneLayer` (0 child day, 1 child night, 2 adult day, 3 adult night, 4+ cutscenes).
    pub scene_layer: i64,
    /// `gSaveContext.dayTime` / `skyboxTime`.
    pub day_time: u16,
}

impl Default for State {
    fn default() -> Self {
        State { gameplay_frames: 0, child: true, night: false, scene_layer: 0, day_time: 0x8000 }
    }
}

struct Machine<'a> {
    prog: &'a Program,
    scene_id: i64,
    state: State,
    scene_ids: &'a HashMap<String, i64>,
    vars: HashMap<String, (String, Val)>,
    dls: Vec<Vec<Option<Cmd>>>,
    opa: RawBuf,
    xlu: RawBuf,
    assumed: BTreeSet<String>,
    errors: Vec<String>,
    depth: usize,
}

fn wrap(ty: &str, v: f64, f: bool) -> (f64, bool) {
    let i = if v.is_finite() { v.trunc() as i64 } else { 0 };
    match ty {
        "u8" => (i as u8 as f64, false),
        "s8" => (i as i8 as f64, false),
        "u16" => (i as u16 as f64, false),
        "s16" => (i as i16 as f64, false),
        "u32" | "unsigned" => (i as u32 as f64, false),
        "s32" | "int" | "signed" => (i as i32 as f64, false),
        "f32" | "f64" | "float" | "double" => (v, true),
        "void" => (0.0, false),
        _ => (v, f),
    }
}

fn rgba(r: f64, g: f64, b: f64, a: f64) -> u32 {
    let c = |v: f64| (v.trunc() as i64 as u32) & 0xFF;
    (c(r) << 24) | (c(g) << 16) | (c(b) << 8) | c(a)
}

fn tile_size(tile: f64, x: f64, y: f64, w: f64, h: f64) -> Cmd {
    let wrap = |v: f64| ((v.trunc() as i64) as u32) % (512 << 2);
    let (x, y) = (wrap(x), wrap(y));
    let (w, h) = (w.trunc() as i64, h.trunc() as i64);
    let lrs = (x as i64 + ((w - 1) << 2)) as u32 & 0xFFF;
    let lrt = (y as i64 + ((h - 1) << 2)) as u32 & 0xFFF;
    Cmd::Raw(0xF200_0000 | ((x & 0xFFF) << 12) | (y & 0xFFF), ((tile as u32 & 7) << 24) | (lrs << 12) | lrt)
}

impl Machine<'_> {
    fn finish(&self, b: &RawBuf) -> Buf {
        let mut out = Buf { segments: BTreeMap::new(), pre: b.pre.clone() };
        for (&s, (rb, src)) in &b.segments {
            let (bind, variants) = match rb {
                RawBind::Sym(n, v) => (Bind::Sym(n.clone()), v.clone()),
                RawBind::DlRef(id, pos) => {
                    let mut cmds = Vec::new();
                    for c in self.dls.get(*id).map(|d| &d[(*pos).min(d.len())..]).unwrap_or(&[]) {
                        match c {
                            Some(c) => {
                                let end = matches!(c, Cmd::Raw(w0, _) if w0 >> 24 == 0xDF);
                                cmds.push(c.clone());
                                if end {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    (Bind::Dl(cmds), Vec::new())
                }
                RawBind::Mtx => (Bind::Mtx, Vec::new()),
                RawBind::Unknown(w) => (Bind::Unknown(w.clone()), Vec::new()),
            };
            out.segments.insert(s, Binding { bind, source: src.clone(), variants });
        }
        out
    }

    fn exec(&mut self, s: &Stmt) {
        self.depth += 1;
        if self.depth > 200 {
            self.depth -= 1;
            return;
        }
        match s {
            Stmt::Expr(e) => {
                let _ = self.eval(e);
            }
            Stmt::Decl(ty, vars) => {
                for (n, init) in vars {
                    let v = match init {
                        Some(e) => self.eval(e),
                        None => Val::Num(0.0, false),
                    };
                    let v = match v {
                        Val::Num(x, f) => {
                            let (x, f) = wrap(ty, x, f);
                            Val::Num(x, f)
                        }
                        other => other,
                    };
                    self.vars.insert(n.clone(), (ty.clone(), v));
                }
            }
            Stmt::If(c, a, b) => {
                let cv = self.eval(c);
                if self.num(&cv) != 0.0 {
                    self.exec(a);
                } else if let Some(b) = b {
                    self.exec(b);
                }
            }
            Stmt::Block(v) => {
                for s in v {
                    self.exec(s);
                }
            }
            Stmt::Skip => {}
        }
        self.depth -= 1;
    }

    fn known_name(&self, n: &str) -> Option<f64> {
        if let Some(&id) = self.scene_ids.get(n) {
            return Some(id as f64);
        }
        let st = &self.state;
        let b = |v: bool| if v { 1.0 } else { 0.0 };
        Some(match n {
            "play.sceneId" | "play->sceneId" => self.scene_id as f64,
            "true" | "LINK_AGE_CHILD" => 1.0,
            "false" | "NULL" | "G_TX_RENDERTILE" | "LINK_AGE_ADULT" | "gSaveContext.cutsceneIndex" => 0.0,
            // z64save.h: LINK_AGE_ADULT = 0, LINK_AGE_CHILD = 1.
            "LINK_IS_CHILD" | "gSaveContext.linkAge" => b(st.child),
            "LINK_IS_ADULT" => b(!st.child),
            "IS_DAY" => b(!st.night),
            "IS_NIGHT" | "gSaveContext.nightFlag" => b(st.night),
            // macros.h: IS_CUTSCENE_LAYER is gSaveContext.sceneLayer > 3.
            "IS_CUTSCENE_LAYER" => b(st.scene_layer > 3),
            "gSaveContext.sceneLayer" => st.scene_layer as f64,
            "play.gameplayFrames" => st.gameplay_frames as f64,
            "gSaveContext.dayTime" | "gSaveContext.skyboxTime" => st.day_time as f64,
            "M_PI" => std::f64::consts::PI,
            _ => return None,
        })
    }

    fn num(&mut self, v: &Val) -> f64 {
        match v {
            Val::Num(x, _) => *x,
            Val::Name(n) | Val::Elem(n, _) => match self.known_name(n) {
                Some(x) => x,
                None => {
                    self.assumed.insert(n.clone());
                    0.0
                }
            },
            Val::Arr(_) | Val::Dl(..) | Val::Mtx => 0.0,
        }
    }

    fn is_float(&self, v: &Val) -> bool {
        matches!(v, Val::Num(_, true))
    }

    fn lookup(&self, name: &str) -> Option<Val> {
        self.vars.get(name).map(|v| v.1.clone())
    }

    /// Path string for an lvalue / name expression.
    fn path(&mut self, e: &Expr) -> Option<String> {
        match e {
            Expr::Name(n) => Some(n.clone()),
            Expr::Member(b, f) => Some(format!("{}.{f}", self.path(b)?)),
            Expr::Index(b, i) => {
                let base = self.path(b)?;
                let iv = self.eval(i);
                let idx = self.num(&iv);
                Some(format!("{base}[{idx}]"))
            }
            Expr::Un(op, x) if op == "*" || op == "&" => self.path(x),
            _ => None,
        }
    }

    fn store(&mut self, path: &str, v: Val) {
        let ty = self.vars.get(path).map(|x| x.0.clone()).unwrap_or_default();
        let v = match v {
            Val::Num(x, f) => {
                let (x, f) = wrap(&ty, x, f);
                Val::Num(x, f)
            }
            o => o,
        };
        self.vars.insert(path.to_string(), (ty, v));
    }

    fn new_dl(&mut self, cmds: Vec<Cmd>) -> Val {
        self.dls.push(cmds.into_iter().map(Some).collect());
        Val::Dl(self.dls.len() - 1, 0)
    }

    fn target(&mut self, e: &Expr) -> Target {
        let (inner, inc) = match e {
            Expr::PostInc(x, n) => (&**x, *n),
            _ => (e, 0),
        };
        let Some(p) = self.path(inner) else { return Target::None };
        match p.as_str() {
            "POLY_OPA_DISP" => Target::Opa,
            "POLY_XLU_DISP" => Target::Xlu,
            _ => match self.lookup(&p) {
                Some(Val::Dl(id, pos)) => {
                    if inc != 0 {
                        let ty = self.vars.get(&p).map(|x| x.0.clone()).unwrap_or_default();
                        self.vars.insert(p, (ty, Val::Dl(id, (pos as i64 + inc).max(0) as usize)));
                    }
                    Target::Dl(id, pos)
                }
                _ => Target::None,
            },
        }
    }

    fn emit(&mut self, target: &Target, c: Cmd) {
        // Syncs and ENDDL only matter inside allocated display lists.
        let is_sync = matches!(c, Cmd::Raw(w0, _) if matches!(w0 >> 24, 0xDF | 0xE7 | 0xE8));
        if is_sync && matches!(target, Target::Opa | Target::Xlu) {
            return;
        }
        match target {
            Target::Opa => self.opa.pre.push(c),
            Target::Xlu => self.xlu.pre.push(c),
            Target::Dl(id, pos) => {
                if let Some(d) = self.dls.get_mut(*id) {
                    if d.len() <= *pos {
                        d.resize(*pos + 1, None);
                    }
                    d[*pos] = Some(c);
                }
            }
            Target::None => {}
        }
    }

    /// GBI macro `name` (without the g/gs prefix) with the target already resolved.
    fn gbi(&mut self, name: &str, target: Target, args: &[Expr], src: &str) {
        let n = |m: &mut Self, k: usize| -> f64 {
            match args.get(k) {
                Some(e) => {
                    let v = m.eval(e);
                    m.num(&v)
                }
                None => 0.0,
            }
        };
        match name {
            "SPSegment" => {
                let seg = n(self, 0) as u8;
                let v = args.get(1).map(|e| self.eval(e)).unwrap_or(Val::Num(0.0, false));
                let bind = match v {
                    Val::Name(s) if self.known_name(&s).is_none() && !s.contains('.') => RawBind::Sym(s, Vec::new()),
                    Val::Elem(s, arr) => {
                        let variants = self.prog.arrays.get(&arr).map(|a| a.names.clone()).unwrap_or_default();
                        RawBind::Sym(s, variants)
                    }
                    Val::Dl(id, pos) => RawBind::DlRef(id, pos),
                    Val::Arr(a) => RawBind::Sym(a, Vec::new()),
                    Val::Mtx => RawBind::Mtx,
                    other => RawBind::Unknown(format!("{other:?}")),
                };
                let src = src.to_string();
                match target {
                    Target::Opa => {
                        self.opa.segments.insert(seg, (bind, src));
                    }
                    Target::Xlu => {
                        self.xlu.segments.insert(seg, (bind, src));
                    }
                    _ => {}
                }
            }
            "DPSetEnvColor" => {
                let c = rgba(n(self, 0), n(self, 1), n(self, 2), n(self, 3));
                self.emit(&target, Cmd::Raw(0xFB00_0000, c));
            }
            "DPSetPrimColor" => {
                let (m, l) = (n(self, 0) as u32 & 0xFF, n(self, 1) as u32 & 0xFF);
                let c = rgba(n(self, 2), n(self, 3), n(self, 4), n(self, 5));
                self.emit(&target, Cmd::Raw(0xFA00_0000 | (m << 8) | l, c));
            }
            "DPSetTileSize" => {
                let (uls, ult, lrs, lrt) = (n(self, 1) as u32, n(self, 2) as u32, n(self, 3) as u32, n(self, 4) as u32);
                let tile = n(self, 0) as u32 & 7;
                self.emit(&target, Cmd::Raw(0xF200_0000 | ((uls & 0xFFF) << 12) | (ult & 0xFFF), (tile << 24) | ((lrs & 0xFFF) << 12) | (lrt & 0xFFF)));
            }
            "DPPipeSync" => self.emit(&target, Cmd::Raw(0xE700_0000, 0)),
            "DPTileSync" => self.emit(&target, Cmd::Raw(0xE800_0000, 0)),
            "SPEndDisplayList" => self.emit(&target, Cmd::Raw(0xDF00_0000, 0)),
            "SPDisplayList" => {
                let v = args.first().map(|e| self.eval(e)).unwrap_or(Val::Num(0.0, false));
                match v {
                    Val::Arr(a) if self.prog.arrays.get(&a).is_some_and(|x| x.is_gfx) && matches!(target, Target::Opa | Target::Xlu) => {
                        let items = self.prog.arrays[&a].gfx.clone();
                        let is_opa = matches!(target, Target::Opa);
                        for it in &items {
                            if let Expr::Call(cn, cargs, csrc) = it
                                && let Some(short) = cn.strip_prefix("gs")
                            {
                                self.gbi(short, if is_opa { Target::Opa } else { Target::Xlu }, cargs, csrc);
                            }
                        }
                    }
                    Val::Name(s) | Val::Elem(s, _) => self.emit(&target, Cmd::CallSym(s)),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], src: &str) -> Val {
        if let Some(short) = name.strip_prefix('g').filter(|s| s.starts_with("SP") || s.starts_with("DP")) {
            let target = match args.first() {
                Some(t) => self.target(t),
                None => Target::None,
            };
            let src_arg = if short == "SPSegment" {
                // Keep the bound expression's text for the report.
                src.split_once(',').and_then(|(_, r)| r.split_once(',')).map(|(_, r)| r.trim().strip_suffix(')').unwrap_or(r).trim().to_string()).unwrap_or_default()
            } else {
                src.to_string()
            };
            self.gbi(short, target, &args[1.min(args.len())..], &src_arg);
            return Val::Num(0.0, false);
        }
        let a = |m: &mut Self, k: usize| -> f64 {
            match args.get(k) {
                Some(e) => {
                    let v = m.eval(e);
                    m.num(&v)
                }
                None => 0.0,
            }
        };
        match name {
            "SEGMENTED_TO_VIRTUAL" | "VIRTUAL_TO_PHYSICAL" | "OS_K0_TO_PHYSICAL" | "OS_PHYSICAL_TO_K0" => {
                args.first().map(|e| self.eval(e)).unwrap_or(Val::Num(0.0, false))
            }
            "Graph_Alloc" => self.new_dl(Vec::new()),
            "Matrix_NewMtx" => Val::Mtx,
            "Gfx_TexScroll" => {
                let (x, y, w, h) = (a(self, 1), a(self, 2), a(self, 3), a(self, 4));
                self.new_dl(vec![Cmd::Raw(0xE800_0000, 0), tile_size(0.0, x, y, w, h), Cmd::Raw(0xDF00_0000, 0)])
            }
            "Gfx_TwoTexScroll" | "Gfx_TwoTexScrollEnvColor" | "Gfx_TwoTexScrollPrimColor" => {
                let v: Vec<f64> = (1..args.len()).map(|k| a(self, k)).collect();
                let g = |k: usize| v.get(k).copied().unwrap_or(0.0);
                let mut cmds = vec![
                    Cmd::Raw(0xE800_0000, 0),
                    tile_size(g(0), g(1), g(2), g(3), g(4)),
                    Cmd::Raw(0xE800_0000, 0),
                    tile_size(g(5), g(6), g(7), g(8), g(9)),
                ];
                if name == "Gfx_TwoTexScrollEnvColor" {
                    cmds.push(Cmd::Raw(0xFB00_0000, rgba(g(10), g(11), g(12), g(13))));
                } else if name == "Gfx_TwoTexScrollPrimColor" {
                    cmds.push(Cmd::Raw(0xFA00_0000, rgba(g(10), g(11), g(12), g(13))));
                }
                cmds.push(Cmd::Raw(0xDF00_0000, 0));
                self.new_dl(cmds)
            }
            "Gfx_EnvColor" => {
                let c = rgba(a(self, 1), a(self, 2), a(self, 3), a(self, 4));
                self.new_dl(vec![Cmd::Raw(0xFB00_0000, c), Cmd::Raw(0xDF00_0000, 0)])
            }
            "CLOCK_TIME" => {
                let (h, m) = (a(self, 0), a(self, 1));
                // macros.h: ((s32)(((hr) * 60 + (min)) * (f32)0x10000 / (24 * 60) + 0.5f))
                Val::Num((((h * 60.0 + m) * 65536.0 / 1440.0) as f32 + 0.5).trunc() as f64, false)
            }
            "coss" | "sins" => {
                let x = a(self, 0);
                let r = x * std::f64::consts::TAU / 65536.0;
                let f = if name == "coss" { r.cos() } else { r.sin() };
                Val::Num((f * 32767.0).trunc(), false)
            }
            "Math_CosS" | "Math_SinS" => {
                let r = a(self, 0) * std::f64::consts::TAU / 65536.0;
                Val::Num(if name == "Math_CosS" { r.cos() } else { r.sin() }, true)
            }
            "sinf" | "cosf" | "Math_SinF" | "Math_CosF" => {
                let r = a(self, 0);
                Val::Num(if name.contains("in") { r.sin() } else { r.cos() }, true)
            }
            "ABS" | "fabsf" => Val::Num(a(self, 0).abs(), false),
            "OPEN_DISPS" | "CLOSE_DISPS" => Val::Num(0.0, false),
            _ => {
                for e in args {
                    let _ = self.eval(e);
                }
                self.assumed.insert(format!("{name}()"));
                Val::Num(0.0, false)
            }
        }
    }

    fn eval(&mut self, e: &Expr) -> Val {
        match e {
            Expr::Num(v, f) => Val::Num(*v, *f),
            Expr::Str => Val::Num(0.0, false),
            Expr::Name(n) => {
                if let Some(v) = self.lookup(n) {
                    return v;
                }
                if self.prog.arrays.contains_key(n) {
                    return Val::Arr(n.clone());
                }
                Val::Name(n.clone())
            }
            Expr::Member(..) => {
                let p = self.path(e).unwrap_or_else(|| "?".into());
                let p = p.replace("->", ".");
                self.lookup(&p).unwrap_or(Val::Name(p))
            }
            Expr::Index(b, i) => {
                let bv = self.eval(b);
                let iv = self.eval(i);
                let idx = self.num(&iv);
                match bv {
                    Val::Arr(a) => {
                        let el = self.prog.arrays.get(&a).and_then(|x| x.names.get(idx.max(0.0) as usize).cloned());
                        match el {
                            Some(el) => Val::Elem(el, a),
                            None => Val::Num(0.0, false),
                        }
                    }
                    _ => {
                        let p = self.path(e).unwrap_or_else(|| "?".into());
                        self.lookup(&p).unwrap_or(Val::Name(p))
                    }
                }
            }
            Expr::Call(n, args, src) => self.call(n, args, src),
            Expr::Un(op, x) => {
                let v = self.eval(x);
                match op.as_str() {
                    "&" | "*" | "+" => v,
                    "-" => {
                        let f = self.is_float(&v);
                        Val::Num(-self.num(&v), f)
                    }
                    "!" => Val::Num(if self.num(&v) == 0.0 { 1.0 } else { 0.0 }, false),
                    "~" => Val::Num(!(self.num(&v) as i64) as f64, false),
                    _ => v,
                }
            }
            Expr::Cast(ty, x) => {
                let v = self.eval(x);
                match v {
                    Val::Num(..) | Val::Name(_) | Val::Elem(..) if !matches!(ty.as_str(), "Gfx") => {
                        let f = self.is_float(&v);
                        let n = self.num(&v);
                        let (n, f) = wrap(ty, n, f);
                        Val::Num(n, f)
                    }
                    o => o,
                }
            }
            Expr::Bin(op, a, b) => {
                let av = self.eval(a);
                if op == "&&" || op == "||" {
                    let x = self.num(&av) != 0.0;
                    if (op == "&&" && !x) || (op == "||" && x) {
                        return Val::Num(x as i32 as f64, false);
                    }
                    let bv = self.eval(b);
                    return Val::Num((self.num(&bv) != 0.0) as i32 as f64, false);
                }
                let bv = self.eval(b);
                let f = self.is_float(&av) || self.is_float(&bv);
                let (x, y) = (self.num(&av), self.num(&bv));
                let (xi, yi) = (x.trunc() as i64, y.trunc() as i64);
                let r = match op.as_str() {
                    "+" => x + y,
                    "-" => x - y,
                    "*" => x * y,
                    "/" => {
                        if f {
                            if y == 0.0 { 0.0 } else { x / y }
                        } else if yi == 0 {
                            0.0
                        } else {
                            (xi / yi) as f64
                        }
                    }
                    "%" => {
                        if yi == 0 {
                            0.0
                        } else {
                            (xi % yi) as f64
                        }
                    }
                    "<<" => (xi << (yi & 63)) as f64,
                    ">>" => (xi >> (yi & 63)) as f64,
                    "&" => (xi & yi) as f64,
                    "|" => (xi | yi) as f64,
                    "^" => (xi ^ yi) as f64,
                    "==" => (x == y) as i32 as f64,
                    "!=" => (x != y) as i32 as f64,
                    "<" => (x < y) as i32 as f64,
                    ">" => (x > y) as i32 as f64,
                    "<=" => (x <= y) as i32 as f64,
                    ">=" => (x >= y) as i32 as f64,
                    _ => 0.0,
                };
                let is_cmp = matches!(op.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">=" | "<<" | ">>" | "&" | "|" | "^" | "%");
                Val::Num(r, f && !is_cmp)
            }
            Expr::Tern(c, a, b) => {
                let cv = self.eval(c);
                if self.num(&cv) != 0.0 { self.eval(a) } else { self.eval(b) }
            }
            Expr::PostInc(x, n) | Expr::PreInc(x, n) => {
                let pre = matches!(e, Expr::PreInc(..));
                let Some(p) = self.path(x) else { return Val::Num(0.0, false) };
                let cur = self.lookup(&p).unwrap_or(Val::Name(p.clone()));
                let new = match &cur {
                    Val::Dl(id, pos) => Val::Dl(*id, (*pos as i64 + n).max(0) as usize),
                    other => {
                        let f = self.is_float(other);
                        Val::Num(self.num(other) + *n as f64, f)
                    }
                };
                self.store(&p, new.clone());
                if pre { new } else { cur }
            }
            Expr::Assign(op, l, r) => {
                let rv = self.eval(r);
                let Some(p) = self.path(l) else { return rv };
                let p = p.replace("->", ".");
                let v = if op == "=" {
                    rv
                } else {
                    let cur = self.lookup(&p).unwrap_or(Val::Name(p.clone()));
                    let bop = op.trim_end_matches('=').to_string();
                    let f = self.is_float(&cur) || self.is_float(&rv);
                    let (x, y) = (self.num(&cur), self.num(&rv));
                    let (xi, yi) = (x.trunc() as i64, y.trunc() as i64);
                    let r = match bop.as_str() {
                        "+" => x + y,
                        "-" => x - y,
                        "*" => x * y,
                        "/" => {
                            if y == 0.0 {
                                0.0
                            } else if f {
                                x / y
                            } else {
                                (xi / yi) as f64
                            }
                        }
                        "%" => {
                            if yi == 0 {
                                0.0
                            } else {
                                (xi % yi) as f64
                            }
                        }
                        "&" => (xi & yi) as f64,
                        "|" => (xi | yi) as f64,
                        "^" => (xi ^ yi) as f64,
                        "<<" => (xi << (yi & 63)) as f64,
                        ">>" => (xi >> (yi & 63)) as f64,
                        _ => y,
                    };
                    Val::Num(r, f)
                };
                self.store(&p, v.clone());
                v
            }
            Expr::Comma(v) => {
                let mut last = Val::Num(0.0, false);
                for x in v {
                    last = self.eval(x);
                }
                last
            }
        }
    }
}
