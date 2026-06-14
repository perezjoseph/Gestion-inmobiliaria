use subtle::ConstantTimeEq;

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let len_matches: subtle::Choice = (a.len() as u64).ct_eq(&(b.len() as u64));
    let cmp_a = if a.is_empty() { &[0u8][..] } else { a };
    let cmp_b = if b.is_empty() { &[0u8][..] } else { b };
    let min_len = cmp_a.len().min(cmp_b.len());
    let content_matches = cmp_a[..min_len].ct_eq(&cmp_b[..min_len]);
    (len_matches & content_matches).into()
}
