use std::borrow::Cow;

/// lines are split on `\r\n`, but with extension lines that start with a space
///
/// This iterator returns 'unfolded' lines
pub struct LineIter<'src> {
    input: &'src str,
}

impl<'src> LineIter<'src> {
    pub fn new(input: &'src str) -> Self {
        Self { input }
    }
}

impl<'src> Iterator for LineIter<'src> {
    type Item = Cow<'src, str>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.input.split("\r\n");
        // Unwrap: splitn iterator always succeeds once.
        let first = iter.next().unwrap();
        let second = iter.next();
        let Some(second) = second else {
            match first {
                "" => return None,
                line => {
                    // last line
                    self.input = "";
                    return Some(Cow::Borrowed(line));
                }
            }
        };
        if !second.starts_with(" ") {
            // skip first line and `\r\n` - we will be on a char boundary
            self.input = &self.input[first.len() + 2..];
            return Some(Cow::Borrowed(first));
        }

        // we have at least 1 extension line
        let mut output = first.to_owned();
        // first char is space, we are on a char boundary
        output.push_str(&second[1..]);
        let mut len = first.len() + 2 + second.len();
        while let Some(next) = iter.next() {
            if next.starts_with(" ") {
                // first char is space, we are on a char boundary
                output.push_str(&next[1..]);
                len += next.len() + 2;
            } else {
                // `next` is following line
                // add 2 for "\r\n"
                len += 2;
                self.input = &self.input[len..];
                return Some(Cow::Owned(output));
            }
        }
        // we got to the end of the iterator
        self.input = "";
        Some(Cow::Owned(output))
    }
}
