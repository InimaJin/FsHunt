mod search;
use search::Hunter;

fn main() {
    let config_result = Hunter::build();
    let hunter: Hunter;
    if let Ok(config) = config_result {
        hunter = config;
        if hunter.print_help {
            println!("{}", Hunter::HELP_MENU);
        } else {
            hunter.start_search();
        }
    } else if let Err(msg) = config_result {
        eprintln!("{}", msg);
    }    
}