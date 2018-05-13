# haricot

Utility to work with HAR files.

I had the objective to reverse-engineer a Web-UI. With Chrome's developer tools it is a breeze
to record the network traffic triggered by specific user actions and save such a bunch of requests
as a HAR file (which describes all requests, URLs, payloads, responses, headers etc. as JSON).

Haricot is a simple tool to print a summary about the entries, and to extract individual payloads,
e.g. POST data or reponse content.
   
    USAGE:
        haricot [FLAGS] [OPTIONS] -f <FILE> [SUBCOMMAND]

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information
        -v               Verbosity level (default=WARNING, 'v'=INFO, 'vv'=DEBUG, 'vvv'=TRACE)

    OPTIONS:
        -c <FILE>        Path to config file. Format may be TOML, YAML, HJSON, JSON.
        -f <FILE>        Path to HAR file.

    SUBCOMMANDS:
        body       Get body data
        entries    Count entries
        help       Prints this message or the help of the given subcommand(s)
        summary    Show a summary of the entries


Example:

    ./target/debug/haricot -f /tmp/my-har-file.har -vv body 3 resp --ecs | jq '.'


---

But foremost, Haricot is my first application while learning Rust. My objectives were,
of course, Rust, and also to glimpse into "clap" for command line parsing and "serde"
to parse and manipulate JSON structures.

As said, this is my first app in Rust, so the code is not elegant, and surely many unnecessary
type casts and (de-)references happen. -- WIP.

## Compile

With a nightly Rust toolchain, run

    cargo build

and then have a look at

    ./target/debug/haricot --help

For development I used this watcher

    cargo watch -x 'run -- -v -f /tmp/my-har-file.har'


