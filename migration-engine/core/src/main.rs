pub mod api;
pub mod cli;
pub mod commands;
mod error;
pub mod migration;
pub mod migration_engine;

use crate::api::RpcApi;
use commands::*;
use datamodel::{self, error::ErrorCollection, Datamodel};
use log::*;
use std::{env, fs, io, io::Read};

pub use error::Error;
pub use migration_engine::*;

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn parse_datamodel(datamodel: &str) -> CommandResult<Datamodel> {
    let result = datamodel::parse_datamodel_or_pretty_error(&datamodel, "datamodel file, line");
    result.map_err(|e| CommandError::Generic { code: 1001, error: e })
}

pub(crate) fn pretty_print_errors(errors: ErrorCollection, datamodel: &str) {
    let file_name = "schema.prisma".to_string();

    for error in errors.to_iter() {
        println!();
        error
            .pretty_print(&mut io::stderr().lock(), &file_name, datamodel)
            .expect("Failed to write errors to stderr");
    }
}

fn main() {
    env_logger::init();

    let matches = cli::clap_app().get_matches();

    if matches.is_present("version") {
        println!(env!("GIT_HASH"));
    } else if let Some(matches) = matches.subcommand_matches("cli") {
        let datasource = matches.value_of("datasource").unwrap();

        match std::panic::catch_unwind(|| cli::run(&matches, &datasource)) {
            Ok(Ok(msg)) => {
                info!("{}", msg);
                std::process::exit(0);
            }
            Ok(Err(error)) => {
                error!("{}", error);
                let exit_code = error.exit_code();
                serde_json::to_writer(std::io::stdout(), &cli::render_error(error)).expect("failed to write to stdout");
                std::process::exit(exit_code);
            }
            Err(panic) => {
                serde_json::to_writer(
                    std::io::stdout(),
                    &user_facing_errors::UnknownError::from_panic_payload(panic),
                )
                .expect("failed to write to stdout");
                std::process::exit(255);
            }
        }
    } else {
        let dml_loc = matches.value_of("datamodel_location").unwrap();
        let mut file = fs::File::open(&dml_loc).unwrap();

        let mut datamodel = String::new();
        file.read_to_string(&mut datamodel).unwrap();

        if matches.is_present("single_cmd") {
            let api = RpcApi::new_sync(&datamodel).unwrap();
            let response = api.handle().unwrap();

            println!("{}", response);
        } else {
            match RpcApi::new_async(&datamodel) {
                Ok(api) => api.start_server(),
                Err(Error::DatamodelError(errors)) => {
                    pretty_print_errors(errors, &datamodel);
                    std::process::exit(1);
                }
                Err(e) => {
                    serde_json::to_writer(std::io::stdout(), &api::render_error(e)).expect("failed to write to stdout");
                    std::process::exit(255);
                }
            }
        }
    }
}
