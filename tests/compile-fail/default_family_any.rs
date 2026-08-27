use nearest_enum::Nearest;

#[derive(Nearest)]
#[nearest(default_family = "any")]
enum E {
    #[nearest(0, family = "x")]
    A,
}

fn main() {}
