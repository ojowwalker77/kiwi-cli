use log::{info, error};
use dialoguer::{Input, Password, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use dotenv::dotenv;
use std::env;
use clap::Parser;

use kiwi::{Result, Config, Cli};

#[derive(Debug, Serialize, Deserialize)]
struct RegisterRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    email: String,
    token: String,
}

async fn register_user(email: String, password: String) -> Result<AuthResponse> {
    let client = Client::new();
    let request = RegisterRequest { email, password };
    
    let response = client
        .post("http://34.41.188.73:8080/register")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Registration failed: {} - {}", status, error_text).into());
    }

    let auth_response = response.json::<AuthResponse>().await?;
    Ok(auth_response)
}

async fn login_user(email: String, password: String) -> Result<AuthResponse> {
    let client = Client::new();
    let request = RegisterRequest { email, password };
    
    let response = client
        .post("http://34.41.188.73:8080/login")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Login failed: {} - {}", status, error_text).into());
    }

    let auth_response = response.json::<AuthResponse>().await?;
    Ok(auth_response)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv().ok();
    
    let mut config = Config::load()?;
    if config.sync_token.is_some() {
        let cli = Cli::parse();
        return cli.execute().await;
    }
    
    println!("Welcome to Kiwi! 🥝");
    println!("Please log in or create a new account.\n");

    let theme = ColorfulTheme::default();
    
    let email: String = Input::with_theme(&theme)
        .with_prompt("Email")
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if !input.contains('@') {
                return Err("Please enter a valid email address");
            }
            Ok(())
        })
        .interact()
        .map_err(|e| format!("Failed to read email: {}", e))?;

    let password: String = Password::with_theme(&theme)
        .with_prompt("Password")
        .with_confirmation("Confirm password", "Passwords don't match")
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.len() < 8 {
                return Err("Password must be at least 8 characters long");
            }
            Ok(())
        })
        .interact()
        .map_err(|e| format!("Failed to read password: {}", e))?;

    match login_user(email.clone(), password.clone()).await {
        Ok(auth) => {
            println!("\n✨ Welcome back!");
            config.sync_token = Some(auth.token);
            config.save()?;
        }
        Err(_) => {
            println!("\nNo account found. Creating new account...");
            match register_user(email, password).await {
                Ok(auth) => {
                    println!("\n✨ Account created successfully!");
                    config.sync_token = Some(auth.token);
                    config.save()?;
                }
                Err(e) => {
                    error!("Failed to create account: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    let cli = Cli::parse();
    cli.execute().await
}
