use clap::{Arg,Command};
use clap::builder::ArgAction;

use std::path::{PathBuf, Path};

pub struct CLIArgs {
    pub mode: String,
    pub project: PathBuf,
    pub database: String,
    pub expr: String,
    pub include_ext: Vec<String>
}

impl CLIArgs {
    pub fn new() -> Self {

        let matches = Command::new("idfind")
            .author("Vignesh Rao")
            .about("Indexed searcher")
            .arg(
                Arg::new("mode")
                    .long("mode")
                    .short('m')
                    .action(ArgAction::Set)
                    .value_parser(["index", "cli", "server", "search"])
                    .required(true)
                    .help("Which mode to run in")
            )
            .arg(
                Arg::new("include-ext")
                    .long("include-ext")
                    .action(ArgAction::Append)
                    .value_delimiter(',')
                    .help("extensions to include in db. Accepts a list separated by ','")
            )
            .arg(
                Arg::new("project")
                    .long("project")
                    .short('p')
                    .required_if_eq("mode", "index")
                    .action(ArgAction::Set)
                    .value_name("project-root")
                    .help("Path of the project root which is to be indexed")
            )
            .arg(
                Arg::new("database")
                    .long("database")
                    .short('d')
                    .action(ArgAction::Set)
                    .required_if_eq("mode", "cli")
                    .required_if_eq("mode", "search")
                    .help("The database to load")
            )
            .arg(
                Arg::new("expression")
                    .long("expression")
                    .short('e')
                    .action(ArgAction::Set)
                    .required_if_eq("mode", "search")
                    .help("The term to search for")
            )
            .get_matches();


        let empty_string = String::new();

        let mode = matches.get_one::<String>("mode").unwrap().to_string();

        let project = if let Some(path) = matches.get_one::<String>("project") {
            match Path::new(path).canonicalize() {
                Ok(path) if path.is_dir() => path,
                Ok(path) => {
                    println!("Invalid path: {path:?} is not a directory");
                    std::process::exit(-1);
                }
                Err(err) => {
                    println!("Invalid project root: {err}");
                    std::process::exit(-1);
                }
            }
        } else {
            PathBuf::new()
        };

        let database = if let Some(path) = matches.get_one::<String>("database") {
            if !Path::new(path).is_file() {
                println!("Database should be a valid file");
                std::process::exit(-1);
            }

            path.to_string()

        } else {
            String::new()
        };

        let expr = matches.get_one::<String>("expression")
            .unwrap_or(&empty_string)
            .to_string();

        let include_exts = matches.get_many::<String>("include-ext")
            .unwrap_or_default()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();

        CLIArgs {
            mode: mode,
            project: project,
            database: database,
            expr: expr,
            include_ext: include_exts,
        }
    }
}
