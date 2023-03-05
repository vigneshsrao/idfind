use std::time::Duration;
use std::fs;
use std::path::PathBuf;

#[macro_export]
macro_rules! unwrap {
    ($result: expr, $message: expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => {
                println!("[-] {}: {}", $message, err);
                std::process::exit(-1);
            }
        }
    }
}

#[macro_export]
macro_rules! unwrap_continue {
    ($result: expr, $message: expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => {
                println!("[-] {}: {}", $message, err);
                continue;
            }
        }
    }
}

// Probably should use a crate for this ;)
pub static _HEADER:     &str = "\x1b[95m";
pub static _OKBLUE:     &str = "\x1b[94m";
pub static _OKCYAN:     &str = "\x1b[96m";
pub static _OKGREEN:    &str = "\x1b[92m";
pub static _WARNING:    &str = "\x1b[93m";
pub static _FAIL:       &str = "\x1b[91m";
pub static _ENDC:       &str = "\x1b[0m";
pub static _BOLD:       &str = "\x1b[1m";
pub static _UNDERLINE:  &str = "\x1b[4m";

/// The extensions to skip. These are all binary formats so even if they are not
/// skipped, they will still not be indexed.
pub static SKIP_EXT: [&str; 22] = [
    "png", "jpg", "jpeg", "pdf",
    "pyc", "zip",  "tgz", "tar",
    "gz",   "so",  "bin", "wasm",
    "o",  "rlib", "json", "dat",
    "whl", "wav",  "pcm", "avif",
    "rmeta", "a",
];

/// The max length of a line for a search match to be printed on the screen.
/// Matches with length over this are not printed in full, but only the filename
/// and line number.
static MAX_LEN: usize = 100;

pub fn print_time_stats(msg: &str, elapsed: Duration) {
    println!("\n==== {msg} Done ====");
    println!("{msg} took: {:.2} ns / {:.2} us / {:.2} ms / {:.2}s",
             elapsed.as_nanos(), elapsed.as_micros(),
             elapsed.as_millis(), elapsed.as_secs_f64());

}


/// Check if the file that is passed as arg contains the `input` string. It
/// returns the number of lines on which the `input` was found
pub fn check_file(path: &PathBuf, input: &str) -> usize {

    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(_)   => return 0
    };

    let mut hits = String::new();
    let mut hit_count = 0;
    let path = match path.as_path().to_str() {
        Some(path) => path,
        None       => return 0
    };

    let jstr = format!("{_FAIL}{_BOLD}{input}{_ENDC}");

    data.split("\n").enumerate().for_each(|(lno, line)| {
        if line.find(input).is_some() {
            let data = if line.len() > MAX_LEN {
                // "*[long matching line]*"
                String::from("*[long matching line]*")
            } else {
                let splits = line.split(input).collect::<Vec<_>>();
                splits.join(&jstr)
            };
            // hits += &format!("{_FAIL}{:04}{_ENDC}:   {}\n", lno+1, data);
            // hits += &format!("{_HEADER}{path}{_ENDC}:{_OKBLUE}{}{_ENDC}:   {}\n", lno+1, &data);
            hits += &format!("{path}:{}:   {}\n", lno+1, &data);
            hit_count += 1;
        }
    });

    if !hits.is_empty() {
        // hits = format!("{_HEADER}{}{_ENDC}\n",
        //                path.as_path().to_str().unwrap()) + &hits;
        println!("{hits}");
    }

    hit_count
}


/// Get a vector of indices for the string passed as input
pub fn get_indices(sample: &str) -> Vec<usize> {
    let mut idc: Vec<usize> = sample.char_indices().map(|(i, _)| i).collect();
    idc.push(sample.len());
    idc
}

/// Split a string into trigrams
pub fn tokenize(sample: &str) -> Vec<String> {
    let idc = get_indices(sample);
    (0..idc.len()-3).map(|i| String::from(&sample[idc[i]..idc[i+3]])).collect()
}
