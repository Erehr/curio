//! The query language and its compiler to SQL.
//!
//! Examples:
//!   model = "Anima" AND steps <= 12 AND cfg = 1
//!   prompt CONTAINS "blue hair" AND lora_strength >= 0.7
//!   (tag = cat OR tag = dog) AND NOT rating < 3 AND size > 2mb
//!   added WITHIN 7d  ext IN (png, webp)  -draft  shape = portrait  color ~ #3366ff
//!   model IS EMPTY   tags HAS ALL (sky, sea)   name MATCHES "^IMG_\d+"
//!   lora = detail WITH lora_strength BETWEEN 0.5 AND 1
//!
//! Grammar (case-insensitive keywords):
//!   expr    := and (OR and)*
//!   and     := unary ([AND] unary)*          -- juxtaposition is AND
//!   unary   := NOT unary | '-' unary | primary
//!   primary := '(' expr ')' | comparison (WITH comparison)* | text
//!   comparison := field op value | field ':' [op] value | field IS [NOT] EMPTY
//!               | field [NOT] IN '(' value, ... ')' | field BETWEEN value AND value
//!               | field HAS ALL|NONE|ANY '(' ... ')' | field WITHIN value
//! Bare words and quoted strings are full-text searched.
//! `WITH` joins conditions on fields that hold lists, met by values at the same place in each list.
//!
//! The compiler emits a WHERE fragment against `assets a` with positional parameters, so the
//! database does all filtering; nothing is loaded into memory to be filtered.

pub mod compile;
pub mod fields;
mod lexer;
pub mod parser;
pub mod saved_rules;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    NotContains,
    Starts,
    Ends,
    Matches,
    In,
    NotIn,
    Between,
    Empty,
    NotEmpty,
    HasAll,
    HasNone,
    Within,
    Similar,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    None,
    Str(String),
    /// number with optional unit suffix (kb, mb, gb, s, m, h, d, w)
    Num(f64, Option<String>),
    List(Vec<Val>),
    Pair(Box<Val>, Box<Val>),
}

impl Val {
    pub fn as_text(&self) -> String {
        match self {
            Val::Str(s) => s.clone(),
            Val::Num(n, u) => {
                let mut s = if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", *n as i64) } else { format!("{n}") };
                if let Some(u) = u {
                    s.push_str(u);
                }
                s
            }
            Val::None => String::new(),
            Val::List(v) => v.iter().map(|x| x.as_text()).collect::<Vec<_>>().join(","),
            Val::Pair(a, b) => format!("{}..{}", a.as_text(), b.as_text()),
        }
    }
    fn items(&self) -> Vec<Val> {
        match self {
            Val::List(v) => v.clone(),
            other => vec![other.clone()],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    True,
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
    Cmp { field: String, op: Op, value: Val },
    Text(String),
    /// Taken within `km` of a point: `NEAR(lat, lon, km)`.
    Near { lat: f64, lon: f64, km: f64 },
    /// Taken inside a box: `INSIDE(south, west, north, east)`; west past east crosses the 180th meridian.
    Inside { south: f64, west: f64, north: f64, east: f64 },
    /// Comparisons joined by `WITH`, each a `Cmp`, met by one value of each field at the same place in its list.
    With(Vec<Expr>),
}

