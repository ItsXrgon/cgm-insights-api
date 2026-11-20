//! CGM Insights - Continuous Glucose Monitor Integration Server
//!
//! This library provides the core application logic for integrating with various
//! CGM devices and platforms.

pub mod config;
pub mod db;

pub use db::DbPool;
pub mod docs;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod scheduler;
pub mod server;
pub mod services;
pub mod telemetry;

pub use config::Config;
pub use error::AppError;
pub use server::create_app;
