use axum::extract::State;
use axum::http::StatusCode;
use axum::{Json, Router};
use axum::body::Body;
use axum::routing::post;
use passphrasex_common::model::user::User;
use crate::AppData;
use crate::handlers::common::HandlerResponse;
use crate::handlers::password::PasswordController;

pub struct UserController {
    pub router: Router<AppData, Body>
}

impl UserController {
    pub fn new() -> Self {
        let router = Router::new()
            .route("/users", post(Self::create_user));

        let password_router = PasswordController::new().router;

        Self {
            router: router.merge(password_router)
        }
    }
    pub async fn create_user(State(state): State<AppData>, Json(payload): Json<User>) -> HandlerResponse {
        match state.user_service.create_user(payload).await {
            Ok(user) => HandlerResponse::new(StatusCode::CREATED, user),
            Err(err) => HandlerResponse::from(err)
        }
    }
}