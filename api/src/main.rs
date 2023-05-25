mod model;
mod handlers;
mod service;

use model::common::DatabaseConfig;

use service::user::UserService;
use service::password::PasswordService;

use handlers::middleware::auth::only_user;
use handlers::user::create_user;
use handlers::password::add_password;

use axum::{routing::{post}, Router, middleware};
use axum::routing::get;
use crate::handlers::password::list_passwords;

#[derive(Clone)]
pub struct AppData {
    user_service: UserService,
    password_service: PasswordService
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new()
        .route("/users", post(create_user))
        .route("/users/:user_id/passwords", post(add_password))
        .route("/users/:user_id/passwords", get(list_passwords))
            .route_layer(middleware::from_fn(only_user));

    let client = DatabaseConfig::new()
        .into_client()
        .await
        .expect("Failed to connect to database");

    let app = app.with_state(AppData {
        user_service: UserService::new(&client),
        password_service: PasswordService::new(&client)
    });

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().expect("Failed to parse address"))
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}
