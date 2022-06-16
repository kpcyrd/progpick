use crate::errors::*;
use crate::tokens::{self, Token};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct Pattern {
    fragments: Vec<Fragment>,
    first: bool,
    done: bool,
}

impl Pattern {
    #[inline]
    pub fn next<'a>(&mut self, out: &'a mut String) -> Option<&'a mut String> {
        if self.done {
            return None;
        }

        for frag in &mut self.fragments {
            match frag {
                Fragment::Chunk(chunk) => out.push_str(chunk),
                Fragment::Switch(switch) => {
                    switch.next(out);
                }
            }
        }

        self.done = self.bump();
        Some(out)
    }

    #[cfg(test)]
    #[inline]
    pub fn next_owned(&mut self) -> Option<String> {
        let mut out = String::new();

        if let Some(_) = self.next(&mut out) {
            Some(out)
        } else {
            None
        }
    }

    #[inline]
    pub fn bump(&mut self) -> bool {
        for frag in &mut self.fragments {
            match frag {
                Fragment::Chunk(_) => (),
                Fragment::Switch(switch) => {
                    if !switch.bump() {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub fn count(&self) -> usize {
        let mut sum = 1;

        for frag in &self.fragments {
            let m = match frag {
                Fragment::Chunk(_) => 1,
                Fragment::Switch(switch) => switch.count(),
            };
            sum *= m;
        }

        sum
    }
}

impl From<Vec<Fragment>> for Pattern {
    fn from(fragments: Vec<Fragment>) -> Pattern {
        Pattern {
            fragments,
            first: true,
            done: false,
        }
    }
}

impl FromStr for Pattern {
    type Err = Error;

    // TODO: this is executed twice(?!)
    fn from_str(s: &str) -> Result<Pattern> {
        let tokens = tokens::parse(s)?;
        debug!("parsed into tokens: {:?}", tokens);

        let mut switches: Vec<Switch> = Vec::new();
        let mut fragments = Vec::new();

        for token in tokens {
            debug!("adding token: {:?}", token);
            match token {
                Token::Chunk(chunk) => {
                    if let Some(tail) = switches.last_mut() {
                        tail.push(Fragment::Chunk(chunk));
                    } else {
                        fragments.push(Fragment::Chunk(chunk));
                    }
                }
                Token::SwitchOpen => {
                    switches.push(Switch::new());
                }
                Token::SwitchClose => {
                    let mut switch = switches.pop().unwrap();
                    switch.reset();

                    if let Some(tail) = switches.last_mut() {
                        tail.push(Fragment::Switch(switch));
                    } else {
                        fragments.push(Fragment::Switch(switch));
                    }
                }
                Token::SwitchNext => {
                    let tail = switches.last_mut().unwrap();
                    tail.bump_write_cursor();
                }
            }
        }

        Ok(Pattern::from(fragments))
    }
}

#[derive(Debug, PartialEq)]
pub enum Fragment {
    Chunk(String),
    Switch(Switch),
}

#[derive(Debug, PartialEq)]
pub struct Switch {
    options: Vec<Vec<Fragment>>,
    ctr: usize,
    write_cursor: usize,
}

impl Switch {
    #[inline]
    pub fn new() -> Switch {
        Switch {
            options: Vec::new(),
            ctr: 0,
            write_cursor: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, frag: Fragment) {
        if self.write_cursor + 1 > self.options.len() {
            self.options.push(Vec::new());
        }

        let tail = &mut self.options[self.write_cursor];
        tail.push(frag);
    }

    #[inline(always)]
    pub fn bump_write_cursor(&mut self) {
        self.write_cursor += 1;
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.write_cursor = 0;
    }

    #[inline]
    pub fn next(&mut self, out: &mut String) {
        if let Some(opt) = self.options.get_mut(self.ctr) {
            for frag in opt {
                match frag {
                    Fragment::Chunk(chunk) => out.push_str(chunk),
                    Fragment::Switch(switch) => {
                        switch.next(out);
                    }
                };
            }
        }
    }

    #[inline]
    pub fn bump(&mut self) -> bool {
        if let Some(opt) = self.options.get_mut(self.ctr) {
            for frag in opt {
                match frag {
                    Fragment::Chunk(_) => (),
                    Fragment::Switch(switch) => {
                        if !switch.bump() {
                            return false;
                        }
                    }
                }
            }
        }

        self.ctr += 1;

        if self.ctr >= self.options.len() {
            self.ctr = 0;
            true
        } else {
            false
        }
    }

    pub fn count(&self) -> usize {
        let mut sum = 0;

        for fragments in &self.options {
            let mut sum2 = 1;

            for frag in fragments {
                let m = match frag {
                    Fragment::Chunk(_) => 1,
                    Fragment::Switch(switch) => switch.count(),
                };
                sum2 *= m;
            }

            sum += sum2;
        }

        sum
    }
}

impl From<Vec<Vec<Fragment>>> for Switch {
    fn from(options: Vec<Vec<Fragment>>) -> Switch {
        Switch {
            options,
            ctr: 0,
            write_cursor: 0,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn all(mut p: Pattern) -> Vec<String> {
        let mut v = Vec::new();
        while let Some(s) = p.next_owned() {
            v.push(s);
        }
        v
    }

    #[test]
    fn simple() {
        let p = Pattern::from_str("abc").unwrap();
        let p2 = Pattern::from(vec![Fragment::Chunk(String::from("abc"))]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(all(p2), vec![String::from("abc"),]);
    }

    #[test]
    fn empty() {
        let p = Pattern::from_str("").unwrap();
        let p2 = Pattern::from(vec![]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(all(p2), vec![String::new(),]);
    }

    #[test]
    fn switch() {
        let p = Pattern::from_str("abc{x,y,z}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Chunk(String::from("abc")),
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Chunk(String::from("x"))],
                vec![Fragment::Chunk(String::from("y"))],
                vec![Fragment::Chunk(String::from("z"))],
            ])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("abcx"),
                String::from("abcy"),
                String::from("abcz"),
            ]
        );
    }

    #[test]
    fn nested() {
        let p = Pattern::from_str("abc{x,{y,z}}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Chunk(String::from("abc")),
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Chunk(String::from("x"))],
                vec![Fragment::Switch(Switch::from(vec![
                    vec![Fragment::Chunk(String::from("y"))],
                    vec![Fragment::Chunk(String::from("z"))],
                ]))],
            ])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("abcx"),
                String::from("abcy"),
                String::from("abcz"),
            ]
        );
    }

    #[test]
    fn nested_multiple_times() {
        let p = Pattern::from_str("{{{a,b,c},x},y}").unwrap();
        let p2 = Pattern::from(vec![Fragment::Switch(Switch::from(vec![
            vec![Fragment::Switch(Switch::from(vec![
                vec![Fragment::Switch(Switch::from(vec![
                    vec![Fragment::Chunk(String::from("a"))],
                    vec![Fragment::Chunk(String::from("b"))],
                    vec![Fragment::Chunk(String::from("c"))],
                ]))],
                vec![Fragment::Chunk(String::from("x"))],
            ]))],
            vec![Fragment::Chunk(String::from("y"))],
        ]))]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("a"),
                String::from("b"),
                String::from("c"),
                String::from("x"),
                String::from("y"),
            ]
        );
    }

    #[test]
    fn prefix_nested() {
        let p = Pattern::from_str("abc{x{y,z}}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Chunk(String::from("abc")),
            Fragment::Switch(Switch::from(vec![vec![
                Fragment::Chunk(String::from("x")),
                Fragment::Switch(Switch::from(vec![
                    vec![Fragment::Chunk(String::from("y"))],
                    vec![Fragment::Chunk(String::from("z"))],
                ])),
            ]])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(all(p2), vec![String::from("abcxy"), String::from("abcxz"),]);
    }

    #[test]
    fn prefix_nested_middle() {
        let p = Pattern::from_str("a{b,c{x,y},d}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Chunk(String::from("a")),
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Chunk(String::from("b"))],
                vec![
                    Fragment::Chunk(String::from("c")),
                    Fragment::Switch(Switch::from(vec![
                        vec![Fragment::Chunk(String::from("x"))],
                        vec![Fragment::Chunk(String::from("y"))],
                    ])),
                ],
                vec![Fragment::Chunk(String::from("d"))],
            ])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("ab"),
                String::from("acx"),
                String::from("acy"),
                String::from("ad"),
            ]
        );
    }

    #[test]
    fn chained() {
        let p = Pattern::from_str("{x,y,z}{x,y,z}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Chunk(String::from("x"))],
                vec![Fragment::Chunk(String::from("y"))],
                vec![Fragment::Chunk(String::from("z"))],
            ])),
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Chunk(String::from("x"))],
                vec![Fragment::Chunk(String::from("y"))],
                vec![Fragment::Chunk(String::from("z"))],
            ])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("xx"),
                String::from("yx"),
                String::from("zx"),
                String::from("xy"),
                String::from("yy"),
                String::from("zy"),
                String::from("xz"),
                String::from("yz"),
                String::from("zz"),
            ]
        );
    }

    #[test]
    fn chained_single_items_once() {
        let p = Pattern::from_str("{a}{a}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Switch(Switch::from(vec![vec![Fragment::Chunk(String::from("a"))]])),
            Fragment::Switch(Switch::from(vec![vec![Fragment::Chunk(String::from("a"))]])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(all(p2), vec![String::from("aa"),]);
    }

    #[test]
    fn chained_single_items_twice() {
        let p = Pattern::from_str("{a}{a}{a}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Switch(Switch::from(vec![vec![Fragment::Chunk(String::from("a"))]])),
            Fragment::Switch(Switch::from(vec![vec![Fragment::Chunk(String::from("a"))]])),
            Fragment::Switch(Switch::from(vec![vec![Fragment::Chunk(String::from("a"))]])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(all(p2), vec![String::from("aaa"),]);
    }

    #[test]
    fn empty_switch() {
        let p = Pattern::from_str("{}").unwrap();
        let p2 = Pattern::from(vec![Fragment::Switch(Switch::from(vec![]))]);
        assert_eq!(p, p2);
        // assert_eq!(p.count(), all(p).len());
        assert_eq!(all(p2), vec![String::new()]);
    }

    #[test]
    fn optional_prefix() {
        let p = Pattern::from_str("{{a..b},}x").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Switch(Switch::from(vec![
                    vec![Fragment::Chunk(String::from("a"))],
                    vec![Fragment::Chunk(String::from("b"))],
                ]))],
                vec![Fragment::Chunk(String::from(""))],
            ])),
            Fragment::Chunk(String::from("x")),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![String::from("ax"), String::from("bx"), String::from("x"),]
        );
    }

    #[test]
    fn one_empty_chunk_right() {
        let p = Pattern::from_str("abc{x,y,}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Chunk(String::from("abc")),
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Chunk(String::from("x"))],
                vec![Fragment::Chunk(String::from("y"))],
                vec![Fragment::Chunk(String::from(""))],
            ])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("abcx"),
                String::from("abcy"),
                String::from("abc"),
            ]
        );
    }

    #[test]
    fn one_empty_chunk_left() {
        let p = Pattern::from_str("abc{,x,y}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Chunk(String::from("abc")),
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Chunk(String::from(""))],
                vec![Fragment::Chunk(String::from("x"))],
                vec![Fragment::Chunk(String::from("y"))],
            ])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("abc"),
                String::from("abcx"),
                String::from("abcy"),
            ]
        );
    }

    #[test]
    fn one_empty_chunk_center() {
        let p = Pattern::from_str("abc{x,,y}").unwrap();
        let p2 = Pattern::from(vec![
            Fragment::Chunk(String::from("abc")),
            Fragment::Switch(Switch::from(vec![
                vec![Fragment::Chunk(String::from("x"))],
                vec![Fragment::Chunk(String::from(""))],
                vec![Fragment::Chunk(String::from("y"))],
            ])),
        ]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("abcx"),
                String::from("abc"),
                String::from("abcy"),
            ]
        );
    }

    #[test]
    fn numeric_range() {
        let p = Pattern::from_str("{0..9}").unwrap();
        let p2 = Pattern::from(vec![Fragment::Switch(Switch::from(vec![
            vec![Fragment::Chunk(String::from("0"))],
            vec![Fragment::Chunk(String::from("1"))],
            vec![Fragment::Chunk(String::from("2"))],
            vec![Fragment::Chunk(String::from("3"))],
            vec![Fragment::Chunk(String::from("4"))],
            vec![Fragment::Chunk(String::from("5"))],
            vec![Fragment::Chunk(String::from("6"))],
            vec![Fragment::Chunk(String::from("7"))],
            vec![Fragment::Chunk(String::from("8"))],
            vec![Fragment::Chunk(String::from("9"))],
        ]))]);
        assert_eq!(p, p2);
        assert_eq!(p.count(), all(p).len());
        assert_eq!(
            all(p2),
            vec![
                String::from("0"),
                String::from("1"),
                String::from("2"),
                String::from("3"),
                String::from("4"),
                String::from("5"),
                String::from("6"),
                String::from("7"),
                String::from("8"),
                String::from("9"),
            ]
        );
    }

    #[test]
    fn numeric_range_errs() {
        assert!(Pattern::from_str("{..}").is_err());
        assert!(Pattern::from_str("{0..}").is_err());
        assert!(Pattern::from_str("{..0}").is_err());
        assert!(Pattern::from_str("{00..}").is_err());
        assert!(Pattern::from_str("{..00}").is_err());
        assert!(Pattern::from_str("{00..00}").is_err());
        assert!(Pattern::from_str("{.").is_err());
        assert!(Pattern::from_str("{..").is_err());
        assert!(Pattern::from_str("{...}").is_err());
        assert!(Pattern::from_str("{a...}").is_err());
    }
}
