use clap::{clap_app, crate_description, crate_version, AppSettings};
use dialoguer;
use grunt::settings::Settings;
use grunt::{get_project_dirs, Grunt};

/// Parses inputs and initializes grunt
fn main() {
    let app = clap_app!(("grunt") =>
        (version: crate_version!())
        (about: crate_description!())
        (setting: AppSettings::ArgRequiredElseHelp)
        (@subcommand setdir =>
            (about: "Change default directory")
            (@arg dir: +required "The directory to use")
        )
        (@subcommand resolve =>
            (about: "Resolve untracked addons")
        )
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
    let mut settings = Settings::from_file_or_new(&settings_path);

    // Set addon dir first
    let subcommand = matches.subcommand();
    if subcommand.0 == "setdir" {
        let args = subcommand.1.unwrap();
        let dir = args.value_of("dir").unwrap().to_string();
        settings.set_default_dir(Some(dir.clone()));
        settings.save(&settings_path);
        println!("Addon directory set to '{}'", dir);
    }

    // Init grunt
    let addon_dir = match settings.default_dir() {
        Some(dir) => dir,
        None => {
            println!("No Addon directory setup. Change it using the `setdir` command");
            return;
        }
    };
    let mut grunt = Grunt::new(addon_dir);

    // Print header
    println!("\x1B[1mGrunt - WoW Addon Manager+\x1B[0m");
    println!("{}", grunt.root_dir().to_str().unwrap());
    println!("{} addons", grunt.addons().len());
    let untracked = grunt.find_untracked();
    if !untracked.is_empty() {
        println!("{} untracked addon dirs", untracked.len());
    }
    println!();

    // Run command
    match matches.subcommand() {
        ("setdir", _) => {} // Implemented further up
        ("update", _) => grunt.update_addons(),
        ("resolve", _) => {
            // Resolve
            println!("Resolving untracked addons...");
            println!();
            let mut first = true;
            let prog_func = move |prog| match prog {
                grunt::ResolveProgress::NewAddon { name, desc } => {
                    if first {
                        println!("\x1B[1mFound:\x1B[0m");
                        first = false;
                    }
                    println!("{:32} {}", name, desc)
                }
                grunt::ResolveProgress::Finished { not_found } => {
                    println!("\x1B[1m{} unresolved:\x1B[0m", not_found.len());
                    not_found.iter().for_each(|x| println!("{}", x));
                }
            };
            grunt.resolve(prog_func);
            grunt.save_lockfile();
        }
        _ => println!("No matched command"),
    }
}
