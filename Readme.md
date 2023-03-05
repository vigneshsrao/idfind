# idfind

`idfind` indexes a project to create a json database file. This database can then be used to search the project. When searching `idfind` uses the database to only look at files which can contain the search string instead of looking at all files in the project hence considerably speeding up the search. This can be used to search large projects with lots of files.

Note that `idfind` will not index any binary file.

# Usage

* Install [rust](https://www.rust-lang.org/tools/install)
* clone this repo
* `cargo build --release`
* `cargo run -- --help`

`idfind` has 4 modes of operation:

* `index`: index a project to create the database file
* `cli`: A cli interface for searching. It loads a db and then searches for the string entered by the user in the prompt
* `server`: Spawns TCP server on port `4141` for `idfind`. This will load a db on the server end (loading can be slow for very large projects). A client can connect to it and send it a search string and the database path and it will return the possible files that the search string can be contined in. This is meant to be used with the `search` mode.
* `search`: A TCP client for `idfind`. This will connect to the server to fetch files that can contain the search string and then search those files to print the results

The client-server mode is useful for integrating `idfind` with an editor. An editor plugin can just execute `idfind` command in `search` mode with a server running to do fast searchs from within the editor.

## Emacs Integration

The `emacs` folder contains lisp file that can be used to add `idfind` to GNU Emacs. This is mainly copied over from [GNU idutils](https://www.gnu.org/software/idutils/) with minor changes. To load it in emacs place the file in a folder visible to emacs for loading and add the following to your config - 
```elisp
(autoload 'idfind "idfind" nil t)
```
After that you can use `M-x idfind` to search for strings from within emacs. Note that it uses [`projectile`](https://github.com/bbatsov/projectile) to find the project root. If you don't have projectile you might need to edit this to add logic to find the project root or directly add the full path of the database files.

