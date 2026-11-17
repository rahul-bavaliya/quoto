// examples/seaorm_province.rs: EXPERIMENT (not part of Quoto): can SeaORM build the
// `provinces` table from a Rust struct?
//
//     cargo run --example seaorm_province -- v1     # sync the struct as it is today
//     cargo run --example seaorm_province -- v2     # same struct + one new column
//
// Reads POSTGRES_* from the environment. Use a THROWAWAY database only.

use sea_orm::{Database, DatabaseConnection, DbErr};

/// The `provinces` table as a struct (version 1: the same columns the migrations make).
mod v1 {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "provinces")]
    pub struct Model {
        #[sea_orm(
            primary_key,
            auto_increment = false,
            default_expr = "Expr::cust(\"gen_random_uuid()\")"
        )]
        pub id: Uuid,
        #[sea_orm(unique)]
        pub code: String,
        #[sea_orm(unique)]
        pub name: String,
        pub country_code: String,
        pub gst_rate_thousandths: Option<i32>,
        pub pst_rate_thousandths: Option<i32>,
        pub hst_rate_thousandths: Option<i32>,
        pub effective_from: Date,
        pub note: String,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub created_at: DateTimeUtc,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Version 2: the same table plus one new column (to see what sync does with a change).
mod v2 {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "provinces")]
    pub struct Model {
        #[sea_orm(
            primary_key,
            auto_increment = false,
            default_expr = "Expr::cust(\"gen_random_uuid()\")"
        )]
        pub id: Uuid,
        #[sea_orm(unique)]
        pub code: String,
        #[sea_orm(unique)]
        pub name: String,
        pub country_code: String,
        pub gst_rate_thousandths: Option<i32>,
        pub pst_rate_thousandths: Option<i32>,
        pub hst_rate_thousandths: Option<i32>,
        pub effective_from: Date,
        pub note: String,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub created_at: DateTimeUtc,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub updated_at: DateTimeUtc,
        /// NEW in v2.
        pub display_order: Option<i32>,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

fn url() -> String {
    let get = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("set {k}"));
    format!(
        "postgres://{}:{}@{}:{}/{}",
        get("POSTGRES_USER"),
        get("POSTGRES_PASSWORD"),
        get("POSTGRES_HOST"),
        get("POSTGRES_PORT"),
        get("POSTGRES_DB")
    )
}

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    let which = std::env::args().nth(1).unwrap_or_else(|| "v1".into());
    let db: DatabaseConnection = Database::connect(url()).await?;
    let builder = db.get_schema_builder();
    let builder = match which.as_str() {
        "v1" => builder.register(v1::Entity),
        "v2" => builder.register(v2::Entity),
        other => panic!("unknown version {other}"),
    };
    builder.sync(&db).await?;
    println!("sync {which}: done");
    Ok(())
}
