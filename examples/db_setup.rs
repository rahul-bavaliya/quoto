// examples/db_setup.rs: build or update the database tables and add the starting rows
//
//     docker compose up -d        (start the database; see README)
//     cargo run --example db_setup
//
// Makes the tables match the Rust structs and adds any missing province (safe to run again;
// a rate someone changed is never put back). It reads the settings from your environment,
// or from the `.env` file in this folder. It needs a running database, so the tests and CI
// never run it.

use quoto::{DatabaseSettings, database, logging, seed_provinces};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    logging::init();
    dotenvy::dotenv().ok();

    let result = async {
        let settings = DatabaseSettings::from_env()?;
        let db = database::connect(&settings).await?;
        database::sync(&db).await?;
        seed_provinces(&db).await?;
        let _ = db.close().await;
        Ok::<_, quoto::QuoteError>(settings.describe())
    }
    .await;

    match result {
        Ok(place) => {
            println!("The database at {place} is up to date.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            error.log();
            println!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
