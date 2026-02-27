use dotenvy::dotenv;
use std::env;

pub struct Config {
    pub database_url: String,
    pub cgm_username: String,
    pub cgm_password: String,
    pub cgm_region: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok(); 
        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            cgm_username: env::var("CGM_USERNAME").expect("CGM_USERNAME must be set"),
            cgm_password: env::var("CGM_PASSWORD").expect("CGM_PASSWORD must be set"),
            cgm_region: env::var("CGM_REGION").unwrap_or_else(|_| "eu".to_string()),
        }
    }
}
