use std::{
    env, fs, thread,
    path::PathBuf,
    sync::{Mutex, Arc, mpsc}
};

#[derive(Debug)]
pub struct Hunter {
    query: String, // String to search for
    root_path: String, // Directory to start the search at
    thread_count: u32, // Number of threads spawned
    ignore_case: bool, // False by default
    pub print_help: bool, // Whether the help menu should be displayed
    exclude_elements: Vec<PathBuf> // List of elements in root dir to exclude (specified with --exclude)
}

impl Hunter {
    // Builds a Hunter instance from the command line arguments
    // and returns it.
    pub fn build() -> Result<Self, String> {
        let mut config = Self {
            query: String::new(),
            root_path: String::new(),
            thread_count: 4,
            ignore_case: false,
            print_help: false,
            exclude_elements: vec![]
        };

        let args: Vec<String> = env::args().collect();
        if args.len() < 2 {
            return Err("Invalid arguments!".to_string());
        }

        for (i, arg) in args.iter().enumerate() {
            if i == 0 { // Skipping name of program
                continue;
            }
            match &arg[..] {
                "--ignore-case" => {
                    config.ignore_case = true;
                },
                "--exclude" => {
                    if i + 1 >= args.len() {
                        return Err("Please specify elements to exclude. See --help".to_string());
                    } else if config.query.is_empty() || config.root_path.is_empty() {
                        return Err("Invalid order of arguments. See --help".to_string());
                    }
                    config.exclude_elements = args[i+1].split(",").map(|string_slice| {
                        // Adding name of element only. For example,
                        // '~/Downloads/file' becomes 'file' 
                        let filename = String::from(PathBuf::from(string_slice).file_name().unwrap().to_str().unwrap());
                        PathBuf::from(filename) 
                    }).collect();
                },
                "--help" => {
                    config.print_help = true;
                    break;
                },
                _ => {
                    let arg_as_string = String::from(arg);
                    if config.query.is_empty() {
                        config.query = arg_as_string;
                    } else if config.root_path.is_empty() {
                        config.root_path = arg_as_string;
                    } else if config.exclude_elements.is_empty() {
                        return Err("Invalid arguments!".to_string());
                    } 
                }
            }
        }

        if !config.print_help && args.len() < 3 {
            return Err("Invalid arguments!".to_string());
        }

        Ok(config)
    }

    // Reads the elements within the root directory into a vector
    // and passes it to self.init_threads().
    pub fn start_search(&self) {
        if let Ok(read_dir) = fs::read_dir(PathBuf::from(&self.root_path)) {
            let root_contents: Vec<PathBuf> = read_dir
                .map(|result| result.unwrap().path())
                .filter(|pathbuf| { // Only add elements to the vector that shouldn't be excluded
                    let element_name = pathbuf.file_name().unwrap().to_str().unwrap();
                    !self.exclude_elements.contains(&PathBuf::from(element_name)) 
                })
                .collect();

            self.init_threads(root_contents);
        }
    }

