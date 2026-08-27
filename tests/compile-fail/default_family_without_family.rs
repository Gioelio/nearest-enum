use nearest_enum::Nearest;

#[derive(Nearest)]
#[nearest(default_family = "a")]
enum E {
    #[nearest(0)]
    A,
}


fn main() {}
