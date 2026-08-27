#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/off.rs");
    t.pass("tests/functional.rs");
    t.pass("tests/negative.rs");
    t.pass("tests/family.rs");
    t.compile_fail("tests/compile-fail/float.rs");
    t.compile_fail("tests/compile-fail/default_family_without_family.rs");
    t.compile_fail("tests/compile-fail/default_family_unknown.rs");
    t.compile_fail("tests/compile-fail/off_with_value.rs");
    t.compile_fail("tests/compile-fail/default_family_any.rs");
}
