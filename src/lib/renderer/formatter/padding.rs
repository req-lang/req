const SPACES_PER_LEVEL: usize = 2;
const MAX_DEPTH: usize = 128;
const INDENT: &str =
    unsafe { std::str::from_utf8_unchecked(&[b' '; SPACES_PER_LEVEL * MAX_DEPTH]) };

#[inline]
pub fn pad(level: usize) -> &'static str {
    &INDENT[..level * 2]
}