    /* 
    - Creates a queue. The queue is a vector holding paths to directories (in the beginning files in the root dir as well) found during the search and
    it is permanently being updated with new paths from the threads until all elements in the root directory (and its children) 
    have been analyzed.
    - Spawns threads that analyze the elements in the queue and send paths to newly found directoriess back to the main thread. 
    */
    fn init_threads(&self, root_contents: Vec<PathBuf>) {
        let (sender, receiver) = mpsc::channel();
        
        // matches_counters: [number of matching dirs found, number of matching files found]
        let matches_counters = Arc::new( Mutex::new(vec![0, 0]) );

        let ignore_case = self.ignore_case;
        let queue = Arc::new( Mutex::new(root_contents) );
        for _ in 0..self.thread_count {
            let sender = sender.clone();
            
            let query = String::from(&self.query);
            
            let matches_counters = Arc::clone(&matches_counters);
            let queue = Arc::clone(&queue);
            thread::spawn(move || {
                let mut element_match: bool; // Whether the file's/ folder's name contains the query
                loop {
                    element_match = false;
                    // Path to the element to be analyzed
                    let path_from_queue = {
                        let mut queue = queue.lock().unwrap();
                        queue.pop()
                    };
                    if let Some(pathbuf) = path_from_queue {
                        if let Some(os_str) = pathbuf.file_name() {
                            if let Some(str) = os_str.to_str() {
                                if ignore_case {
                                    let filename_lowercase = String::from(str).to_lowercase();
                                    if filename_lowercase.contains(&query.to_lowercase()) {
                                        print_match(&pathbuf, str, &query, true);
                                        element_match = true;
                                    }
                                }

                                else if str.contains(&query) {
                                    print_match(&pathbuf, str, &query, false);
                                    element_match = true;
                                }
                            }
                        }
                        
                        if pathbuf.is_dir() {
                            if element_match {
                                let mut matches_counters = matches_counters.lock().unwrap();
                                matches_counters[0] += 1;
                            }
                            if let Ok(read_dir) = fs::read_dir(pathbuf) {
                                for result in read_dir {
                                    // Sending each sub-element in the pathbuf (which is a directory)
                                    // back to the main thread.
                                    sender.send( result.unwrap().path() );
                                }
                            }
                            
                        } else if pathbuf.is_file() && element_match {
                            let mut matches_counters = matches_counters.lock().unwrap();
                            matches_counters[1] += 1;
                        }
                    } else {
                        // There are no elements left in the queue.
                        break;
                    }
                }
            });
        }
        // Dropping the original sender.
        // Cloned versions may still exist in threads, but get dropped as threads end.
        drop(sender);
        // Directories sent from the spawned threads will be pushed into the queue
        // so the threads can continue analyzing.
        for dir in receiver {
            let mut queue = queue.lock().unwrap();
            queue.push(dir);
        }

        let matches_counters = {
            matches_counters.lock().unwrap()
        };
        let (dirs, files) = (matches_counters[0], matches_counters[1]);
        println!("\n===========================");
        println!("{} matches found.", dirs+files);
        println!("  - Directories: {}\n  - Files: {}", dirs, files);
        println!("===========================");
    }
}

/*
- Finds where the element's name matches the query (case-sensitive/-insensitive).
- Prints the element's name with the matching substring highlighted.
*/
fn print_match(path: &PathBuf, filename: &str, query: &str, ignore_case: bool) {
    let mut path = path.clone();
    path.pop();
    if let Some(_) = path.to_str() {
        if ignore_case {
            let mut filename = String::from(filename);
            if let Some(index) = filename.to_lowercase().find(String::from(query).to_lowercase().as_str()) {
                let matching_slice = &filename[index..index+query.len()];
                filename.replace_range(index..index+query.len(), &format!("\u{001b}[31m{}\u{001b}[0m", matching_slice));
                
                path.push(filename);
            }
        }
        else {
            let colored_query = format!("\u{001b}[31m{}\u{001b}[0m", query);
            let filename_highlighted = String::from(filename).replace(query, &colored_query);
            path.push(filename_highlighted);
        }

        println!("{}", path.to_str().unwrap());
    }
}

impl Hunter {
    pub const HELP_MENU: &'static str = "==========\x1b[1mFS_HUNT\x1b[0m==========    

USAGE:
    fs_hunt <KEYWORD> <DIRECTORY> [OPTIONS]


OPTIONS:
    --help              Display this menu
    --ignore-case       Initiate case-insensitive search
    --exclude           Comma-separated list of elements in the starting directory to ignore.


EXAMPLES:
    -> Search /home/my_user/Desktop for filenames containing \"pdf\":
        \x1b[1m fs_hunt \"pdf\" /home/my_user/Desktop \x1b[0m
    
    -> Search ./Pictures for filenames containing \"4k\" or \"4K\", but
    ignore ./Pictures/Backups and ./Pictures/Wallpapers:
        \x1b[1m fs_hunt 4k Pictures --ignore-case --exclude Backups,Wallpapers \x1b[0m";
}