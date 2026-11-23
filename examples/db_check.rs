// examples/db_check.rs: check that Quoto can reach PostgreSQL
//
//     docker compose up -d        (start the database; see README)
//     cargo run --example db_check
//
// It reads the settings from your environment, or from the `.env` file in this
// folder (real environment variables win over `.env`). It needs a running
// database, so the tests and CI never run it.

use quoto::{DatabaseSettings, database};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // Load `.env` if there is one. No file is fine: the settings may come from the environment.
    dotenvy::dotenv().ok();

    let result = async {
        let settings = DatabaseSettings::from_env()?;
        let db = database::connect(&settings).await?;
        let _ = db.close().await;
        Ok::<_, quoto::AppError>(settings.describe())
    }
    .await;

    match result {
        Ok(place) => {
            println!("Connected to the database at {place}.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            error.log();
            println!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
