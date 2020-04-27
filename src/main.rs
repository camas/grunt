use clap::{clap_app, crate_authors, crate_description, crate_version};
use grunt::settings::Settings;
use grunt::{get_project_dirs, Grunt};

/// Parses inputs and initializes grunt
fn main() {
    let app = clap_app!(("grunt") =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
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

    // Create directories if they don't exist
    let config_dir = get_project_dirs().config_dir();
    if !config_dir.exists() {
        std::fs::create_dir(config_dir).expect("Error creating config dir");
    }

    // Init settings
    let settings_path = config_dir.join("config.json");
    let settings = Settings::from_file_or_new(settings_path);

    // Init grunt
    //let mut grunt = Grunt::init();
    todo!();

    // Run command
    match matches.subcommand() {
        _ => println!("No matched command"),
    }
}
