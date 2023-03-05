use rayon;
use rayon::prelude::*;

use std::io::*;
use std::time::Instant;
use std::sync::mpsc;
use std::thread;
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};

mod cli;
mod network;
mod utils;
mod idb;

use idb::Idb;
use utils::*;
use cli::CLIArgs;
use network::{Request, Response, Transfer};

fn cli(args: CLIArgs) {
    let db = unwrap!(Idb::load(&args.database), "Error while loading db");

    unwrap!(std::env::set_current_dir(&db.project_root),
            "Unable to change current dir");

    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        unwrap!(std::io::stdin().read_line(&mut input), "User input failed");

        let input = input.strip_suffix("\n").unwrap_or(&input);

        if input.len() < 3 {
            break;
        }

        let found = db.find(input);
        if found == 0 {
            println!("{_FAIL}{_BOLD}[!] Not Found{_ENDC}");
        } else {
            println!("{_OKGREEN}{_BOLD}Hits: {found}{_ENDC}");
        }
    }
}

fn main() {

    let args = CLIArgs::new();

    if args.mode == "index" {

        unwrap!(std::env::set_current_dir(&args.project),
                "Unable to change current dir");

        let mut db = Idb::new(&args.project);

        db.iterate_dir(&args.include_ext);

        println!("Files Indexed: {}",db.cur_id);
        unwrap!(db.save(), "Error while serializing and saving the database");

    } else if args.mode == "cli" {
        cli(args);
    } else if args.mode == "server" {

        let listener = unwrap!(TcpListener::bind("127.0.0.1:4141"),
                               "Failed to bind to given host/port");

        let mut map: HashMap<String, mpsc::Sender<(String, TcpStream)>>
            = HashMap::new();

        for stream in listener.incoming() {
            let mut stream = unwrap_continue!(stream, "Unable to get stream");
            let req = unwrap_continue!(Request::receive(&mut stream),
                                       "Error Receiving Request");

            if map.get(&req.dbname).is_none() {
                let (tx, rx) = mpsc::channel();
                map.insert(req.dbname.clone(), tx);
                let dbname = req.dbname.clone();
                thread::spawn(move || {
                    handle_connection(dbname, rx);
                });
            };

            // This unwrap is safe as we already handled the else part above
            let tx = map.get(&req.dbname).unwrap();


            if tx.send((req.needle, stream)).is_err() {
                println!("Lost connection to thread! Error db: {}",
                         &req.dbname);

                let _ = map.remove(&req.dbname);
            }
        }

    } else if args.mode == "search" {

        let database = args.database.clone();
        let needle   = args.expr;
        let request  = Request {
            dbname: database,
            needle: needle.clone(),
        };


        let mut stream = unwrap!(TcpStream::connect("127.0.0.1:4141"),
                                 "Failed to connect to the server");

        unwrap!(request.send(&mut stream), "Failed to send request to the server");

        let resp = unwrap!(Response::receive(&mut stream),
                           "Failed to receive a response from the server");

        if resp.error {
            println!("Error: {}", resp.message);
            std::process::exit(-1);
        }

        let path = if resp.message == "." {
            "/home/vignesh/Documents/exploits/firefox/firefoxnew/gecko-dev/".to_string()
        } else {
            resp.message.clone()
        };

        unwrap!(std::env::set_current_dir(&path), "unable to change cwd");
        let now = Instant::now();

        let found: usize = resp.files
                               .par_iter()
                               .map(|path| check_file(&path, &needle))
                               .sum();

        print_time_stats("Query", now.elapsed());
        println!("Searched files: {}", resp.files.len());

        if found == 0 {
            println!("{_FAIL}{_BOLD}[!] Not Found{_ENDC}");
        } else {
            println!("{_OKGREEN}{_BOLD}Hits: {found}{_ENDC}");
        }

    }

}

fn handle_connection(dbname: String, rx: mpsc::Receiver<(String, TcpStream)>) {
    let db = match Idb::load(&dbname) {
        Ok(db)   => db,
        Err(err) => {
            println!("Error Loading db: {err}");
            return;
        }
    };

    let project_root = db.project_root.to_str().unwrap().to_string();

    std::env::set_current_dir(&db.project_root).unwrap();
    for (message, mut stream) in rx.iter() {

        // Reject the request if the len of the search string is too small
        if message.len() < 3 {
            let resp  = Response::err("Input to short");
            unwrap_continue!(resp.send(&mut stream), "unable to send resp");
            continue;
        }

        let found = db.find_file_names(&message);
        let resp  = Response::new(project_root.clone(), found);
        unwrap_continue!(resp.send(&mut stream), "unable to send resp");
    }
}
