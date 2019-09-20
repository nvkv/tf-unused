use std::fs;
use std::path::Path;
use std::process;

use clap::{App, Arg};
use glob::glob;
use regex::Regex;

const VAR_DECLARATION_PATTERN: &str = r#"variable\s+"([\w_]+)"\s+\{"#;
const VAR_USE_PATTERN: &str = r#"var\.([\w_]+)"#;
const APP_VERSION: &str = "2019-09-1";

#[derive(Debug)]
struct Variable {
    name: String,
    defined_in: String,
}

#[derive(Debug)]
struct VarUse {
    name: String,
    found_in: String,
}

fn find_var_definitions(file: &Path, text: &str) -> Vec<Variable> {
    let re =
        Regex::new(VAR_DECLARATION_PATTERN).expect("Failed to compile variable declaration regex");
    re.captures_iter(text)
        .filter(|cap| cap.len() > 1)
        .map(|cap| Variable {
            name: cap[1].to_string(),
            defined_in: file.to_str().unwrap_or("unknown").to_string(),
        })
        .collect()
}

fn find_var_usages(file: &Path, text: &str) -> Vec<VarUse> {
    let re = Regex::new(VAR_USE_PATTERN).expect("Failed to compile variable usage regex");
    re.captures_iter(text)
        .filter(|cap| cap.len() > 1)
        .map(|cap| VarUse {
            name: cap[1].to_string(),
            found_in: file.to_str().unwrap_or("unknown").to_string(),
        })
        .collect()
}

fn main() {
    let matches = App::new("tf-unused")
        .version(APP_VERSION)
        .about("Find unused variables in terraform files")
        .author("Semyon Novikov <nsa@bedlam.io>")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets directory to check")
                .required(false)
                .index(1),
        )
        .get_matches();

    let working_dir = matches.value_of("INPUT").unwrap_or(".");
    let wd_path = Path::new(working_dir);

    if !wd_path.exists() {
        println!("Path {} does not exists", working_dir);
        process::exit(1);
    }

    if !wd_path.is_dir() {
        println!("{} is not a directory", working_dir);
        process::exit(1);
    }

    let path_buf = wd_path.join("*.tf");
    let tf_glob = path_buf
        .as_path()
        .to_str()
        .expect("Failed to construct glob expression");

    let mut definitions: Vec<Variable> = Vec::new();
    let mut usages: Vec<VarUse> = Vec::new();

    let glob_results = glob(tf_glob).expect("Failed to read glob pattern");
    for tf_file in glob_results {
        if let Ok(tf_file) = tf_file {
            let p = Path::new(working_dir).join(tf_file);
            if let Ok(content) = fs::read_to_string(&p) {
                definitions.append(&mut find_var_definitions(&p, &content));
                usages.append(&mut find_var_usages(&p, &content));
            } else {
                println!("Cant open file, skipping: {:?}", p);
            }
        }
    }

    let res = definitions
        .iter()
        .filter(|var| usages.iter().find(|usage| var.name == usage.name).is_none());

    for unused in res {
        println!(
            "Unused variable \"{}\" defined in {}",
            unused.name, unused.defined_in
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_pattern() {
        let re = Regex::new(VAR_DECLARATION_PATTERN).unwrap();
        let test_string = r#"
        variable "surprisingly_important_variable" {
        	default = 42
        }
        "#;
        assert!(re.is_match(test_string));
        for cap in re.captures_iter(test_string) {
            assert!(&cap[1] == "surprisingly_important_variable")
        }
    }

    #[test]
    fn test_variable_use_pattern() {
        let test_string = r#"something = "${foo(var.very_important_variable)}""#;
        let re = Regex::new(VAR_USE_PATTERN).unwrap();

        assert!(re.is_match(test_string));

        for cap in re.captures_iter(test_string) {
            assert!(&cap[1] == "very_important_variable");
        }
    }
}
