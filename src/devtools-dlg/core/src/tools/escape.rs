//! Text as it is written inside a string literal: a backslash before what cannot stand as itself. The escapes are
//! the ones JSON, JavaScript, C and Rust share, so the result pastes into any of them.

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}

/// Reads every escape `escape` writes, and the common others: `\'`, `\/`, `\0`, `\xHH` and `\uXXXX` with surrogate
/// pairs. Anything else after a backslash is reported with where it is, rather than guessed at.
pub fn unescape(text: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().enumerate().peekable();
    while let Some((at, c)) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some((_, kind)) = chars.next() else {
            return Err(format!(
                "a backslash at character {} with nothing after it",
                at + 1
            ));
        };
        match kind {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            '0' => out.push('\0'),
            '\\' | '"' | '\'' | '/' => out.push(kind),
            'x' => {
                let code = hex_digits(&mut chars, 2)
                    .ok_or(format!("\\x at character {} wants two hex digits", at + 1))?;
                out.push(char::from(code as u8));
            }
            'u' => {
                let wants = || format!("\\u at character {} wants four hex digits", at + 1);
                let high = hex_digits(&mut chars, 4).ok_or_else(wants)?;
                let code = if (0xd800..0xdc00).contains(&high) {
                    let low = match (chars.next(), chars.next()) {
                        (Some((_, '\\')), Some((_, 'u'))) => hex_digits(&mut chars, 4),
                        _ => None,
                    }
                    .filter(|low| (0xdc00..0xe000).contains(low))
                    .ok_or(format!(
                        "\\u at character {} starts a surrogate pair that does not end",
                        at + 1
                    ))?;
                    0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00)
                } else {
                    high
                };
                out.push(
                    char::from_u32(code)
                        .ok_or(format!("\\u at character {} is not a character", at + 1))?,
                );
            }
            other => {
                return Err(format!(
                    "\\{other} at character {} is not an escape",
                    at + 1
                ))
            }
        }
    }
    Ok(out)
}

fn hex_digits(chars: &mut impl Iterator<Item = (usize, char)>, count: usize) -> Option<u32> {
    let mut code = 0;
    for _ in 0..count {
        code = code * 16 + chars.next()?.1.to_digit(16)?;
    }
    Some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_cannot_stand_in_a_string_is_written_with_a_backslash() {
        assert_eq!(
            escape("say \"hi\"\n\tC:\\path\u{1}"),
            "say \\\"hi\\\"\\n\\tC:\\\\path\\u0001"
        );
        assert_eq!(
            escape("привет — 😀"),
            "привет — 😀",
            "letters of any language stay as they are"
        );
    }

    #[test]
    fn unescaping_reads_back_what_escaping_wrote() {
        let text = "line one\r\nline \"two\"\t\\ end\u{7f}\u{8}\u{c}";
        assert_eq!(unescape(&escape(text)).unwrap(), text);
    }

    #[test]
    fn the_common_escapes_of_other_languages_are_read_too() {
        assert_eq!(unescape(r"it\'s \/ \x41\0").unwrap(), "it's / A\0");
        assert_eq!(unescape(r"é 😀").unwrap(), "é 😀");
    }

    #[test]
    fn a_broken_escape_is_reported_with_where_it_is() {
        assert_eq!(
            unescape(r"ab\q").unwrap_err(),
            r"\q at character 3 is not an escape"
        );
        assert!(unescape("ends with \\").is_err());
        assert!(unescape(r"\u12").is_err());
        assert!(unescape(r"\xZZ").is_err());
        assert!(unescape(r"\ud83d alone").is_err());
    }
}
