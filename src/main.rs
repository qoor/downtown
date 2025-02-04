// Copyright 2023. The downtown authors all rights reserved.

use dotenvy::dotenv;
use downtown::{config::Config, env::get_env_or_panic};
use sqlx::mysql::MySqlPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "downtown=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_file(true).with_line_number(true))
        .init();

    // Load environment variables from '.env' file
    dotenv().ok();

    println!("Starting the server...");
    println!();

    let pool = match MySqlPoolOptions::new().connect(&get_env_or_panic("DATABASE_URL")).await {
        Ok(pool) => {
            println!("Connection to the database is successful.");
            pool
        }
        Err(err) => {
            println!("Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    sqlx::migrate!().run(&pool).await.unwrap_or_else(|err| {
        println!("Failed to migrate database: {:?}", err);
        std::process::exit(1);
    });

    let config = Config::new();
    let address = config.address().to_string();
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    let app = downtown::app(config, &pool).await;

    print_server_started(&address);

    axum::serve(listener, app).await.unwrap();
}

fn print_server_started(address: &str) {
    println!();
    print!("{}", downtown::about());
    println!("Server started successfully. (address: {})", address);
}
