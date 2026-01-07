//! HTTP server module.

mod handlers;
mod routes;
mod state;

pub use routes::{create_router, run};
pub use state::AppState;
