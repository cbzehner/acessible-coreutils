use std::process;

#[macro_use]
extern crate clap;
use atty::Stream;
use clap::{App, Arg};
use yaml_rust::Yaml;

// TODO: Generate completions for all shells (https://docs.rs/clap/2.33.3/clap/enum.Shell.html)

const FAILURE_UNKNOWN: i32 = -1;

// TODO: Write tests!
// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//         assert_eq!(2 + 2, 4);
//     }
// }

/// Given a CLAP-style YAML configuration file with a few additional fields (see below) build the executable wrapper
/// which will be used to translate from the CLAP configuratino to the underlying binary.
/// - executable: the name of the underlying binary wrapped by this library
/// - humanize: a list of extra arguments to include if a human runs the command
/// - map: for each non-positional argument, specify how to map the wrapper to the underlying binary
pub fn build_executable(yaml: &Yaml) {
    // Build the wrapper based on the YAML configuration.
    let matches = App::from(yaml)
        // Global arguments available in all binaries
        .args(&[
            // TODO: Set a verbosity level.
            // Arg::with_name("verbose")
            //     .short("v")
            //     .multiple(true)
            //     .help("Sets the level of verbosity"),
            Arg::new("version")
                .long("version")
                .about("Show the current version"),
        ])
        .author(crate_authors!())
        .version(crate_version!())
        .get_matches();

    // Get the name of the underlying binary.
    let command_name = yaml["executable"].as_str().unwrap();
    // Initialize a vector to hold arguments to pass through to the underlying binary.
    let mut command_args: Vec<&str> = vec![];

    // Print the version and exit when the version flag is requested.
    if matches.is_present("version") {
        println!("{}", crate_version!());
        process::exit(0)
    }

    // If the command is run by a human, include the humanize flags.
    if atty::is(Stream::Stdout) {
        if !&yaml["humanize"].is_badvalue() {
            command_args.push(&yaml["humanize"].as_str().unwrap())
        }
    }

    // Iterate through the args from the YAML configuration.
    let wrapper_args = yaml["args"].as_vec().unwrap();
    for arg in wrapper_args {
        for (k, v) in arg.as_hash().unwrap() {
            let key = k.as_str().unwrap();
            // If the argument is present in the current invocation...
            if matches.is_present(key) {
                // If a map is present on the config, push it to the list of args for the wrapped command.
                if !&v["map"].is_badvalue() {
                    command_args.push(&v["map"].as_str().unwrap())
                }

                // If a value is present in the wrapper, push it to the list of args for the wrapped command.
                if let Some(value) = matches.value_of(key) {
                    command_args.push(value)
                }
            }
        }
    }

    // Call the wrapped command by name, passing in any relevant arguments.
    match command(command_name, command_args) {
        Ok(success) => process::exit(success),
        Err(message) => {
            eprintln!("Error: {}", message);
            process::exit(FAILURE_UNKNOWN)
        }
    }
}

/// Spawn a child process to run the named command with the provided arguments. Block until the process completes and then return the resulting status code and error message.
/// Note: A successful (`Ok(i32)`) result does not indicate that the invoked command ran successfully (exit status: 0) but rather indicates that the wrapped command was successfully invoked.
fn command(name: &str, arguments: Vec<&str>) -> Result<i32, String> {
    let child = process::Command::new(name)
        .args(&arguments)
        .spawn()
        .map_err(|error| error.to_string())?;
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    return output.status.code().ok_or("Missing ExitStatus".into());
}
