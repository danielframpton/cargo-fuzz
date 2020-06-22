pub mod project;

use self::project::*;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;

fn cargo_fuzz() -> Command {
    Command::cargo_bin("cargo-fuzz").unwrap()
}

#[test]
fn help() {
    cargo_fuzz().arg("help").assert().success();
}

#[test]
fn init() {
    let project = project("init").build();
    project.cargo_fuzz().arg("init").assert().success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("fuzz_target_1").is_file());
    project
        .cargo_fuzz()
        .arg("run")
        .arg("fuzz_target_1")
        .arg("--")
        .arg("-runs=1")
        .assert()
        .success();
}

#[test]
fn init_with_target() {
    let project = project("init_with_target").build();
    project
        .cargo_fuzz()
        .arg("init")
        .arg("-t")
        .arg("custom_target_name")
        .assert()
        .success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("custom_target_name").is_file());
    project
        .cargo_fuzz()
        .arg("run")
        .arg("custom_target_name")
        .arg("--")
        .arg("-runs=1")
        .assert()
        .success();
}

#[test]
fn init_twice() {
    let project = project("init_twice").build();

    // First init should succeed and make all the things.
    project.cargo_fuzz().arg("init").assert().success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("fuzz_target_1").is_file());

    // Second init should fail.
    project
        .cargo_fuzz()
        .arg("init")
        .assert()
        .stderr(predicates::str::contains("File exists (os error 17)").and(
            predicates::str::contains(format!(
                "failed to create directory {}",
                project.fuzz_dir().display()
            )),
        ))
        .failure();
}

#[test]
fn init_finds_parent_project() {
    let project = project("init_finds_parent_project").build();
    project
        .cargo_fuzz()
        .current_dir(project.root().join("src"))
        .arg("init")
        .assert()
        .success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("fuzz_target_1").is_file());
}

#[test]
fn add() {
    let project = project("add").with_fuzz().build();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("new_fuzz_target")
        .assert()
        .success();
    assert!(project.fuzz_target_path("new_fuzz_target").is_file());

    assert!(project.fuzz_cargo_toml().is_file());
    let cargo_toml = fs::read_to_string(project.fuzz_cargo_toml()).unwrap();
    let expected_bin_attrs = "test = false\ndoc = false";
    assert!(cargo_toml.contains(expected_bin_attrs));

    project
        .cargo_fuzz()
        .arg("run")
        .arg("new_fuzz_target")
        .arg("--")
        .arg("-runs=1")
        .assert()
        .success();
}

#[test]
fn add_twice() {
    let project = project("add").with_fuzz().build();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("new_fuzz_target")
        .assert()
        .success();
    assert!(project.fuzz_target_path("new_fuzz_target").is_file());
    project
        .cargo_fuzz()
        .arg("add")
        .arg("new_fuzz_target")
        .assert()
        .stderr(
            predicate::str::contains("could not add target")
                .and(predicate::str::contains("File exists (os error 17)")),
        )
        .failure();
}

#[test]
fn list() {
    let project = project("add").with_fuzz().build();

    // Create some targets.
    project.cargo_fuzz().arg("add").arg("c").assert().success();
    project.cargo_fuzz().arg("add").arg("b").assert().success();
    project.cargo_fuzz().arg("add").arg("a").assert().success();

    // Make sure that we can list our targets, and that they're always sorted.
    project
        .cargo_fuzz()
        .arg("list")
        .assert()
        .stdout("a\nb\nc\n")
        .success();
}

#[test]
fn run_no_crash() {
    let project = project("run_no_crash")
        .with_fuzz()
        .fuzz_target(
            "no_crash",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    run_no_crash::pass_fuzzing(data);
                });
            "#,
        )
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("no_crash")
        .arg("--")
        .arg("-runs=1000")
        .assert()
        .stderr(predicate::str::contains("Done 1000 runs"))
        .success();
}

#[test]
fn run_with_crash() {
    let project = project("run_with_crash")
        .with_fuzz()
        .fuzz_target(
            "yes_crash",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    run_with_crash::fail_fuzzing(data);
                });
            "#,
        )
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("yes_crash")
        .arg("--")
        .arg("-runs=1000")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .stderr(
            predicate::str::contains("panicked at 'I'm afraid of number 7'")
                .and(predicate::str::contains("ERROR: libFuzzer: deadly signal"))
                .and(predicate::str::contains("run_with_crash::fail_fuzzing"))
                .and(predicate::str::contains(
                    "────────────────────────────────────────────────────────────────────────────────\n\
                     \n\
                     Failing input:\n\
                     \n\
                     \tfuzz/artifacts/yes_crash/crash-"
                ))
                .and(predicate::str::contains("Output of `std::fmt::Debug`:"))
                .and(predicate::str::contains(
                    "Reproduce with:\n\
                     \n\
                     \tcargo fuzz run yes_crash fuzz/artifacts/yes_crash/crash-"
                ))
                .and(predicate::str::contains(
                    "Minimize test case with:\n\
                     \n\
                     \tcargo fuzz tmin yes_crash fuzz/artifacts/yes_crash/crash-"
                )),
        )
        .failure();
}

