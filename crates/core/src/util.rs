pub fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_null_terminator() {
        let w = to_wide_null("AB");
        assert_eq!(w, vec![0x41, 0x42, 0x00]);
    }

    #[test]
    fn empty_string_is_single_null() {
        assert_eq!(to_wide_null(""), vec![0x00]);
    }
}
