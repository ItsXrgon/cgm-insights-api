use crate::middleware::cors_layer;
use axum::Router;
use sqlx::{Pool, Postgres};

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
}

pub fn create_app(db: Pool<Postgres>) -> Router {
    let state = AppState { db };

    Router::new()
        .merge(crate::handlers::health::routes())
        .merge(crate::handlers::api_info::routes())
        .merge(crate::handlers::glucose::routes())
        .merge(crate::handlers::sync::routes())
        .with_state(state)
        .layer(cors_layer())
}
