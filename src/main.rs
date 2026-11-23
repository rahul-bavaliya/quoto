// src/main.rs: starts the web API (see `src/api.rs`). It is the one place where errors stop the
// program: it logs the code, prints the message, and exits with the error's exit code.

use quoto::api::{ApiSettings, serve};
use quoto::database::{self, DatabaseSettings};
use quoto::QuoteResult;
use std::process::ExitCode;

async fn run() -> QuoteResult<()> {
    // Reads `.env` if there is one (the real environment wins). A missing file is fine.
    dotenvy::dotenv().ok();
    let api = ApiSettings::from_env()?;
    let db = database::connect(&DatabaseSettings::from_env()?).await?;
    serve(api, db).await
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error.log();
            eprintln!("Quoto could not start: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
