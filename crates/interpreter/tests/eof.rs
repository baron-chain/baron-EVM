use bcevm_interpreter::analysis::{validate_raw_eof, EofError};
use bcevm_primitives::{Bytes, Eof};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};
use walkdir::{DirEntry, WalkDir};

#[test]
fn eof_run_all_tests() {
    run_test(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/EOFTests"))
}

#[test]
fn eof_validation() {
    run_test(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/EOFTests/eof_validation"))
}

#[test]
fn eof_validation_eip5450() {
    run_test(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/EOFTests/EIP5450"))
}

#[test]
fn eof_validation_eip3670() {
    run_test(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/EOFTests/EIP3670"))
}

#[test]
fn eof_validation_eip4750() {
    let inst = Instant::now();
    run_test(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/EOFTests/EIP4750"));
    println!("Elapsed:{:?}", inst.elapsed())
}

#[test]
fn eof_validation_eip3540() {
    run_test(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/EOFTests/EIP3540"))
}

#[test]
fn eof_validation_eip4200() {
    run_test(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/EOFTests/EIP4200"))
}

fn run_test(path: &Path) {
    let test_files = find_all_json_tests(path);
    let mut test_sum = 0;
    let mut passed_tests = 0;

    #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    enum ErrorType {
        FalsePositive,
        Error(EofError),
    }
    let mut types_of_error: BTreeMap<ErrorType, usize> = BTreeMap::new();
    
    for test_file in test_files {
        let s = std::fs::read_to_string(test_file).unwrap();
        let suite: TestSuite = serde_json::from_str(&s).unwrap();
        for (name, test_unit) in suite.0 {
            for (vector_name, test_vector) in test_unit.vectors {
                test_sum += 1;
                let res = validate_raw_eof(test_vector.code.clone());
                if res.is_ok() != test_vector.results.prague.result {
                    let eof = Eof::decode(test_vector.code.clone());
                    println!(
                        "\nTest failed: {} - {}\nresult:{:?}\nbcevm err_result:{:#?}\nbytes:{:?}\n,eof:{eof:#?}",
                        name, vector_name, test_vector.results.prague, res.as_ref().err(), test_vector.code
                    );
                    *types_of_error
                        .entry(res.err().map(ErrorType::Error).unwrap_or(ErrorType::FalsePositive))
                        .or_default() += 1;
                } else {
                    passed_tests += 1;
                }
            }
        }
    }
    println!("Types of error: {:#?}", types_of_error);
    println!("Passed tests: {}/{}", passed_tests, test_sum);
}

fn find_all_json_tests(path: &Path) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .map(DirEntry::into_path)
        .collect()
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct TestSuite(BTreeMap<String, TestUnit>);

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestUnit {
    #[serde(default, rename = "_info")]
    info: Option<serde_json::Value>,
    vectors: BTreeMap<String, TestVector>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestVector {
    code: Bytes,
    results: PragueResult,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PragueResult {
    #[serde(rename = "Prague")]
    prague: Result,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Result {
    result: bool,
    exception: Option<String>,
}
