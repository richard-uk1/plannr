//! Generic helpers for parsing

use std::borrow::Cow;

/// Like the std library equivalent, except: owned -> owned (involves allocation)
///
/// returns original string if split was not possible
pub fn try_split_once<'a>(
    input: Cow<'a, str>,
    delim: char,
) -> Result<(Cow<'a, str>, Cow<'a, str>), Cow<'a, str>> {
    match input {
        Cow::Borrowed(s) => {
            let Some((before, after)) = s.split_once(delim) else {
                return Err(Cow::Borrowed(s));
            };
            Ok((Cow::Borrowed(before), Cow::Borrowed(after)))
        }
        Cow::Owned(mut s) => {
            let Some(split_idx) = s.find(delim) else {
                return Err(Cow::Owned(s));
            };
            let after = s.split_off(split_idx + delim.len_utf8());
            // pop delimiter
            s.pop();
            Ok((Cow::Owned(s), Cow::Owned(after)))
        }
    }
}

/// Like the std library equivalent, except:
///  1. owned -> owned (involves allocation)
///  2. if no delim then return all in first part
pub fn split_once<'a>(input: Cow<'a, str>, delim: char) -> (Cow<'a, str>, Cow<'a, str>) {
    match input {
        Cow::Borrowed(s) => match s.split_once(delim) {
            Some((before, after)) => (Cow::Borrowed(before), Cow::Borrowed(after)),
            None => (input, Cow::Borrowed("")),
        },
        Cow::Owned(mut s) => {
            let Some(split_idx) = s.find(delim) else {
                return (Cow::Owned(s), Cow::Borrowed(""));
            };
            let after = s.split_off(split_idx + delim.len_utf8());
            // pop delimiter
            s.pop();
            (Cow::Owned(s), Cow::Owned(after))
        }
    }
}

/// Split on the given character, or return everything in `.0` if it isn't
/// found.
///
/// This function will only split outside quoted strings (i.e. when the
/// number of seen quotes is odd).
pub fn split_once_outside_quotes<'a>(
    input: Cow<'a, str>,
    test_ch: char,
) -> (Cow<'a, str>, Cow<'a, str>) {
    match input {
        Cow::Borrowed(input) => split_once_outside_quotes_borrowed(input, test_ch),
        Cow::Owned(input) => split_once_outside_quotes_owned(input, test_ch),
    }
}

fn split_once_outside_quotes_borrowed<'a>(
    input: &'a str,
    test_ch: char,
) -> (Cow<'a, str>, Cow<'a, str>) {
    let mut chars = input.chars();
    let mut inside_quote = false;
    while let Some(ch) = chars.next() {
        if ch == '"' {
            inside_quote = !inside_quote;
        } else if ch == test_ch && !inside_quote {
            let rest = chars.as_str();
            let first = &input[..input.len() - rest.len() - test_ch.len_utf8()];
            return (Cow::Borrowed(first), Cow::Borrowed(rest));
        }
    }
    (Cow::Borrowed(input), Cow::Borrowed(""))
}

fn split_once_outside_quotes_owned(
    mut input: String,
    test_ch: char,
) -> (Cow<'static, str>, Cow<'static, str>) {
    let mut chars = input.chars();
    let mut inside_quote = false;
    let mut first_end_idx = None;
    while let Some(ch) = chars.next() {
        if ch == '"' {
            inside_quote = !inside_quote;
        } else if ch == test_ch && !inside_quote {
            let rest = chars.as_str();
            first_end_idx = Some(input.len() - rest.len());
            break;
        }
    }
    let Some(first_end_idx) = first_end_idx else {
        return (Cow::Owned(input), Cow::Borrowed(""));
    };
    let after = input.split_off(first_end_idx);
    // remove `test_ch`
    input.pop();
    (Cow::Owned(input), Cow::Owned(after))
}

pub fn pop_front_bytes(input: &mut String, chars: usize) {
    let mut finished = false;
    let mut count = 0;
    input.retain(|_ch| {
        if finished {
            return true;
        }
        if count >= chars {
            finished = true;
            return true;
        }
        count += 1;
        false
    });
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    #[test]
    fn split_once() {
        let input = "first;second;";

        let (first, rest) = super::split_once(Cow::Borrowed(input), ';');
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let (first, rest) = super::split_once(Cow::Owned(input.to_string()), ';');
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let input = "first second";

        let (first, rest) = super::split_once(Cow::Borrowed(input), ';');
        assert_eq!(first, "first second");
        assert_eq!(rest, "");

        let (first, rest) = super::split_once(Cow::Owned(input.to_string()), ';');
        assert_eq!(first, "first second");
        assert_eq!(rest, "");
    }

    #[test]
    fn try_split_once() {
        let input = "first;second;";

        let (first, rest) = super::try_split_once(Cow::Borrowed(input), ';').unwrap();
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let (first, rest) = super::try_split_once(Cow::Owned(input.to_string()), ';').unwrap();
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let input = "first second";

        assert!(super::try_split_once(Cow::Borrowed(input), ';').is_err());
        assert!(super::try_split_once(Cow::Owned(input.to_string()), ';').is_err());
    }

    #[test]
    fn split_once_outside_quotes() {
        let input = "first;second;";

        let (first, rest) = super::split_once_outside_quotes(Cow::Borrowed(input), ';');
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let (first, rest) = super::split_once_outside_quotes(Cow::Owned(input.to_string()), ';');
        assert_eq!(first, "first");
        assert_eq!(rest, "second;");

        let input = "first second";

        let (first, rest) = super::split_once_outside_quotes(Cow::Borrowed(input), ';');
        assert_eq!(first, "first second");
        assert_eq!(rest, "");

        let (first, rest) = super::split_once_outside_quotes(Cow::Owned(input.to_string()), ';');
        assert_eq!(first, "first second");
        assert_eq!(rest, "");
    }

    #[test]
    fn pop_front_bytes() {
        let mut input = "   test".to_string();
        super::pop_front_bytes(&mut input, 3);
        assert_eq!(input, "test");
    }
}
