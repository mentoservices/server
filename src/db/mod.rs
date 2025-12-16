use mongodb::{Client, Database};
use rocket::{Rocket, Build};
use rocket::fairing::AdHoc;

pub fn init() -> AdHoc {
    AdHoc::on_ignite("MongoDB", |rocket| async {
        match connect().await {
            Ok(database) => {
                info!("✓ MongoDB connected successfully");
                rocket.manage(database)
            }
            Err(e) => {
                error!("✗ Failed to connect to MongoDB: {}", e);
                rocket
            }
        }
    })
}

async fn connect() -> Result<Database, mongodb::error::Error> {
    let uri = crate::config::Config::mongodb_uri();
    let client = Client::with_uri_str(&uri).await?;
    
    // Test connection
    client
        .database("admin")
        .run_command(mongodb::bson::doc! {"ping": 1}, None)
        .await?;
    
    Ok(client.database("mento-services"))
}

pub type DbConn = Database;