#[test]
fn run_without_sanitizer_with_crash() {
    let project = project("run_without_sanitizer_with_crash")
        .with_fuzz()
        .fuzz_target(
            "yes_crash",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    run_without_sanitizer_with_crash::fail_fuzzing(data);
                });
            "#,
        )
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("yes_crash")
        .arg("--")
        .arg("-runs=1000")
        .arg("-sanitizer=none")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .stderr(
            predicate::str::contains("panicked at 'I'm afraid of number 7'")
                .and(predicate::str::contains("ERROR: libFuzzer: deadly signal"))
                .and(predicate::str::contains("run_without_sanitizer_with_crash::fail_fuzzing"))
                .and(predicate::str::contains(
                    "────────────────────────────────────────────────────────────────────────────────\n\
                     \n\
                     Failing input:\n\
                     \n\
                     \tfuzz/artifacts/yes_crash/crash-"
                ))
                .and(predicate::str::contains("Output of `std::fmt::Debug`:"))
                .and(predicate::str::contains(
                    "Reproduce with:\n\
                     \n\
                     \tcargo fuzz run yes_crash fuzz/artifacts/yes_crash/crash-"
                ))
                .and(predicate::str::contains(
                    "Minimize test case with:\n\
                     \n\
                     \tcargo fuzz tmin yes_crash fuzz/artifacts/yes_crash/crash-"
                )),
        )
        .failure();
}

#[test]
fn run_one_input() {
    let corpus = Path::new("fuzz").join("corpus").join("run_one");

    let project = project("run_one_input")
        .with_fuzz()
        .fuzz_target(
            "run_one",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    assert!(data.is_empty());
                });
            "#,
        )
        .file(corpus.join("pass"), "")
        .file(corpus.join("fail"), "not empty")
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("run_one")
        .arg(corpus.join("pass"))
        .assert()
        .stderr(
            predicate::str::contains("Running 1 inputs 1 time(s) each.").and(
                predicate::str::contains("Running: fuzz/corpus/run_one/pass"),
            ),
        )
        .success();
}

#[test]
fn run_a_few_inputs() {
    let corpus = Path::new("fuzz").join("corpus").join("run_few");

    let project = project("run_a_few_inputs")
        .with_fuzz()
        .fuzz_target(
            "run_few",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    assert!(data.len() != 4);
                });
            "#,
        )
        .file(corpus.join("pass-0"), "")
        .file(corpus.join("pass-1"), "1")
        .file(corpus.join("pass-2"), "12")
        .file(corpus.join("pass-3"), "123")
        .file(corpus.join("fail"), "fail")
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("run_few")
        .arg(corpus.join("pass-0"))
        .arg(corpus.join("pass-1"))
        .arg(corpus.join("pass-2"))
        .arg(corpus.join("pass-3"))
        .assert()
        .stderr(
            predicate::str::contains("Running 4 inputs 1 time(s) each.").and(
                predicate::str::contains("Running: fuzz/corpus/run_few/pass"),
            ),
        )
        .success();
}

#[test]
fn run_alt_corpus() {
    let corpus = Path::new("fuzz").join("corpus").join("run_alt");
    let alt_corpus = Path::new("fuzz").join("alt-corpus").join("run_alt");

    let project = project("run_alt_corpus")
        .with_fuzz()
        .fuzz_target(
            "run_alt",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    assert!(data.len() <= 1);
                });
            "#,
        )
        .file(corpus.join("fail"), "fail")
        .file(alt_corpus.join("pass-0"), "0")
        .file(alt_corpus.join("pass-1"), "1")
        .file(alt_corpus.join("pass-2"), "2")
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("run_alt")
        .arg(&alt_corpus)
        .arg("--")
        .arg("-runs=0")
        .assert()
        .stderr(
            predicate::str::contains("3 files found in fuzz/alt-corpus/run_alt")
                .and(predicate::str::contains("fuzz/corpus/run_alt").not())
                // libFuzzer will always test the empty input, so the number of
                // runs performed is always one more than the number of files in
                // the corpus.
                .and(predicate::str::contains("Done 4 runs in")),
        )
        .success();
}

