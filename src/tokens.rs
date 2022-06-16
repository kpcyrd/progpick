use crate::errors::*;

#[derive(Debug, PartialEq)]
pub enum Token {
    Chunk(String),
    SwitchOpen,
    SwitchClose,
    SwitchNext,
}

// TODO: refactor to iterator
// TODO: custom bail macro that points at error position: ~~~^
pub fn parse(s: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();

    let mut in_switch: u8 = 0;
    let mut escape = false;

    let mut x = String::new();
    let mut iter = s.chars().peekable();
    while let Some(c) = iter.next() {
        match c {
            c if escape => {
                x.push(c);
                escape = false;
            }
            '{' => {
                if !x.is_empty() {
                    tokens.push(Token::Chunk(x));
                    x = String::new();
                }
                tokens.push(Token::SwitchOpen);
                in_switch += 1;
            }
            '}' => {
                if in_switch == 0 {
                    bail!("unmatched }}, not in a switch statement");
                }

                if !x.is_empty() {
                    tokens.push(Token::Chunk(x));
                    x = String::new();
                }
                tokens.push(Token::SwitchClose);
                in_switch -= 1;

                // make sure the next ',' doesn't consider this an explicit empty option
                if iter.peek() == Some(&',') {
                    iter.next();
                    tokens.push(Token::SwitchNext);

                    // if this is an explicit empty option add empty string
                    if iter.peek() == Some(&'}') {
                        tokens.push(Token::Chunk(String::new()));
                    }
                }
            }
            ',' if in_switch > 0 => {
                tokens.push(Token::Chunk(x));
                tokens.push(Token::SwitchNext);

                // in case of an explicit last ',', push an empty chunk
                if iter.peek() == Some(&'}') {
                    tokens.push(Token::Chunk(String::new()));
                }

                x = String::new();
            }
            '.' if in_switch > 0 => {
                if iter.peek() == Some(&'.') {
                    iter.next(); // consume the value

                    // ensure start is a single byte
                    if x.as_bytes().len() != 1 {
                        bail!("range patterns only support a single ascii character");
                    }

                    let start = x.chars().next().unwrap();

                    // test for escape sequence
                    let mut end = iter
                        .next()
                        .context("unexpected end of string in range pattern")?;

                    if end == '\\' {
                        end = iter
                            .next()
                            .context("unexpected end of string in escape sequence")?;
                    }

                    // ensure end is also a single byte
                    if end.len_utf8() != 1 {
                        bail!("range patterns only support a single ascii character");
                    }

                    if iter.peek() != Some(&'}') {
                        bail!("range patterns only support a single ascii character");
                    }

                    // expand range
                    let start = start as u8;
                    let end = end as u8;

                    if start >= end {
                        bail!("start needs to be smaller than end");
                    }

                    for c in start..=end {
                        tokens.push(Token::Chunk((c as char).to_string()));
                        if c < end {
                            tokens.push(Token::SwitchNext);
                        }
                    }

                    x = String::new();
                } else {
                    x.push(c);
                }
            }
            '\\' => {
                escape = true;
            }
            c => x.push(c),
        };
    }

    if !x.is_empty() {
        tokens.push(Token::Chunk(x));
    }

    if in_switch > 0 {
        bail!("unmatched {{, still in switch at end of string");
    }

    Ok(tokens)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple() {
        let p = parse("abc").unwrap();
        assert_eq!(p, vec![Token::Chunk(String::from("abc")),]);
    }

    #[test]
    fn empty() {
        let p = parse("").unwrap();
        assert_eq!(p, vec![]);
    }

    #[test]
    fn switch() {
        let p = parse("abc{x,y,z}").unwrap();
        assert_eq!(
            p,
            vec![
                Token::Chunk(String::from("abc")),
                Token::SwitchOpen,
                Token::Chunk(String::from("x")),
                Token::SwitchNext,
                Token::Chunk(String::from("y")),
                Token::SwitchNext,
                Token::Chunk(String::from("z")),
                Token::SwitchClose,
            ]
        );
    }

    #[test]
    fn nested() {
        let p = parse("abc{x,{y,z}}").unwrap();
        assert_eq!(
            p,
            vec![
                Token::Chunk(String::from("abc")),
                Token::SwitchOpen,
                Token::Chunk(String::from("x")),
                Token::SwitchNext,
                Token::SwitchOpen,
                Token::Chunk(String::from("y")),
                Token::SwitchNext,
                Token::Chunk(String::from("z")),
                Token::SwitchClose,
                Token::SwitchClose,
            ]
        );
    }

    #[test]
    fn prefix_nested() {
        let p = parse("abc{x{y,z}}").unwrap();
        assert_eq!(
            p,
            vec![
                Token::Chunk(String::from("abc")),
                Token::SwitchOpen,
                Token::Chunk(String::from("x")),
                Token::SwitchOpen,
                Token::Chunk(String::from("y")),
                Token::SwitchNext,
                Token::Chunk(String::from("z")),
                Token::SwitchClose,
                Token::SwitchClose,
            ]
        );
    }

    #[test]
    fn optional_prefix() {
        let p = parse("{{a..b},}x").unwrap();
        assert_eq!(
            p,
            vec![
                Token::SwitchOpen,
                Token::SwitchOpen,
                Token::Chunk(String::from("a")),
                Token::SwitchNext,
                Token::Chunk(String::from("b")),
                Token::SwitchClose,
                Token::SwitchNext,
                Token::Chunk(String::from("")),
                Token::SwitchClose,
                Token::Chunk(String::from("x")),
            ]
        );
    }

    #[test]
    fn optional_prefix2() {
        let p = parse("{{a..b},,}x").unwrap();
        assert_eq!(
            p,
            vec![
                Token::SwitchOpen,
                Token::SwitchOpen,
                Token::Chunk(String::from("a")),
                Token::SwitchNext,
                Token::Chunk(String::from("b")),
                Token::SwitchClose,
                Token::SwitchNext,
                Token::Chunk(String::from("")),
                Token::SwitchNext,
                Token::Chunk(String::from("")),
                Token::SwitchClose,
                Token::Chunk(String::from("x")),
            ]
        );
    }

    #[test]
    fn one_empty_chunk_right() {
        let p = parse("abc{x,y,}").unwrap();
        assert_eq!(
            p,
            vec![
                Token::Chunk(String::from("abc")),
                Token::SwitchOpen,
                Token::Chunk(String::from("x")),
                Token::SwitchNext,
                Token::Chunk(String::from("y")),
                Token::SwitchNext,
                Token::Chunk(String::from("")),
                Token::SwitchClose,
            ]
        );
    }

    #[test]
    fn one_empty_chunk_left() {
        let p = parse("abc{,x,y}").unwrap();
        assert_eq!(
            p,
            vec![
                Token::Chunk(String::from("abc")),
                Token::SwitchOpen,
                Token::Chunk(String::from("")),
                Token::SwitchNext,
                Token::Chunk(String::from("x")),
                Token::SwitchNext,
                Token::Chunk(String::from("y")),
                Token::SwitchClose,
            ]
        );
    }

    #[test]
    fn one_empty_chunk_center() {
        let p = parse("abc{x,,y}").unwrap();
        assert_eq!(
            p,
            vec![
                Token::Chunk(String::from("abc")),
                Token::SwitchOpen,
                Token::Chunk(String::from("x")),
                Token::SwitchNext,
                Token::Chunk(String::from("")),
                Token::SwitchNext,
                Token::Chunk(String::from("y")),
                Token::SwitchClose,
            ]
        );
    }

    #[test]
    fn numeric_range() {
        let p = parse("{0..9}").unwrap();
        assert_eq!(
            p,
            vec![
                Token::SwitchOpen,
                Token::Chunk(String::from("0")),
                Token::SwitchNext,
                Token::Chunk(String::from("1")),
                Token::SwitchNext,
                Token::Chunk(String::from("2")),
                Token::SwitchNext,
                Token::Chunk(String::from("3")),
                Token::SwitchNext,
                Token::Chunk(String::from("4")),
                Token::SwitchNext,
                Token::Chunk(String::from("5")),
                Token::SwitchNext,
                Token::Chunk(String::from("6")),
                Token::SwitchNext,
                Token::Chunk(String::from("7")),
                Token::SwitchNext,
                Token::Chunk(String::from("8")),
                Token::SwitchNext,
                Token::Chunk(String::from("9")),
                Token::SwitchClose,
            ]
        );
    }
}
