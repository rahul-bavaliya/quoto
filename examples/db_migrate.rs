// examples/db_migrate.rs: build or update the database tables
//
//     docker compose up -d        (start the database; see README)
//     cargo run --example db_migrate
//
// Runs every file in `migrations/` that hasn't run yet (safe to run again).
// It reads the settings from your environment, or from the `.env` file in this
// folder. It needs a running database, so the tests and CI never run it.

use quoto::{DatabaseSettings, database, logging};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    logging::init();
    dotenvy::dotenv().ok();

    let result = async {
        let settings = DatabaseSettings::from_env()?;
        let pool = database::connect(&settings).await?;
        database::migrate(&pool).await?;
        pool.close().await;
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
