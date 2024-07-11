use std::error::Error;
use std::{
    env, fs,
};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Config {
    pub query: String,
    pub root_path: String
}

impl Config {
    pub fn build() -> Result<Self, String> {
        let mut config = Self {
            query: String::new(),
            root_path: String::new()
        };

        let args: Vec<String> = env::args().collect();
        if args.len() != 3 {
            return Err("Invalid arguments!".to_string());
        }
        config.query = String::from(&args[1]);
        config.root_path = String::from(&args[2]);

        Ok(config)
    }
}

pub fn search_dir(query: &str, dir: PathBuf) -> Result<(), Box<dyn Error>> {
    let dir_contents = fs::read_dir(dir)?;
    for result in dir_contents {
        let dir_entry = result?;
        let path = dir_entry.path();
        
        let file_name_option = path.file_name();
        if let Some(os_str) = file_name_option {
            let filename = String::from(os_str.to_str().unwrap());
            if filename.contains(query) {
                print_match(&path, query);
            }
        }

        if path.is_dir() {
            search_dir(query, path);
        }
    }

    Ok(())
}

fn print_match(path: &PathBuf, query: &str) {
    let mut path_highlight = String::new();
    if let Some(str) = path.to_str() {
        path_highlight.push_str(str);
        path_highlight = path_highlight.replacen(query, &format!("\u{001b}[31m{}\u{001b}[0m", query), 1);
        println!("{}", path_highlight);
    }
}