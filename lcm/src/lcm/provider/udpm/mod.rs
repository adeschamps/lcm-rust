use std::marker::PhantomData;

pub struct UdpmProvider<'a> {
    _pd: PhantomData<&'a ()>,
}
