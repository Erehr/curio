//! A font's licence as a short name people filter by, when its text or address is one of the licences fonts are commonly released under.

const KNOWN: &[(&[&str], &str)] = &[
    (&["open font license", "openfontlicense.org", "scripts.sil.org/ofl", "sil ofl", "ofl-1.1", "ofl 1.1"], "OFL-1.1"),
    (&["apache license", "apache.org/licenses"], "Apache-2.0"),
    (&["ubuntu font licen"], "UFL-1.0"),
    (&["liberationfontlicense", "liberation fonts license"], "GPL-2.0 with exceptions"),
    (&["gnu lesser general public", "gnu library general public"], "LGPL"),
    (&["gnu general public license", "gnu.org/licenses/gpl"], "GPL"),
    (&["mit license", "opensource.org/licenses/mit"], "MIT"),
    (&["bitstream vera", "dejavu fonts license"], "Bitstream Vera"),
    (&["gust font license"], "GUST"),
    (&["ipa font license"], "IPA"),
    (&["arphic public license"], "Arphic"),
    (&["creativecommons.org/publicdomain/zero", "cc0"], "CC0"),
    (&["creativecommons.org/licenses/by-sa", "cc by-sa", "attribution-sharealike"], "CC BY-SA"),
    (&["creativecommons.org/licenses/by/", "cc by ", "creative commons attribution"], "CC BY"),
    (&["public domain"], "Public domain"),
];

/// How much of a licence's text is searched for its name.
const MAX_OPENING: usize = 300;
/// Longest a licence's first sentence is kept as its name.
const MAX_SENTENCE: usize = 160;

/// The licence's short name, else the first sentence of its description.
pub(super) fn name(text: Option<&str>, url: Option<&str>) -> Option<String> {
    // the licence's opening and address name it; a long text may quote others further on (Arial's cites the MIT licence for a part)
    let opening: String = text.unwrap_or_default().chars().take(MAX_OPENING).collect();
    let hay = format!("{opening} {}", url.unwrap_or_default()).to_lowercase();
    if let Some((_, n)) = KNOWN.iter().find(|(needles, _)| needles.iter().any(|n| hay.contains(n))) {
        return Some(n.to_string());
    }
    let text = text?.trim();
    let end = text.char_indices().find(|&(i, c)| c == '\n' || (c == '.' && text[i + 1..].starts_with(|n: char| n.is_whitespace()))).map(|(i, c)| if c == '.' { i + 1 } else { i }).unwrap_or(text.len());
    let first = text[..end].trim().trim_end_matches('.');
    if first.is_empty() {
        return None;
    }
    if first.chars().count() <= MAX_SENTENCE {
        return Some(first.to_string());
    }
    let cut: String = first.chars().take(MAX_SENTENCE - 1).collect();
    Some(format!("{}…", cut.trim_end()))
}
