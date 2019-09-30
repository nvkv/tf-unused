use std::fs;
use std::path::Path;
use std::process;

use clap::{App, Arg};
use glob::glob;
use itertools::Itertools;
use regex::Regex;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref DEFINTION_REGEX: Regex = Regex::new(r#"variable\s+"([\w_]+)"\s+\{"#).unwrap();
    static ref VALUE_REGEX: Regex = Regex::new(r#"([\w_]+)\s+=\s+(.*)"#).unwrap();
    static ref USE_REGEX: Regex = Regex::new(r#"var\.([\w_]+)"#).unwrap();
}

#[derive(Debug, Clone, Copy)]
enum EntryType {
    Definition,
    Use,
    Value,
}

impl EntryType {
    fn regex(&self) -> &Regex {
        match self {
            Self::Definition => &DEFINTION_REGEX,
            Self::Use => &USE_REGEX,
            Self::Value => &VALUE_REGEX,
        }
    }
}

#[derive(Debug)]
struct Variable {
    entry_type: EntryType,
    name: String,
    at: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FileType {
    Source,
    Vars,
}

impl FileType {
    fn ext(self) -> String {
        match self {
            FileType::Source => "tf".to_string(),
            FileType::Vars => "tfvars".to_string(),
        }
    }
}

#[derive(Debug)]
struct File {
    file_type: FileType,
    path: String,
    contents: String,
}

impl File {
    fn files_in(dir: &Path) -> Result<Vec<Result<File, String>>, String> {
        let mut files = Self::get_files(FileType::Source, dir)?;
        files.extend(Self::get_files(FileType::Vars, dir)?);
        Ok(files)
    }

    fn get_files(file_type: FileType, dir: &Path) -> Result<Vec<Result<File, String>>, String> {
        let path_buf = dir.join(format!("*.{}", file_type.ext()));

        let g = match path_buf.as_path().to_str() {
            Some(glob_path) => glob_path.to_string(),
            None => return Err("Failed to construct glob expression".to_string()),
        };

        let file_paths = match glob(&g) {
            Ok(files) => files.filter_map(Result::ok),
            Err(err) => return Err(err.to_string()),
        };

        let files = file_paths
            .map(|path| {
                let path_str = path
                    .clone()
                    .into_os_string()
                    .into_string()
                    .unwrap_or_else(|_| "unknown path".to_string());
                if let Ok(contents) = fs::read_to_string(path) {
                    let file = File {
                        file_type,
                        path: path_str.to_string(),
                        contents,
                    };
                    Ok(file)
                } else {
                    Err(format!("Error: could not read file {}", path_str))
                }
            })
            .collect();
        Ok(files)
    }

    fn get_var_entries(&self, entry_type: EntryType) -> Vec<Variable> {
        entry_type
            .regex()
            .captures_iter(&self.contents)
            .filter(|cap| cap.len() > 1)
            .map(|cap| Variable {
                name: cap[1].to_string(),
                at: self.path.clone(),
                entry_type,
            })
            .collect()
    }
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

fn report_unsued(unused: &[&Variable]) {
    let by_file = unused.iter().group_by(|v| &v.at);
    for (file, vars) in &by_file {
        println!("In {}:", file);
        for v in vars {
            let prefix = match v.entry_type {
                EntryType::Definition => "Unused definition",
                EntryType::Value => "Unused value for",
                _ => "Shouldn't be there",
            };
            println!(" * {} {}", prefix, v.name);
        }
        println!();
    }
}

fn main() {
    let matches = App::new("tf-unused")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Find unused variables in terraform files")
        .author("Semyon Novikov <nsa@bedlam.io>")
        .arg(
            Arg::with_name("INPUT")
                .help("Directory to validate")
                .required(false)
                .index(1),
        )
        .get_matches();

    let working_dir = matches.value_of("INPUT").unwrap_or(".");
    let wd_path = validate_and_get_path(working_dir).unwrap_or_else(|e| {
        println!("{}", e);
        process::exit(1)
    });

    let files: Vec<_> = File::files_in(&wd_path)
        .unwrap_or_else(|e| {
            println!("{}", e);
            process::exit(1);
        })
        .into_iter()
        .filter_map(|f| match f {
            Ok(f) => Some(f),
            Err(e) => {
                println!("{}", e);
                None
            }
        })
        .collect();

    let definitions: Vec<_> = files
        .iter()
        .filter(|f| f.file_type == FileType::Source)
        .map(|f| f.get_var_entries(EntryType::Definition))
        .flatten()
        .collect();

    let uses: Vec<_> = files
        .iter()
        .filter(|f| f.file_type == FileType::Source)
        .map(|f| f.get_var_entries(EntryType::Use))
        .flatten()
        .collect();

    let values: Vec<_> = files
        .iter()
        .filter(|f| f.file_type == FileType::Vars)
        .map(|f| f.get_var_entries(EntryType::Value))
        .flatten()
        .collect();

    let unused: Vec<_> = definitions
        .iter()
        .filter(|def| uses.iter().find(|inst| inst.name == def.name).is_none())
        .collect();

    let unused_vals: Vec<_> = values
        .iter()
        .filter(|val| {
            definitions
                .iter()
                .find(|def| def.name == val.name)
                .is_none()
        })
        .collect();

    report_unsued(&unused);
    report_unsued(&unused_vals);

    if !&unused.is_empty() || !&unused_vals.is_empty() {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tfvars_pattern() {
        let re = &VALUE_REGEX;
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
        let re = &DEFINTION_REGEX;
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
        let re = &USE_REGEX;

        assert!(re.is_match(test_string));

        for cap in re.captures_iter(test_string) {
            assert!(&cap[1] == "very_important_variable");
        }
    }
}
