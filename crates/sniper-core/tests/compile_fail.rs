//! Compile-fail locks for the H0.1 trust boundary: the sealed execution
//! currency must be unconstructible outside the issuance boundary, and the
//! deserialization-forge path must not typecheck as a revive.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
