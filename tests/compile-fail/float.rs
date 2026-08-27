use nearest_enum::Nearest;

// float values not allowed, should fails with custom syn error

#[derive(Nearest)]
pub enum BadEnum {
    #[nearest(1.5)]
    Invalid = 0x1,
}

fn main() {}
