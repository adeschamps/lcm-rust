use lcm::Message;

#[test]
fn hashes() {
    // Expected hash values were generated manually from the C
    // implementation of lcm-gen.
    assert_eq!(::MemberGroup::HASH, 0xae7e5fba5eeca11e);
    assert_eq!(::MyConstants::HASH, 0x000000002468acf0);
    assert_eq!(::MyStruct::HASH, 0x4fab8e09620e9ec9);
    assert_eq!(::Point2dList::HASH, 0x4f85d1e7da2fc594);
    assert_eq!(::Temperature::HASH, 0xa07fa3d64cbea6ea);
}
