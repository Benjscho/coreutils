// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore STRINGSTRING INTEGERINTEGER FILEFILE

#![no_main]
use libfuzzer_sys::fuzz_target;
use uu_test::uumain;

use rand::seq::SliceRandom;
use rand::Rng;
use std::ffi::OsString;

mod fuzz_common;
use crate::fuzz_common::{generate_and_run_uumain, run_gnu_cmd};

#[derive(PartialEq, Debug, Clone)]
enum ArgType {
    STRING,
    STRINGSTRING,
    INTEGER,
    INTEGERINTEGER,
    FILE,
    FILEFILE,
    // Add any other types as needed
}

static CMD_PATH: &str = "test";

fn generate_random_string(max_length: usize) -> String {
    let mut rng = rand::thread_rng();
    let valid_utf8: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        .chars()
        .collect();
    let invalid_utf8 = [0xC3, 0x28]; // Invalid UTF-8 sequence
    let mut result = String::new();

    for _ in 0..rng.gen_range(1..=max_length) {
        if rng.gen_bool(0.9) {
            let ch = valid_utf8.choose(&mut rng).unwrap();
            result.push(*ch);
        } else {
            let ch = invalid_utf8.choose(&mut rng).unwrap();
            if let Some(c) = char::from_u32(*ch as u32) {
                result.push(c);
            }
        }
    }

    result
}

#[derive(Debug, Clone)]
struct TestArg {
    arg: String,
    arg_type: ArgType,
}

fn generate_random_path(rng: &mut dyn rand::RngCore) -> &'static str {
    match rng.gen_range(0..=3) {
        0 => "/dev/null",
        1 => "/dev/random",
        2 => "/tmp",
        _ => "/dev/urandom",
    }
}

fn generate_test_args() -> Vec<TestArg> {
    vec![
        TestArg {
            arg: "-z".to_string(),
            arg_type: ArgType::STRING,
        },
        TestArg {
            arg: "-n".to_string(),
            arg_type: ArgType::STRING,
        },
        TestArg {
            arg: "=".to_string(),
            arg_type: ArgType::STRINGSTRING,
        },
        TestArg {
            arg: "!=".to_string(),
            arg_type: ArgType::STRINGSTRING,
        },
        TestArg {
            arg: "-eq".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-ne".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-gt".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-ge".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-lt".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-le".to_string(),
            arg_type: ArgType::INTEGERINTEGER,
        },
        TestArg {
            arg: "-f".to_string(),
            arg_type: ArgType::FILE,
        },
        TestArg {
            arg: "-d".to_string(),
            arg_type: ArgType::FILE,
        },
        TestArg {
            arg: "-e".to_string(),
            arg_type: ArgType::FILE,
        },
        TestArg {
            arg: "-ef".to_string(),
            arg_type: ArgType::FILEFILE,
        },
        TestArg {
            arg: "-nt".to_string(),
            arg_type: ArgType::FILEFILE,
        },
    ]
}

fn generate_test_arg() -> String {
    let mut rng = rand::thread_rng();
    let test_args = generate_test_args();
    let mut arg = String::new();

    let choice = rng.gen_range(0..=5);

    match choice {
        0 => {
            arg.push_str(&rng.gen_range(-100..=100).to_string());
        }
        1..=3 => {
            let test_arg = test_args
                .choose(&mut rng)
                .expect("Failed to choose a random test argument");
            if test_arg.arg_type == ArgType::INTEGER {
                arg.push_str(&format!(
                    "{} {} {}",
                    &rng.gen_range(-100..=100).to_string(),
                    test_arg.arg,
                    &rng.gen_range(-100..=100).to_string()
                ));
            } else if test_arg.arg_type == ArgType::STRINGSTRING {
                let random_str = generate_random_string(rng.gen_range(1..=10));
                let random_str2 = generate_random_string(rng.gen_range(1..=10));

                arg.push_str(&format!(
                    "{} {} {}",
                    &random_str, test_arg.arg, &random_str2
                ));
            } else if test_arg.arg_type == ArgType::STRING {
                let random_str = generate_random_string(rng.gen_range(1..=10));
                arg.push_str(&format!("{} {}", test_arg.arg, &random_str));
            } else if test_arg.arg_type == ArgType::FILEFILE {
                let path = generate_random_path(&mut rng);
                let path2 = generate_random_path(&mut rng);
                arg.push_str(&format!("{} {} {}", path, test_arg.arg, path2));
            } else if test_arg.arg_type == ArgType::FILE {
                let path = generate_random_path(&mut rng);
                arg.push_str(&format!("{} {}", test_arg.arg, path));
            }
        }
        4 => {
            let random_str = generate_random_string(rng.gen_range(1..=10));
            arg.push_str(&random_str);
        }
        _ => {
            let path = generate_random_path(&mut rng);

            let file_test_args: Vec<TestArg> = test_args
                .iter()
                .filter(|ta| ta.arg_type == ArgType::FILE)
                .cloned()
                .collect();

            if let Some(test_arg) = file_test_args.choose(&mut rng) {
                arg.push_str(&format!("{}{}", test_arg.arg, path));
            }
        }
    }

    arg
}

fuzz_target!(|_data: &[u8]| {
    let mut rng = rand::thread_rng();
    let max_args = rng.gen_range(1..=6);
    let mut args = vec![OsString::from("test")];

    for _ in 0..max_args {
        args.push(OsString::from(generate_test_arg()));
    }

    let (rust_output, uumain_exit_status) = generate_and_run_uumain(&args, uumain);

    // Run GNU test with the provided arguments and compare the output
    match run_gnu_cmd(CMD_PATH, &args[1..], false) {
        Ok((gnu_output, gnu_exit_status)) => {
            let gnu_output = gnu_output.trim().to_owned();
            println!("gnu_exit_status {}", gnu_exit_status);
            println!("uumain_exit_status {}", uumain_exit_status);
            if rust_output != gnu_output || uumain_exit_status != gnu_exit_status {
                println!("Discrepancy detected!");
                println!("Test: {:?}", &args[1..]);
                println!("My output: {}", rust_output);
                println!("GNU output: {}", gnu_output);
                println!("My exit status: {}", uumain_exit_status);
                println!("GNU exit status: {}", gnu_exit_status);
                panic!();
            } else {
                println!(
                    "Outputs and exit statuses matched for expression {:?}",
                    &args[1..]
                );
            }
        }
        Err(_) => {
            println!("GNU test execution failed for expression {:?}", &args[1..]);
        }
    }
});
