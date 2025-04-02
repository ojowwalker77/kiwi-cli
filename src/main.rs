use log::{info, error};
use dialoguer::{Input, Password, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use dotenv::dotenv;
use std::env;
use clap::Parser;
use serde_json::json;
use std::process;

use kiwi::{Result, Config, Cli};

const DEFAULT_SYNC_URL: &str = "http://34.41.188.73:8080";
const MAX_LOGIN_ATTEMPTS: u32 = 3;

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

async fn authenticate(theme: &ColorfulTheme) -> Result<AuthResponse> {
    let mut attempts = 0;
    let mut last_email = String::new();
    
    loop {
        if attempts >= MAX_LOGIN_ATTEMPTS {
            println!("\n❌ Maximum login attempts exceeded. Please try again later.");
            process::exit(1);
        }

        let email = if attempts == 0 {
            Input::with_theme(theme)
                .with_prompt("Email")
                .validate_with(|input: &String| -> std::result::Result<(), &str> {
                    if !input.contains('@') {
                        return Err("Please enter a valid email address");
                    }
                    Ok(())
                })
                .interact()
                .map_err(|e| format!("Failed to read email: {}", e))?
        } else {
            Input::with_theme(theme)
                .with_prompt("Email")
                .default(last_email.clone())
                .interact()
                .map_err(|e| format!("Failed to read email: {}", e))?
        };

        last_email = email.clone();

        let password: String = if attempts == 0 {
            Password::with_theme(theme)
                .with_prompt("Password")
                .with_confirmation("Confirm password", "Passwords don't match")
                .validate_with(|input: &String| -> std::result::Result<(), &str> {
                    if input.len() < 8 {
                        return Err("Password must be at least 8 characters long");
                    }
                    Ok(())
                })
                .interact()
                .map_err(|e| format!("Failed to read password: {}", e))?
        } else {
            Password::with_theme(theme)
                .with_prompt("Password")
                .interact()
                .map_err(|e| format!("Failed to read password: {}", e))?
        };

        // Try to login first
        match login_user(email.clone(), password.clone()).await {
            Ok(auth) => {
                println!("\n✨ Welcome back!");
                return Ok(auth);
            }
            Err(_) => {
                if attempts == 0 {
                    // First attempt, try to register
                    println!("\nAttempting to create new account...");
                    match register_user(email.clone(), password).await {
                        Ok(auth) => {
                            println!("\n✨ Account created successfully!");
                            return Ok(auth);
                        }
                        Err(e) => {
                            if e.to_string().contains("User already exists") {
                                println!("\n❌ Account exists but password is incorrect.");
                                println!("Please try logging in again with the correct password.");
                            } else {
                                error!("Failed to create account: {}", e);
                                process::exit(1);
                            }
                        }
                    }
                } else {
                    println!("\n❌ Login failed: Invalid email or password.");
                    println!("Attempts remaining: {}", MAX_LOGIN_ATTEMPTS - attempts - 1);
                }
            }
        }
        
        attempts += 1;
    }
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
    
    // Handle authentication
    match authenticate(&theme).await {
        Ok(auth) => {
            // Set up sync configuration
            config.sync_token = Some(auth.token.clone());
            
            // Initialize user's remote storage
            let client = Client::new();
            let _ = client
                .post(format!("{}/sync", config.sync_url.as_deref().unwrap_or(DEFAULT_SYNC_URL)))
                .header("Authorization", format!("Bearer {}", auth.token))
                .json(&json!({
                    "files": {},
                    "packages": []
                }))
                .send()
                .await?;

            config.save()?;
        }
        Err(e) => {
            error!("Authentication failed: {}", e);
            process::exit(1);
        }
    }

    // After successful login/registration, execute the CLI command
    let cli = Cli::parse();
    cli.execute().await
}
