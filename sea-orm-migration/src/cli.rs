use std::future::Future;

use clap::Parser;
use dotenvy::dotenv;
use std::{error::Error, fmt::Display, process::exit};
use tracing_subscriber::{EnvFilter, prelude::*};

use sea_orm::{ConnectOptions, Database, DbConn, DbErr};
use sea_orm_cli::{MigrateSubcommands, run_migrate_generate, run_migrate_init};

use super::MigratorTrait;

const MIGRATION_DIR: &str = "./";

pub async fn run_cli<M>(migrator: M)
where
    M: MigratorTrait,
{
    run_cli_with_connection(migrator, Database::connect).await;
}

/// Same as [`run_cli`] where you provide the function to create the [`DbConn`].
///
/// This allows configuring the database connection as you see fit.
/// E.g. you can change settings in [`ConnectOptions`] or you can load sqlite
/// extensions.
pub async fn run_cli_with_connection<M, F, Fut>(migrator: M, make_connection: F)
where
    M: MigratorTrait,
    F: FnOnce(ConnectOptions) -> Fut,
    Fut: Future<Output = Result<DbConn, DbErr>>,
{
    dotenv().ok();
    let cli = Cli::parse();

    let url = cli
        .database_url
        .expect("Environment variable 'DATABASE_URL' not set");
    let schema = cli.database_schema.unwrap_or_else(|| "public".to_owned());

    let connect_options = ConnectOptions::new(url)
        .set_schema_search_path(schema)
        .to_owned();

    let db = make_connection(connect_options)
        .await
        .expect("Fail to acquire database connection");

    run_migrate(migrator, &db, cli.command, cli.verbose)
        .await
        .unwrap_or_else(handle_error);
}

pub async fn run_migrate<M>(
    _: M,
    db: &DbConn,
    command: Option<MigrateSubcommands>,
    verbose: bool,
) -> Result<(), Box<dyn Error>>
where
    M: MigratorTrait,
{
    let filter = match verbose {
        true => "debug",
        false => "sea_orm_migration=info",
    };

    let filter_layer = EnvFilter::try_new(filter).unwrap();

    if verbose {
        let fmt_layer = tracing_subscriber::fmt::layer();
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init()
    } else {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_level(false)
            .without_time();
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init()
    };

    match command {
        Some(MigrateSubcommands::Fresh) => M::fresh(db).await?,
        Some(MigrateSubcommands::Refresh) => M::refresh(db).await?,
        Some(MigrateSubcommands::Reset) => M::reset(db).await?,
        Some(MigrateSubcommands::Status) => M::status(db).await?,
        Some(MigrateSubcommands::Up { num }) => M::up(db, num).await?,
        Some(MigrateSubcommands::Down { num }) => M::down(db, Some(num)).await?,
        Some(MigrateSubcommands::Init) => run_migrate_init(MIGRATION_DIR)?,
        Some(MigrateSubcommands::Generate {
            migration_name,
            universal_time: _,
            local_time,
        }) => run_migrate_generate(MIGRATION_DIR, &migration_name, !local_time)?,
        _ => M::up(db, None).await?,
    };

    Ok(())
}

#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[arg(short = 'v', long, global = true, help = "Show debug messages")]
    verbose: bool,

    #[arg(
        global = true,
        short = 's',
        long,
        env = "DATABASE_SCHEMA",
        long_help = "Database schema\n \
                    - For MySQL and SQLite, this argument is ignored.\n \
                    - For PostgreSQL, this argument is optional with default value 'public'.\n"
    )]
    database_schema: Option<String>,

    #[arg(
        global = true,
        short = 'u',
        long,
        env = "DATABASE_URL",
        help = "Database URL"
    )]
    database_url: Option<String>,

    #[command(subcommand)]
    command: Option<MigrateSubcommands>,
}

fn handle_error<E>(error: E)
where
    E: Display,
{
    eprintln!("{error}");
    exit(1);
}
