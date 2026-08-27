use nearest_enum::Nearest;

// negative values are supported in the input but not as nearest-enum values 

#[derive(Clone, PartialEq, Debug, Copy, Nearest)]
#[nearest(ty = "i32")]
pub enum UnsignedEnum {
    #[nearest(1)]
    One = 0x1,
    #[nearest(2)]
    Two = 0x2,
}

#[test]
fn negative_values_input() {
    assert_eq!(UnsignedEnum::nearest(-1), UnsignedEnum::One);
}

fn main() {}
