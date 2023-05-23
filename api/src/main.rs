mod model;
mod handlers;

use model::common::DatabaseConfig;
use handlers::user::create_user;
use axum::{routing::{post}, Router};
use crate::handlers::password::add_password;

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new()
        .route("/users", post(create_user))
        .route("/users/:user_id/passwords", post(add_password));

    let db_config = DatabaseConfig::new();
    let client = db_config.into_client().await.unwrap();
    let app = app.with_state(client);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().expect("Failed to parse address"))
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}