#[test]
fn debug_fmt() {
    let corpus = Path::new("fuzz").join("corpus").join("debugfmt");
    let project = project("debugfmt")
        .with_fuzz()
        .fuzz_target(
            "debugfmt",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;
                use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured, Result};

                #[derive(Debug)]
                pub struct Rgb {
                    r: u8,
                    g: u8,
                    b: u8,
                }

                impl Arbitrary for Rgb {
                    fn arbitrary(raw: &mut Unstructured<'_>) -> Result<Self> {
                        let mut buf = [0; 3];
                        raw.fill_buffer(&mut buf)?;
                        let r = buf[0];
                        let g = buf[1];
                        let b = buf[2];
                        Ok(Rgb { r, g, b })
                    }
                }

                fuzz_target!(|data: Rgb| {
                    let _ = data;
                });
            "#,
        )
        .file(corpus.join("0"), "111")
        .build();

    project
        .cargo_fuzz()
        .arg("fmt")
        .arg("debugfmt")
        .arg("fuzz/corpus/debugfmt/0")
        .assert()
        .stderr(predicates::str::contains(
            "
Rgb {
    r: 49,
    g: 49,
    b: 49,
}",
        ))
        .success();
}

#[test]
fn cmin() {
    let corpus = Path::new("fuzz").join("corpus").join("foo");
    let project = project("cmin")
        .with_fuzz()
        .fuzz_target(
            "foo",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    let _ = data;
                });
            "#,
        )
        .file(corpus.join("0"), "")
        .file(corpus.join("1"), "a")
        .file(corpus.join("2"), "ab")
        .file(corpus.join("3"), "abc")
        .file(corpus.join("4"), "abcd")
        .build();

    let corpus_count = || {
        fs::read_dir(project.root().join("fuzz").join("corpus").join("foo"))
            .unwrap()
            .map(|e| e.unwrap())
            .count()
    };
    assert_eq!(corpus_count(), 5);

    project
        .cargo_fuzz()
        .arg("cmin")
        .arg("foo")
        .assert()
        .success();
    assert_eq!(corpus_count(), 1);
}

#[test]
fn tmin() {
    let corpus = Path::new("fuzz").join("corpus").join("i_hate_zed");
    let test_case = corpus.join("test-case");
    let project = project("tmin")
        .with_fuzz()
        .fuzz_target(
            "i_hate_zed",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    let s = String::from_utf8_lossy(data);
                    if s.contains('z') {
                        panic!("nooooooooo");
                    }
                });
            "#,
        )
        .file(&test_case, "pack my box with five dozen liquor jugs")
        .build();
    let test_case = project.root().join(test_case);
    project
        .cargo_fuzz()
        .arg("tmin")
        .arg("i_hate_zed")
        .arg(&test_case)
        .assert()
        .stderr(
            predicates::str::contains("CRASH_MIN: minimizing crash input: ")
                .and(predicate::str::contains("(1 bytes) caused a crash"))
                .and(predicate::str::contains(
                    "────────────────────────────────────────────────────────────────────────────────\n\
                     \n\
                     Minimized artifact:\n\
                     \n\
                     \tfuzz/artifacts/i_hate_zed/minimized-from-"))
                .and(predicate::str::contains(
                    "Reproduce with:\n\
                     \n\
                     \tcargo fuzz run i_hate_zed fuzz/artifacts/i_hate_zed/minimized-from-"
                )),
        )
        .success();
}

#[test]
fn build_all() {
    let project = project("build_all").with_fuzz().build();

    // Create some targets.
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_all_a")
        .assert()
        .success();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_all_b")
        .assert()
        .success();

    // Build to ensure that the build directory is created and
    // `fuzz_build_dir()` won't panic.
    project.cargo_fuzz().arg("build").assert().success();

    let build_dir = project.fuzz_build_dir().join("release");

    let a_bin = build_dir.join("build_all_a");
    let b_bin = build_dir.join("build_all_b");

    // Remove the files we just built.
    fs::remove_file(&a_bin).unwrap();
    fs::remove_file(&b_bin).unwrap();

    assert!(!a_bin.is_file());
    assert!(!b_bin.is_file());

    // Test that building all fuzz targets does in fact recreate the files.
    project.cargo_fuzz().arg("build").assert().success();

    assert!(a_bin.is_file());
    assert!(b_bin.is_file());
}

#[test]
fn build_one() {
    let project = project("build_one").with_fuzz().build();

    // Create some targets.
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_one_a")
        .assert()
        .success();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_one_b")
        .assert()
        .success();

    // Build to ensure that the build directory is created and
    // `fuzz_build_dir()` won't panic.
    project.cargo_fuzz().arg("build").assert().success();

    let build_dir = project.fuzz_build_dir().join("release");
    let a_bin = build_dir.join("build_one_a");
    let b_bin = build_dir.join("build_one_b");

    // Remove the files we just built.
    fs::remove_file(&a_bin).unwrap();
    fs::remove_file(&b_bin).unwrap();

    assert!(!a_bin.is_file());
    assert!(!b_bin.is_file());

    // Test that we can build one and not the other.
    project
        .cargo_fuzz()
        .arg("build")
        .arg("build_one_a")
        .assert()
        .success();

    assert!(a_bin.is_file());
    assert!(!b_bin.is_file());
}
