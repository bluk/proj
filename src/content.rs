#[derive(Debug)]
enum State {
    SearchForBeginMarker,
    StartedBeginMarker {
        marker_count: usize,
    },
    EndedBeginMarker {
        marker_count: usize,
    },
    StartedFrontMatter {
        marker_count: usize,
        front_matter_start: usize,
        front_matter_end: usize,
        end_marker_count: Option<usize>,
    },
    EndedFrontMatter {
        front_matter_start: usize,
        front_matter_end: usize,
    },
    Done {
        front_matter_start: usize,
        front_matter_end: usize,
        contents_start: usize,
    },
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("invalid start marker")]
    InvalidStartMarker,
    #[error("end of file")]
    Eof,
}

pub fn parse(contents: &str) -> Result<(Option<&str>, usize, &str), Error> {
    let mut state = State::SearchForBeginMarker;
    let mut chars = contents.char_indices();

    loop {
        let (idx, ch) = chars.next().ok_or(Error::Eof)?;

        match &mut state {
            State::SearchForBeginMarker => match ch {
                '+' => {
                    state = State::StartedBeginMarker { marker_count: 1 };
                }
                '\n' | '\r' => {
                    // ignored
                }
                _ => {
                    return Ok((None, 0, contents));
                }
            },
            State::StartedBeginMarker { marker_count } => match ch {
                '+' => {
                    *marker_count += 1;
                }
                '\n' | '\r' => {
                    if *marker_count >= 3 {
                        state = State::EndedBeginMarker {
                            marker_count: *marker_count,
                        };
                    } else {
                        return Err(Error::InvalidStartMarker);
                    }
                }
                _ => {
                    return Err(Error::InvalidStartMarker);
                }
            },
            State::EndedBeginMarker { marker_count } => match ch {
                '\n' | '\r' => {
                    // ignore
                }
                '+' => {
                    state = State::StartedFrontMatter {
                        marker_count: *marker_count,
                        front_matter_start: idx,
                        front_matter_end: idx,
                        end_marker_count: Some(1),
                    };
                }
                _ => {
                    state = State::StartedFrontMatter {
                        marker_count: *marker_count,
                        front_matter_start: idx,
                        front_matter_end: idx,
                        end_marker_count: Some(0),
                    };
                }
            },
            State::StartedFrontMatter {
                marker_count,
                front_matter_start,
                front_matter_end,
                end_marker_count,
            } => match ch {
                '\n' | '\r' => {
                    if let Some(count) = end_marker_count {
                        debug_assert!(count <= marker_count);
                        if count == marker_count {
                            state = State::EndedFrontMatter {
                                front_matter_start: *front_matter_start,
                                front_matter_end: *front_matter_end,
                            }
                        } else {
                            *front_matter_end = idx;
                            *end_marker_count = Some(0);
                        }
                    } else {
                        *front_matter_end = idx;
                        *end_marker_count = Some(0);
                    }
                }
                '+' => {
                    if let Some(count) = end_marker_count {
                        *count += 1;
                    }
                }
                _ => {
                    *front_matter_end = idx;
                    *end_marker_count = None;
                }
            },
            State::EndedFrontMatter {
                front_matter_start,
                front_matter_end,
            } => {
                match ch {
                    '\n' | '\r' => {
                        // ignore
                    }
                    _ => {
                        state = State::Done {
                            front_matter_start: *front_matter_start,
                            front_matter_end: *front_matter_end,
                            contents_start: idx,
                        }
                    }
                }
            }
            State::Done {
                front_matter_start,
                front_matter_end,
                contents_start,
            } => {
                let (_, tail) = contents.split_at(*front_matter_start);
                let (front_matter, tail) = tail.split_at(*front_matter_end - *front_matter_start);
                let (_, contents) = tail.split_at(*contents_start - *front_matter_end);

                return Ok((Some(front_matter), *contents_start, contents));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_front_matter() {
        let input = r#"
Hello world!;
"#;

        assert_eq!(Ok((None, 0, input)), parse(input));
    }

    #[test]
    fn only_front_matter() {
        let input = r#"
+++
title = "Hello World!"

Hello.
"#;

        assert_eq!(Err(Error::Eof), parse(input));
    }

    #[test]
    fn invalid_start_marker_wrong_characters() {
        let input = r#"
+a+
title = "Hello World!"
+a+
Hello.
"#;

        assert_eq!(Err(Error::InvalidStartMarker), parse(input));
    }

    #[test]
    fn invalid_start_marker_not_enough_chars() {
        let input = r#"
++
title = "Hello World!"
++
Hello.
"#;

        assert_eq!(Err(Error::InvalidStartMarker), parse(input));
    }

    #[test]
    fn empty_front_matter() {
        let input = r#"
+++
+++
Hello."#;

        assert_eq!(Ok((Some(r#""#), 9, r#"Hello."#)), parse(input));
    }

    #[test]
    fn simple_front_matter() {
        let input = r#"
+++
title = "Hello World!"
+++
Hello."#;

        assert_eq!(
            Ok((Some(r#"title = "Hello World!""#), 32, r#"Hello."#)),
            parse(input)
        );
    }
}
