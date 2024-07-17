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
    ignore_case: bool // False by default
}

impl Hunter {
    // Builds a Hunter instance from the command line arguments
    // and returns it.
    pub fn build() -> Result<Self, String> {
        let mut config = Self {
            query: String::new(),
            root_path: String::new(),
            thread_count: 4,
            ignore_case: false
        };

        let args: Vec<String> = env::args().collect();
        if args.len() < 3 {
            return Err("Invalid arguments!".to_string());
        }

        for arg in &args[1..] {
            match &arg[..] {
                "--ignore-case" => {
                    config.ignore_case = true;
                },
                _ => {
                    let arg_as_string = String::from(arg);
                    if config.query.is_empty() {
                        config.query = arg_as_string;
                    } else if config.root_path.is_empty() {
                        config.root_path = arg_as_string;
                    } else {
                        return Err("Invalid arguments!".to_string());
                    }
                }
            }
        }

        println!("{:?}", config);
        Ok(config)
    }

    // Reads the elements within the root directory into a vector
    // and passes it to self.init_threads().
    pub fn start_search(&self) {
        if let Ok(read_dir) = fs::read_dir(PathBuf::from(&self.root_path)) {
            let root_contents: Vec<PathBuf> = read_dir
                .map(|result| result.unwrap().path())
                .collect();
            
            self.init_threads(root_contents);
        }
    }

    /* 
    - Creates a queue. The queue is a vector holding paths to files/ folders found during the search and
    it is permanently being updated by the threads until all elements in the root directory (and its children) 
    have been analyzed.
    - Spawns threads that edit the queue and analyze the elements in it. 
    */
    fn init_threads(&self, root_contents: Vec<PathBuf>) {
        let (sender, receiver) = mpsc::channel();
        
        let ignore_case = self.ignore_case;
        let queue = Arc::new( Mutex::new(root_contents) );
        for _ in 0..self.thread_count {
            let sender = sender.clone();
            let query = String::from(&self.query);
            let queue = Arc::clone(&queue);
            thread::spawn(move || {
                loop {
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
                                    }
                                }

                                else if str.contains(&query) {
                                    print_match(&pathbuf, str, &query, false);
                                }
                            }
                        }
                        
                        if pathbuf.is_dir() {
                            if let Ok(read_dir) = fs::read_dir(pathbuf) {
                                for result in read_dir {
                                    // Sending each sub-element in the pathbuf (which is a directory)
                                    // back to the main thread.
                                    sender.send( result.unwrap().path() );
                                }
                            }
                            
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