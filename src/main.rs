use std::path::PathBuf;

mod search;
use search::Config;

fn main() {
    let config_result = Config::build();
    let mut config: Config;
    if let Ok(c) = config_result {
        config = c;
        println!("{:#?}", config);
    } else {
        eprintln!("{}", config_result.unwrap_err());
        std::process::exit(1);
    }


    // Initiate recursive search
    search::search_dir(&config.query, PathBuf::from(config.root_path));
}