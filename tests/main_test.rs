use assert_cmd::prelude::*;
use itertools::Itertools;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn no_arg_specified() {
    let mut cmd = Command::cargo_bin("tx_proc").expect("could not build main binary");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid argument count"));
}

#[test]
fn can_not_open_file() {
    let mut cmd = Command::cargo_bin("tx_proc").expect("could not build main binary");

    cmd.arg("invalid_file.csv")
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to open file"));
}

#[test]
fn tests_from_data_dir() {
    let test_cases = [
        (
            "tests/data/test_case_from_instructions.csv",
            "client,available,held,total,locked\n2,2,0,2,false\n1,1.5,0,1.5,false\n",
        ),
        ("tests/data/no_headers.csv", ""),
        (
            "tests/data/invalid_records.csv",
            "client,available,held,total,locked\n2,0,0,0,false\n1,1.0000,1.0005,2.0005,false\n",
        ),
    ];

    for (file, expected_stdout) in test_cases {
        let mut cmd = Command::cargo_bin("tx_proc").expect("could not build main binary");

        cmd.arg(file)
            .assert()
            .success()
            .stdout(predicate::function(|stdout_str: &str| {
                predicate::eq(expected_stdout.split('\n').sorted().as_slice())
                    .eval(stdout_str.split('\n').sorted().as_slice())
            }));
    }
}
