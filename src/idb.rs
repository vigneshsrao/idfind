use rayon;
use rayon::prelude::*;

use walkdir::{DirEntry, WalkDir};

use serde::{Deserialize, Serialize};
use serde_json;

use std::fs;
use std::io::*;
use std::time::Instant;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

use crate::utils::*;

/// The search index database
#[derive(Serialize, Deserialize)]
pub struct Idb {
    pub cur_id:         u32,
    pub project_root:   PathBuf,
    pub idx_db:         HashMap<u32, PathBuf>,
    pub str_db:         HashMap<String, HashSet<u32>>,
}

impl Idb {

    /// Create a new Idb for the project at path `project`
    pub fn new(project: &PathBuf) -> Idb {
        Idb {
            cur_id: 0,
            project_root: project.clone(),
            idx_db: HashMap::new(),
            str_db: HashMap::new(),
        }
    }

    /// Load the search database from file and create an `Idb` instance
    pub fn load(path: &String) -> Result<Idb> {

        println!("Loading database: {path}");

        let now = Instant::now();

        let json = fs::read_to_string(path)?;
        let db   = serde_json::from_str(&json)?;

        print_time_stats("Loading", now.elapsed());

        Ok(db)
    }

    /// JSON serialize this database and save it into the `sdb.json` file in the
    /// current working dir
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(&self)?;
        fs::write("sdb.json", json)?;
        Ok(())
    }

    /// Iterate over all keys passed and add them to the database
    fn update_db(&mut self, fname: &PathBuf, keys: HashSet<String>) {

        let id = self.cur_id;
        self.cur_id+=1;
        self.idx_db.insert(id, fname.clone());

        keys.iter().for_each(|key| {
            if let Some(ref mut set) = self.str_db.get_mut(key) {
                set.insert(id);
            } else {
                let mut set = HashSet::new();
                set.insert(id);
                self.str_db.insert(key.to_string(), set);
            }
        });
    }

    /// This function will iterate over the directory that is passed to it and build
    /// the search database from the files present in those dirs
    pub fn iterate_dir(&mut self, valid_exts: &Vec<String>) {

        let ext_filter = !valid_exts.is_empty();

        let is_hidden = |entry: &DirEntry|  {
            entry.file_name()
                .to_str()
                .map(|s| s.starts_with(".") && (s.len() > 1))
                .unwrap_or(false)
        };

        println!("Enumerating files...");

        let now = Instant::now();

        // Iterate over the dir structure, and collect all the files that we are
        // interested in
        let files: Vec<PathBuf> = WalkDir::new(".")
            .into_iter()
            .filter_entry(|entry| !is_hidden(entry))
            .filter_map(|x| x.ok())
            .filter(|x| {

                // If this is not a file, then skip this entry
                if !x.file_type().is_file() {
                    return false;
                }

                // Get the file extension and convert it to a str
                let ext = x.path()
                        .extension()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default();

                // If this is an extension to be skipped, then skip it
                if SKIP_EXT.contains(&ext) {
                    return false;
                }

                // If we don't have include extension filter then we are done
                if !ext_filter {
                    return true;
                }

                // Process this only if it is present in the extension whitelist
                valid_exts.contains(&&ext.to_string())
            }).map(|entry| entry.path()
                .strip_prefix("./")
                .unwrap_or(entry.path())
                .to_path_buf())
            .collect();

        print_time_stats("Enumeration", now.elapsed());

        let totalfiles = files.len() as u64;
        let pfiles = Arc::new(AtomicU64::new(0));
        let pfiles_clone = Arc::clone(&pfiles);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        println!("\nFiles to process: {}", files.len());

        let now = Instant::now();

        // We are going to parallelize the iteration. First we spawn a worker thread
        // to do insertion into the database. Then we start iterating over the file
        // we previously collected to read the file contents, split it into trigrams
        // and then send this along with the file name to the worker that inserts
        // this data into the db.
        rayon::scope(|s| {

            // Create a channel for the workers to communicate
            let (token_tx, token_rx) = mpsc::channel();
            let (data_tx,  data_rx)  = mpsc::channel();

            // Spawn a thread to show the progress of the indexing
            s.spawn(move |_| {
                let dur = std::time::Duration::from_millis(100);

                print!("\x1b[?25l");
                loop {
                    let p = pfiles_clone.load(Ordering::SeqCst);
                    let pc = (p as f64 / totalfiles as f64 )* 100f64;
                    print!("\rIndexing [{:6}/{totalfiles}] {:.2}%", p, pc);
                    if stop_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(dur);

                    stdout().flush().unwrap();
                }
                print!("\x1b[?25h");
                print!("\n");
            });

            // The worker thread to insert data into the database
            s.spawn(move |_| {
                token_rx.iter().for_each(|(input, fname)| {
                    self.update_db(&fname, input);
                    pfiles.fetch_add(1, Ordering::SeqCst);
                });
                stop.store(true, Ordering::SeqCst);
            });

            // Iterate over the file data to split it into trigrams and transmit
            // them to the receiver worker.
            s.spawn(move |_| {
                data_rx.into_iter()
                    .par_bridge()
                    .for_each_with(token_tx, |token_tx, (data, fname): (String, PathBuf)| {
                        let idc = get_indices(&data);
                        if idc.len() < 4 {
                            return;
                        }

                        // We are going to use a set to store the trigrams. This
                        // will prevent duplicate str's and significantly improve
                        // the time required to insert into the db
                        let mut key_set = HashSet::new();
                        (0..idc.len()-3).into_iter().for_each(|i| {
                            key_set.insert(data[idc[i]..idc[i+3]].to_string());
                        });

                        token_tx.send((key_set, fname)).unwrap();
                    });
            });

            // Iterate over the collected files, read the data and then and transmit
            // them to the receiver worker.
            for file in files {

                if let Ok(input) = fs::read_to_string(file.as_path()) {

                    // Ignore files too small to tokenize
                    if input.len() < 3 {
                        return;
                    }

                    // Send the trigrams set along with the file name to the
                    // worker thread for inserting into the db
                    data_tx.send((input, file.clone())).unwrap();
                }
            }
        });

        print_time_stats("Indexation", now.elapsed());
    }

    /// Search for the input string using the provided index. Returns the number of
    /// lines on which this input was found.
    pub fn find(&self, input: &str) -> usize {

        // Get the files likely to contain the input string
        let files = self.find_file_names(input);
        let total = files.len();

        let now = Instant::now();

        // Parallely check all the files to see which all contain the input and
        // sum the total number of lines found in files
        let found: usize = files.par_iter()
                                .map(|path| check_file(path, input))
                                .sum();

        print_time_stats("Query", now.elapsed());
        println!("Searched files: {total}");

        found
    }

    /// Generates a list of file names which might contain the string passed as
    /// input
    pub fn find_file_names(&self, input: &str) -> Vec<PathBuf> {

        // Tokenize the input string
        let tokens = tokenize(input);

        // Make a list of sets which match contain the input string in the db
        let mut hits = Vec::new();
        for token in tokens {
            if let Some(set) = self.str_db.get(&token) {
                hits.push(set);
            } else {
                return vec![];
            }
        }

        // Take an intersection of all the sets in the list generated above and
        // then map the file id back to the original file name
        // Note: The BitAnd in the fold will create a new set each time. We can
        // optimize this further if required
        let intset: HashSet<u32> = hits[0].clone();

        let found = hits.iter()
                        .fold(intset, |acc, set| &acc & *set)
                        .iter()
                        .filter_map(|id| self.idx_db.get(&id).map(|f| f.clone()))
                        .collect::<Vec<_>>();

        found
    }
}
