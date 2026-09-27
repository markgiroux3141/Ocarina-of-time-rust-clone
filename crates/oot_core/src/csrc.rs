//! Minimal reader for static C initializers in decomp source, so tables like Player's
//! display-list groups are taken from `z_player_lib.c` rather than copied by hand.
//!
//! Handles `type name[..][..] = { ... };` with nested braces, identifiers, numbers and
//! comments. Preprocessor conditionals are not evaluated: the first definition wins, which
//! for the decomp is the non-`AVOID_UB` (as-shipped) variant.

use anyhow::{Result, bail};

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
        let s = self.atom()?.trim();
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
    while i < b.len() {
        match b[i] {
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
    fn reads_enum_members_in_order() {
        let src = "typedef enum {\n /* 0x00 */ P_A,\n /* 0x01 */ P_B, // note\n /* 0x02 */ P_MAX\n} P;\nx = P_A;";
        assert_eq!(enum_members(src, "P_"), vec!["P_A", "P_B", "P_MAX"]);
    }
}
