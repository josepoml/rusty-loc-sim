use std::{
    io::{BufRead, Write},
    path::PathBuf,
    sync::Arc,
};

use rusty_loc_sim::device::Device;
use std::env;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let mut dynamic_path = env::current_exe().unwrap();
    dynamic_path.pop();
    let wintun_path = dynamic_path.join("wintun.dll");

    // --- CLI Banner ---
    println!(
        r#"
    ____             __              __                    _____ _         
   / __ \__  _______/ /___  __      / /   ____  _____     / ___/(_)___ ___ 
  / /_/ / / / / ___/ __/ / / /_____/ /   / __ \/ ___/_____\__ \/ / __ `__ \
 / _, _/ /_/ (__  ) /_/ /_/ /_____/ /___/ /_/ / /__/_____/__/ / / / / / / /
/_/ |_|\__,_/____/\__/\__, /     /_____/\____/\___/     /____/_/_/ /_/ /_/ 
                     /____/                                                
                                                                        
rustymobiledevice CLI
Commands:
  connect                Connect to device
  reveal-developer-mode  Reveals Ios developer mode
  simulate-location -lat <latitude> -lng <longitude>
                         Simulate device location
  exit | quit            Exit the CLI
"#
    );
    let stdin = std::io::stdin();
    let mut device = Device::new();

    let termination_token = Arc::new(RwLock::new(false));

    loop {
        print!("$> ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        if stdin.lock().read_line(&mut input).is_err() {
            println!("Failed to read input");
            continue;
        }
        if *(termination_token.read().await) {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        let mut parts = input.split_whitespace();
        let command = parts.next().unwrap_or("");

        match command {
            "connect" => match device.connect(wintun_path.clone()).await {
                Ok(handle_tuple) => {
                    let termination_token_clone = termination_token.clone();
                    tokio::spawn(async move {
                        let (handle1, handle2, handle3) = handle_tuple;

                        tokio::select! {
                            res1 = handle1 => {
                                if let Err(e) = res1 {

                                    if e.is_panic() {
                                         let mut tt_writer = termination_token_clone.write().await;
                                         *tt_writer = true;
                                    }
                                }
                            }
                            res2 = handle2 => {
                                if let Err(e) = res2 {

                                    if e.is_panic() {
                                        let mut tt_writer = termination_token_clone.write().await;
                                        *tt_writer = true;
                                    }
                                }
                            }
                            res3 = handle3 => {

                                if let Err(e) = res3 {
                                    if e.is_panic() {
                                        let mut tt_writer = termination_token_clone.write().await;
                                        *tt_writer = true;
                                    }
                                }
                            }
                        }
                    });
                    println!("Connected")
                }
                Err(error) => {
                    println!("{:?}", error)
                }
            },
            "simulate-location" => {
                let mut lat = None;
                let mut lng = None;
                let mut args = parts.peekable();
                while let Some(arg) = args.next() {
                    match arg {
                        "-lat" => {
                            if let Some(val) = args.next() {
                                lat = val.parse::<f64>().ok();
                            }
                        }
                        "-lng" => {
                            if let Some(val) = args.next() {
                                lng = val.parse::<f64>().ok();
                            }
                        }
                        _ => {}
                    }
                }
                if let (Some(lat), Some(lng)) = (lat, lng) {
                    device.simulate_location(lat, lng).await.unwrap();
                    println!("Operation completed")
                }
            }
            "reveal-developer-mode" => match device.reveal_developer_mode().await {
                Ok(_) => {
                    println!("Operation completed")
                }
                Err(err) => {
                    println!("{}", err)
                }
            },

            "exit" | "quit" => break,
            _ => println!("Unknown command"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, time::Duration};

    use super::*;
    use std::io::{self, BufRead};
    use tokio;

    #[tokio::test]
    async fn test_device() {}
}
