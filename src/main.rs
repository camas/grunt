use clap::{clap_app, crate_authors, crate_description, crate_version};
use grunt::{get_project_dirs, init_logging, Grunt};

/// Parses inputs and initializes grunt
fn main() {
    let app = clap_app!(("grunt") =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg verbosity: -v +multiple "Verbosity. -v to -vvvvv")
        (@subcommand update =>
            (about: "Update addons")
        )
        (@subcommand add =>
            (about: "Add addon(s)")
        )
        (@subcommand remove =>
            (about: "Remove addon(s)")
        )
        (@subcommand tsm =>
            (about: "Update TSM auction data")
        )
    );

    // Parse args
    let matches = app.get_matches();

    // Set up logging
    let verbosity = matches.occurrences_of("verbosity") as u8;
    let log_path = get_project_dirs().data_dir().join("log.txt");
    init_logging(verbosity, Some((log_path, 5)));

    // Init grunt
    //let mut grunt = Grunt::init();
    todo!();

    // Run command
    match matches.subcommand() {
        _ => println!("No matched command"),
    }
}
