/*
macro_rules! bail {
    ($input:expr) => {
        return Err(nom::Err::Error($input.into()))
    };
}

macro_rules! itry {
    ($input:expr) => {
        match $input {
            Ok(v) => v,
            Err(e) => return Err(nom::Err::Error(e.into())),
        }
    };
}
*/
