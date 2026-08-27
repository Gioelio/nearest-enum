use nearest_enum::Nearest;

#[derive(Nearest)]
enum E {
    #[nearest(off, 5)]
    A,
}

fn main() {}
