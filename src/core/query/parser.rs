//! Tokens parsed into an expression tree.

use crate::core::query::Op;
use crate::core::query::compile::EMPTY_LIST;
use crate::core::query::Val;
use crate::core::query::lexer::tokenize;
use crate::core::query::Expr;
use crate::core::query::lexer::Tok;

const MAX_DEPTH: usize = 64;

pub struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    depth: usize,
    is_field: &'a dyn Fn(&str) -> bool,
}

fn kw(t: &Tok, k: &str) -> bool {
    matches!(t, Tok::Word(w) if w.eq_ignore_ascii_case(k))
}

pub fn parse(input: &str, is_field: &dyn Fn(&str) -> bool) -> Result<Expr, String> {
    let toks = tokenize(input)?;
    if toks.is_empty() {
        return Ok(Expr::True);
    }
    let mut p = Parser { toks, pos: 0, depth: 0, is_field };
    let e = p.parse_or()?;
    if p.pos < p.toks.len() {
        return Err(format!("Unexpected {:?}", p.toks[p.pos]));
    }
    Ok(e)
}

const OP_WORDS: &[&str] = &[
    "CONTAINS", "IN", "BETWEEN", "IS", "STARTS", "ENDS", "STARTSWITH", "ENDSWITH", "MATCHES", "HAS", "WITHIN", "LIKE",
    "BEFORE", "AFTER", "ON", "SIMILAR",
];

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn peek_at(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut items = vec![self.parse_and()?];
        while let Some(t) = self.peek() {
            if kw(t, "OR") {
                self.pos += 1;
                items.push(self.parse_and()?);
            } else {
                break;
            }
        }
        Ok(if items.len() == 1 { items.pop().unwrap() } else { Expr::Or(items) })
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut items = vec![self.parse_unary()?];
        loop {
            match self.peek() {
                None => break,
                Some(Tok::RParen) => break,
                Some(t) if kw(t, "OR") => break,
                Some(t) if kw(t, "AND") => {
                    self.pos += 1;
                    items.push(self.parse_unary()?);
                }
                Some(_) => items.push(self.parse_unary()?),
            }
        }
        Ok(if items.len() == 1 { items.pop().unwrap() } else { Expr::And(items) })
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(t) if kw(t, "NOT") => {
                if self.depth >= MAX_DEPTH {
                    return Err("Query is nested too deeply".into());
                }
                self.pos += 1;
                self.depth += 1;
                let inner = self.parse_unary();
                self.depth -= 1;
                Ok(Expr::Not(Box::new(inner?)))
            }
            Some(Tok::Minus) => {
                if self.depth >= MAX_DEPTH {
                    return Err("Query is nested too deeply".into());
                }
                self.pos += 1;
                self.depth += 1;
                let inner = self.parse_unary();
                self.depth -= 1;
                Ok(Expr::Not(Box::new(inner?)))
            }
            _ => self.parse_primary(),
        }
    }

    /// Numbers separated by commas up to the closing parenthesis, which is taken too.
    fn numbers(&mut self) -> Result<Vec<f64>, String> {
        let mut out = Vec::new();
        loop {
            let neg = if self.peek() == Some(&Tok::Minus) {
                self.pos += 1;
                true
            } else {
                false
            };
            match self.next() {
                Some(Tok::Word(w)) => {
                    let (w, neg) = match w.strip_prefix('-') {
                        Some(rest) => (rest.to_string(), !neg),
                        None => (w, neg),
                    };
                    let n: f64 = w.parse().map_err(|_| format!("'{w}' isn't a number"))?;
                    if !n.is_finite() {
                        return Err(format!("'{w}' isn't a number"));
                    }
                    out.push(if neg { -n } else { n });
                }
                other => return Err(format!("Expected a number, found {other:?}")),
            }
            match self.next() {
                Some(Tok::Comma) => continue,
                Some(Tok::RParen) => return Ok(out),
                _ => return Err("Missing ')'".into()),
            }
        }
    }

    fn op_follows(&self) -> bool {
        self.op_at(1)
    }
    fn op_at(&self, n: usize) -> bool {
        match self.peek_at(n) {
            Some(Tok::Sym(_)) => true,
            Some(t) => {
                OP_WORDS.iter().any(|k| kw(t, k))
                    || (kw(t, "NOT")
                        && self
                            .peek_at(n + 1)
                            .map(|t2| kw(t2, "IN") || kw(t2, "CONTAINS") || kw(t2, "LIKE"))
                            .unwrap_or(false))
            }
            None => false,
        }
    }
    /// `WITH` and a field's condition after it; "with" before anything else is a word to search for.
    fn with_follows(&self) -> bool {
        matches!(self.peek(), Some(t) if kw(t, "WITH")) && matches!(self.peek_at(1), Some(Tok::Word(w)) if (self.is_field)(w)) && self.op_at(2)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            None => Err("Unexpected end of query".into()),
            Some(Tok::LParen) => {
                if self.depth >= MAX_DEPTH {
                    return Err("Query is nested too deeply".into());
                }
                self.pos += 1;
                self.depth += 1;
                let e = self.parse_or();
                self.depth -= 1;
                let e = e?;
                match self.next() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err("Missing ')'".into()),
                }
            }
            Some(Tok::RParen) => Err("Unexpected ')'".into()),
            Some(Tok::Comma) => Err("Unexpected ','".into()),
            Some(Tok::Sym(s)) => Err(format!("Unexpected '{s}'")),
            Some(Tok::Minus) => Err("Unexpected '-'".into()),
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Expr::Text(s))
            }
            Some(Tok::Word(w)) if (kw(&Tok::Word(w.clone()), "NEAR") || kw(&Tok::Word(w.clone()), "INSIDE")) && self.peek_at(1) == Some(&Tok::LParen) => {
                self.pos += 2;
                let n = self.numbers()?;
                if kw(&Tok::Word(w.clone()), "NEAR") {
                    let [lat, lon, km] = n[..] else { return Err("NEAR takes a latitude, a longitude and a distance in km".into()) };
                    if lat.abs() > 90.0 || lon.abs() > 180.0 || km <= 0.0 {
                        return Err("NEAR: latitude -90 to 90, longitude -180 to 180, and more than 0 km".into());
                    }
                    Ok(Expr::Near { lat, lon, km })
                } else {
                    let [south, west, north, east] = n[..] else { return Err("INSIDE takes south, west, north and east".into()) };
                    if south > north || [south, north].iter().any(|x| x.abs() > 90.0) || [west, east].iter().any(|x| x.abs() > 180.0) {
                        return Err("INSIDE: south to north between -90 and 90, west and east between -180 and 180".into());
                    }
                    Ok(Expr::Inside { south, west, north, east })
                }
            }
            Some(Tok::Word(w)) => {
                if (self.is_field)(&w) && self.op_follows() {
                    self.pos += 1;
                    let mut all = vec![self.parse_comparison(w)?];
                    while self.with_follows() {
                        let Some(Tok::Word(f)) = self.peek_at(1).cloned() else { unreachable!() };
                        self.pos += 2;
                        all.push(self.parse_comparison(f)?);
                    }
                    return Ok(if all.len() == 1 { all.pop().unwrap() } else { Expr::With(all) });
                }
                self.pos += 1;
                Ok(Expr::Text(w))
            }
        }
    }

    fn parse_value(&mut self) -> Result<Val, String> {
        match self.next() {
            Some(Tok::Str(s)) => Ok(Val::Str(s)),
            Some(Tok::Word(w)) => Ok(word_value(&w)),
            Some(Tok::Minus) => match self.next() {
                Some(Tok::Word(w)) => Ok(Val::Str(format!("-{w}"))),
                _ => Err("Expected a value".into()),
            },
            Some(Tok::LParen) => {
                if self.depth >= MAX_DEPTH {
                    return Err("Query is nested too deeply".into());
                }
                self.depth += 1;
                let mut items = Vec::new();
                let res = (|| {
                    loop {
                        if let Some(Tok::RParen) = self.peek() {
                            self.pos += 1;
                            break;
                        }
                        items.push(self.parse_value()?);
                        match self.next() {
                            Some(Tok::Comma) => continue,
                            Some(Tok::RParen) => break,
                            _ => return Err("Expected ',' or ')' in list".into()),
                        }
                    }
                    if items.is_empty() {
                        return Err(EMPTY_LIST.into());
                    }
                    Ok(Val::List(items))
                })();
                self.depth -= 1;
                res
            }
            other => Err(format!("Expected a value, found {other:?}")),
        }
    }

    fn parse_comparison(&mut self, field: String) -> Result<Expr, String> {
        let t = self.next().ok_or("Expected an operator")?;
        let op = match &t {
            Tok::Sym(s) => match s.as_str() {
                "=" => Op::Eq,
                "!=" | "<>" => Op::Ne,
                "<" => Op::Lt,
                "<=" => Op::Le,
                ">" => Op::Gt,
                ">=" => Op::Ge,
                "~" => Op::Similar,
                "!~" => Op::NotContains,
                ":" => {
                    // shorthand field:value or field:>=value
                    if let Some(Tok::Sym(s2)) = self.peek().cloned() {
                        self.pos += 1;
                        let op = match s2.as_str() {
                            "<" => Op::Lt,
                            "<=" => Op::Le,
                            ">" => Op::Gt,
                            ">=" => Op::Ge,
                            "=" => Op::Eq,
                            "!=" => Op::Ne,
                            _ => return Err(format!("Unexpected '{s2}'")),
                        };
                        let v = self.parse_value()?;
                        return Ok(Expr::Cmp { field, op, value: v });
                    }
                    let v = self.parse_value()?;
                    return Ok(Expr::Cmp { field, op: Op::Similar, value: v });
                }
                other => return Err(format!("Unknown operator '{other}'")),
            },
            t if kw(t, "CONTAINS") || kw(t, "LIKE") => Op::Contains,
            t if kw(t, "STARTS") || kw(t, "STARTSWITH") => {
                if self.peek().map(|t| kw(t, "WITH")).unwrap_or(false) {
                    self.pos += 1;
                }
                Op::Starts
            }
            t if kw(t, "ENDS") || kw(t, "ENDSWITH") => {
                if self.peek().map(|t| kw(t, "WITH")).unwrap_or(false) {
                    self.pos += 1;
                }
                Op::Ends
            }
            t if kw(t, "MATCHES") => Op::Matches,
            t if kw(t, "SIMILAR") => {
                if self.peek().map(|t| kw(t, "TO")).unwrap_or(false) {
                    self.pos += 1;
                }
                Op::Similar
            }
            t if kw(t, "IN") => Op::In,
            t if kw(t, "BEFORE") => Op::Lt,
            t if kw(t, "AFTER") => Op::Gt,
            t if kw(t, "ON") => Op::Eq,
            t if kw(t, "WITHIN") => Op::Within,
            t if kw(t, "NOT") => match self.next() {
                Some(t2) if kw(&t2, "IN") => Op::NotIn,
                Some(t2) if kw(&t2, "CONTAINS") || kw(&t2, "LIKE") => Op::NotContains,
                _ => return Err("Expected IN or CONTAINS after NOT".into()),
            },
            t if kw(t, "IS") => {
                let mut negate = false;
                if self.peek().map(|t| kw(t, "NOT")).unwrap_or(false) {
                    self.pos += 1;
                    negate = true;
                }
                return match self.next() {
                    Some(t2) if kw(&t2, "EMPTY") || kw(&t2, "NULL") => Ok(Expr::Cmp {
                        field,
                        op: if negate { Op::NotEmpty } else { Op::Empty },
                        value: Val::None,
                    }),
                    Some(Tok::Word(w)) => {
                        Ok(Expr::Cmp { field, op: if negate { Op::Ne } else { Op::Eq }, value: word_value(&w) })
                    }
                    Some(Tok::Str(s)) => Ok(Expr::Cmp { field, op: if negate { Op::Ne } else { Op::Eq }, value: Val::Str(s) }),
                    _ => Err("Expected EMPTY after IS".into()),
                };
            }
            t if kw(t, "BETWEEN") => {
                let a = self.parse_value()?;
                match self.next() {
                    Some(t2) if kw(&t2, "AND") => {}
                    _ => return Err("Expected AND in BETWEEN".into()),
                }
                let b = self.parse_value()?;
                return Ok(Expr::Cmp { field, op: Op::Between, value: Val::Pair(Box::new(a), Box::new(b)) });
            }
            t if kw(t, "HAS") => {
                let op = match self.peek() {
                    Some(t2) if kw(t2, "ALL") => {
                        self.pos += 1;
                        Op::HasAll
                    }
                    Some(t2) if kw(t2, "NONE") => {
                        self.pos += 1;
                        Op::HasNone
                    }
                    Some(t2) if kw(t2, "ANY") => {
                        self.pos += 1;
                        Op::In
                    }
                    _ => Op::Eq,
                };
                let v = self.parse_value()?;
                return Ok(Expr::Cmp { field, op, value: v });
            }
            other => return Err(format!("Expected an operator after '{field}', found {other:?}")),
        };
        let value = self.parse_value()?;
        Ok(Expr::Cmp { field, op, value })
    }
}

pub(super) fn word_value(w: &str) -> Val {
    let lower = w.to_lowercase();
    let mut num_end = 0;
    for (i, c) in lower.char_indices() {
        if c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+')) {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if num_end > 0 {
        if let Ok(n) = lower[..num_end].parse::<f64>() {
            let unit = &lower[num_end..];
            if unit.is_empty() {
                return Val::Num(n, None);
            }
            if ["b", "kb", "mb", "gb", "tb", "s", "sec", "m", "min", "h", "hr", "d", "w", "y", "ms", "px"].contains(&unit) {
                return Val::Num(n, Some(unit.to_string()));
            }
        }
    }
    Val::Str(w.to_string())
}
