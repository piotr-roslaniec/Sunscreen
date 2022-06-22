use sunscreen::{
    types::{bfv::Signed, Cipher},
    *,
};

#[test]
fn compiling_multiple_programs_yields_same_params() {
    #[fhe_program(scheme = "bfv")]
    fn add(a: Cipher<Signed>, b: Cipher<Signed>) -> Cipher<Signed> {
        a + b
    }

    #[fhe_program(scheme = "bfv")]
    fn mul(a: Cipher<Signed>, b: Cipher<Signed>) -> Cipher<Signed> {
        a * b
    }

    let app = Compiler::new()
        .fhe_program(add)
        .fhe_program(mul)
        .compile()
        .unwrap();

    assert_eq!(*app.params(), app.get_program(add).unwrap().metadata.params);
    assert_eq!(*app.params(), app.get_program(mul).unwrap().metadata.params);
}

#[test]
fn can_reference_program_strongly_or_stringly() {
    #[fhe_program(scheme = "bfv")]
    fn add(a: Cipher<Signed>, b: Cipher<Signed>) -> Cipher<Signed> {
        a + b
    }

    #[fhe_program(scheme = "bfv")]
    fn mul(a: Cipher<Signed>, b: Cipher<Signed>) -> Cipher<Signed> {
        a * b
    }

    let app = Compiler::new()
        .fhe_program(add)
        .fhe_program(mul)
        .compile()
        .unwrap();

    assert_eq!(mul.name(), "mul");
    assert_eq!(add.name(), "add");

    assert_eq!(app.get_program(mul).is_some(), true);
    assert_eq!(app.get_program("mul").is_some(), true);
    assert_eq!(app.get_program(add).is_some(), true);
    assert_eq!(app.get_program("add").is_some(), true);
}