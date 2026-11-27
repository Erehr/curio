//! The query text cut into tokens.

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Tok {
    LParen,
    RParen,
    Comma,
    Minus,
    Sym(String),
    Word(String),
    Str(String),
}

pub(super) fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '"' | '\'' => {
                let q = c;
                i += 1;
                let mut buf = String::new();
                let mut closed = false;
                while i < chars.len() {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        buf.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    if chars[i] == q {
                        closed = true;
                        i += 1;
                        break;
                    }
                    buf.push(chars[i]);
                    i += 1;
                }
                if !closed {
                    return Err("Unclosed quote".into());
                }
                out.push(Tok::Str(buf));
            }
            '=' | '<' | '>' | '!' | '~' | ':' => {
                let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
                let sym = match two.as_str() {
                    "<=" | ">=" | "!=" | "<>" | "!~" | "==" => {
                        i += 2;
                        two
                    }
                    _ => {
                        i += 1;
                        c.to_string()
                    }
                };
                if sym == "!" {
                    out.push(Tok::Word("NOT".into()));
                    continue;
                }
                out.push(Tok::Sym(if sym == "==" { "=".into() } else { sym }));
            }
            '-' if (i + 1 < chars.len())
                && !chars[i + 1].is_ascii_digit()
                && !chars[i + 1].is_whitespace()
                && (i == 0 || chars[i - 1].is_whitespace() || chars[i - 1] == '(') =>
            {
                out.push(Tok::Minus);
                i += 1;
            }
            _ => {
                let mut buf = String::new();
                while i < chars.len() {
                    let ch = chars[i];
                    if ch.is_whitespace() || "()\",=<>!~:'".contains(ch) {
                        break;
                    }
                    buf.push(ch);
                    i += 1;
                }
                if buf.is_empty() {
                    return Err(format!("Unexpected '{}'", chars[i]));
                }
                out.push(Tok::Word(buf));
            }
        }
    }
    Ok(out)
}
