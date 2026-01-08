//! HTTP server module.

mod format;
mod handlers;
mod response;
mod routes;
mod state;

pub use routes::{create_router, run};
pub use state::AppState;
