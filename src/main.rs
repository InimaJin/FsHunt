mod search;
use search::Hunter;

fn main() {
    let config_result = Hunter::build();
    let hunter: Hunter;
    if let Ok(c) = config_result {
        hunter = c;
        hunter.start_search();
    } else if let Err(msg) = config_result {
        eprintln!("{}", msg);
    }    
}