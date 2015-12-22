extern crate crypto;
extern crate regex;
extern crate rustc_serialize;

use crypto::md5::Md5;
use crypto::digest::Digest;

use rustc_serialize::{json, Decodable, Encodable};

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, DirEntry, File};
use std::io::prelude::*;
use std::path::Path;
use std::process::{self, Command};

trait Stringable {
    fn to_string(self) -> String;
}

impl Stringable for DirEntry {
    fn to_string(self) -> String {
        self.path().as_path().to_str().unwrap().to_owned()
    }
}

impl Stringable for OsString {
    fn to_string(self) -> String {
        self.as_os_str().to_str().unwrap().to_owned()
    }
}

const INDEX_PATH: &'static str = "blaze/file_index.json";
const CONFIG_PATH: &'static str = "blaze/lint_config.json";

// Map from path to hash
#[derive(RustcDecodable, RustcEncodable, Default)]
pub struct Index {
    files: HashMap<String, String>,
}

#[derive(RustcDecodable)]
pub struct Lint {
    name: String,
    command: String,
    args: Vec<String>,
}

impl Lint {
    fn command(&self, paths: &[String]) -> Command {
        let mut c = Command::new(&self.command);
        for arg in &self.args {
            c.arg(arg);
        }
        for path in paths {
            c.arg(path);
        }
        c
    }

    // runs this lint on all given files, return all the clean files
    fn resolve(&self, files: Vec<String>) -> Vec<String> {
        let output = &self.command(&files)
                          .output()
                          .unwrap_or_else(|_| {
                              println!("{} not installed", self.name);
                              process::exit(1);
                          });
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout != "" {
            println!("{}", stdout);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr != "" {
            println!("{}", stderr);
        }
        files.into_iter()
             .filter(|file| {
                 let re = regex::Regex::new(&format!(".*{}.*", file)).unwrap();
                 !re.is_match(&stdout) && !re.is_match(&stderr)
             })
             .collect()
    }
}

#[derive(RustcDecodable)]
pub struct LintConfig {
    // filetypes is a map from extension to lints
    filetypes: HashMap<String, Vec<Lint>>,
    global_ignore: Vec<String>,
}

impl Default for LintConfig {
    fn default() -> Self {
        println!("No {} file was found", CONFIG_PATH);
        process::exit(1);
    }
}

fn main() {
    let lint_config: LintConfig = load(CONFIG_PATH);
    let mut new_index = Index::default();

    for (extension, lints) in &lint_config.filetypes {
        let mut files = dirty_files(extension, &lint_config.global_ignore, &mut new_index);

        for lint in lints.iter() {
            if !files.is_empty() {
                println!("Running {} on {} files", lint.name, files.len());
                files = lint.resolve(files);
            }
        }

        while let Some(file) = files.pop() {
            let file_hash = hash(&file);
            new_index.files.insert(file, file_hash);
        }
    }

    save(INDEX_PATH, &new_index);
}

fn dirty_files(extension: &str, ignore: &[String], new_index: &mut Index) -> Vec<String> {
    let old_index: Index = load(INDEX_PATH);
    let mut dirty_files = vec![];

    let mut dirs_to_visit: Vec<String> = vec![".".to_owned()];
    while let Some(dir) = dirs_to_visit.pop() {

        for entry in fs::read_dir(dir).unwrap() {
            let dir_entry = entry.unwrap();

            let file_name = dir_entry.file_name().to_string();
            let file_extension = match dir_entry.path().extension() {
                Some(ext) => ext.to_str().unwrap().to_owned(),
                None => "".to_owned(),
            };

            if ignore.contains(&file_name) {
                continue;
            }

            let path = dir_entry.to_string();

            if (fs::metadata(path.clone()).unwrap()).is_dir() {
                dirs_to_visit.push(path);
                continue;
            }

            if &file_extension != extension {
                continue;
            }

            let file_hash = hash(&path);
            if old_index.files.get(&path) != Some(&file_hash) {
                dirty_files.push(path);
            } else {
                new_index.files.insert(path, file_hash);
            }
        }

    }
    dirty_files
}

fn hash(path: &str) -> String {
    let mut file = match File::open(path) {
        Err(_) => panic!("couldn't open file to get hash: {}", path),
        Ok(file) => file,
    };
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(_) => panic!("couldn't read file to string: {}", path),
        Ok(_) => (),
    };

    let mut hasher = Md5::new();
    hasher.input(s.as_bytes());
    let mut output = [0; 16]; // Md5 is 16 bytes
    hasher.result(&mut output);
    let temp: &[u8] = &output;
    rustc_serialize::hex::ToHex::to_hex(temp)
}

fn save<P: AsRef<Path>, T: Encodable>(path: P, t: &T) {
    let encoded = json::encode(t).unwrap();
    let mut file = File::create(path).unwrap();
    match write!(file, "{}", encoded) {
        Err(_) => panic!(),
        Ok(_) => (),
    };
}

fn load<P: AsRef<Path>, T: Decodable + Default>(path: P) -> T {
    let mut file = match File::open(path) {
        Err(_) => return T::default(),
        Ok(file) => file,
    };
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    json::decode(&s).expect("Could not decode json")
}
