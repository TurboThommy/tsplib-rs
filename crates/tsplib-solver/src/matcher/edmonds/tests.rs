use super::*;

#[test]
fn edge_key_is_canonical() {
    assert_eq!(edge_key(3, 7), (3, 7));
    assert_eq!(edge_key(7, 3), (3, 7));
}
