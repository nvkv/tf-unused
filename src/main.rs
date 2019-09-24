use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use clap::{App, Arg};
use glob::glob;
use itertools::Itertools;
use regex::Regex;

#[macro_use]
extern crate lazy_static;

const TFVAR_PATTERN: &str = r#"([\w_]+)\s+=\s+(.*)"#;
const VAR_DECLARATION_PATTERN: &str = r#"variable\s+"([\w_]+)"\s+\{"#;
const VAR_USE_PATTERN: &str = r#"var\.([\w_]+)"#;
const APP_VERSION: &str = "2019-09-1";

enum Filetype {
    TfFile,
    TfVarsFile,
}

impl Filetype {
    fn ext(&self) -> String {
        match self {
            Filetype::TfFile => "tf".to_string(),
            Filetype::TfVarsFile => "tfvars".to_string(),
        }
    }

    fn files_in(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let path_buf = dir.join(format!("*.{}", self.ext()));

        let g = match path_buf.as_path().to_str() {
            Some(glob_path) => Ok(glob_path.to_string()),
            None => return Err("Failed to construct glob expression".to_string()),
        };

        let g = match g {
            Ok(g) => g,
            Err(e) => return Err(e),
        };

        match glob(&g) {
            Ok(files) => Ok(files.filter(|f| f.is_ok()).map(|f| f.unwrap()).collect()),
            Err(err) => Err(err.to_string()),
        }
    }
}

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

#[derive(Debug)]
struct TfVar {
    name: String,
    defined_in: String,
}

fn find_tfvars(file: &Path, text: &str) -> Vec<TfVar> {
    lazy_static! {
        static ref TFVARS_REGEX: Regex =
            Regex::new(TFVAR_PATTERN).expect("Failed to compile variable declaration regex");
    }
    TFVARS_REGEX
        .captures_iter(text)
        .filter(|cap| cap.len() > 1)
        .map(|cap| TfVar {
            name: cap[1].to_string(),
            defined_in: file.to_str().unwrap_or("unknown").to_string(),
        })
        .collect()
}

fn find_var_definitions(file: &Path, text: &str) -> Vec<Variable> {
    lazy_static! {
        static ref VAR_DECLARATION_REGEX: Regex = Regex::new(VAR_DECLARATION_PATTERN)
            .expect("Failed to compile variable declaration regex");
    }
    VAR_DECLARATION_REGEX
        .captures_iter(text)
        .filter(|cap| cap.len() > 1)
        .map(|cap| Variable {
            name: cap[1].to_string(),
            defined_in: file.to_str().unwrap_or("unknown").to_string(),
        })
        .collect()
}

fn find_var_usages(file: &Path, text: &str) -> Vec<VarUse> {
    lazy_static! {
        static ref VAR_USE_REGEX: Regex =
            Regex::new(VAR_USE_PATTERN).expect("Failed to compile variable usage regex");
    }
    VAR_USE_REGEX
        .captures_iter(text)
        .filter(|cap| cap.len() > 1)
        .map(|cap| VarUse {
            name: cap[1].to_string(),
            found_in: file.to_str().unwrap_or("unknown").to_string(),
        })
        .collect()
}

fn validate_and_get_path(wd: &str) -> Result<Box<&Path>, String> {
    let wd_path = Path::new(wd);
    if !wd_path.exists() {
        return Err(format!("Path {} does not exists", wd));
    }

    if !wd_path.is_dir() {
        return Err(format!("{} is not a directory", wd));
    }

    Ok(Box::new(wd_path))
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
    let wd_path = validate_and_get_path(working_dir).unwrap_or_else(|e| {
        println!("{}", e);
        process::exit(1)
    });

    let mut definitions: Vec<Variable> = Vec::new();
    let mut usages: Vec<VarUse> = Vec::new();
    let files = Filetype::TfFile.files_in(&wd_path).unwrap_or_else(|e| {
        println!("{}", e);
        process::exit(1);
    });

    for tf_file in files {
        if let Ok(content) = fs::read_to_string(&tf_file) {
            definitions.append(&mut find_var_definitions(&tf_file, &content));
            usages.append(&mut find_var_usages(&tf_file, &content));
        } else {
            println!("Cant open file, skipping: {:?}", &tf_file);
        }
    }

    let unused_vars: Vec<&Variable> = definitions
        .iter()
        .filter(|var| usages.iter().find(|usage| var.name == usage.name).is_none())
        .collect();

    for unused in unused_vars.iter() {
        println!(
            "Unused variable \"{}\" defined in {}",
            unused.name, unused.defined_in
        );
    }

    let mut tf_vars: Vec<TfVar> = Vec::new();
    let tf_var_files = Filetype::TfVarsFile.files_in(&wd_path).unwrap_or_else(|e| {
        println!("{}", e);
        process::exit(1);
    });

    for tfvar_file in tf_var_files {
        if let Ok(content) = fs::read_to_string(&tfvar_file) {
            tf_vars.append(&mut find_tfvars(&tfvar_file, &content));
        } else {
            println!("Cant open file, skipping: {:?}", tfvar_file);
        }
    }

    let unused_tfvars = tf_vars
        .iter()
        .filter(|tfvar| definitions.iter().find(|d| d.name == tfvar.name).is_none())
        .group_by(|tfvar| &tfvar.defined_in);

    for (file, vars) in &unused_tfvars {
        println!("{}:", file);
        for v in vars {
            println!(" - {}", v.name);
        }
        println!();
    }

    if !unused_vars.is_empty() || unused_tfvars.into_iter().count() > 0 {
        process::exit(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tfvars_pattern() {
        let re = Regex::new(TFVAR_PATTERN).unwrap();
        let test_string = r#"
        one = 42
        two = "booooring"
    two_and_a_half = "now it's getting interesting"

          three_times_fourty_two = true
        "#;
        assert!(re.is_match(test_string));
        for cap in re.captures_iter(test_string) {
            //assert!(&cap[1] == "surprisingly_important_variable")
            println!("MATCH: {} -> {}", &cap[1], &cap[2]);
        }
    }

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
