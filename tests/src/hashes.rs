use lcm::Message;

#[test]
fn hashes() {
    // Expected hash values were generated manually from the C
    // implementation of lcm-gen.
    assert_eq!(::member_group::HASH, 0xae7e5fba5eeca11e);
    assert_eq!(::my_constants_t::HASH, 0x000000002468acf0);
    assert_eq!(::my_struct_t::HASH, 0x4fab8e09620e9ec9);
    assert_eq!(::point2d_list_t::HASH, 0x4f85d1e7da2fc594);
    assert_eq!(::temperature_t::HASH, 0xa07fa3d64cbea6ea);
}
