use nearest_enum::Nearest;

// Off should be matched only if value is 0

#[derive(Clone, PartialEq, Debug, Copy, Nearest)]
pub enum Odr {
    #[nearest(off)]
    Off = 0x0,
    #[nearest(5)]
    SomeValue = 0x1,
}

#[test]
fn off_should_match_only_0_values() {

    assert_eq!(Odr::ceil(1), Odr::SomeValue);
    assert_eq!(Odr::nearest(1), Odr::SomeValue);

    // those should match with `Off` variant
    assert_eq!(Odr::nearest(0), Odr::Off);
    assert_eq!(Odr::ceil(0), Odr::Off);
}

fn main() {}
