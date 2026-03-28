use std::io::{self, Write};
use lfi_vsa_core::agent::LfiAgent;

fn main() {
    println!("============================================================");
    println!(" LFI Sovereign Intelligence — Sovereign Cognitive Agent Terminal");
    println!(" Type 'exit' or 'quit' to terminate the session.");
    println!("============================================================\n");

    let mut agent = match LfiAgent::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to initialize Sovereign Agent: {:?}", e);
            return;
        }
    };

    // --- SECURE LOGIN CHALLENGE ---
    print!("Sovereign Identity Verification Required.\nEnter Password> ");
    let _ = io::stdout().flush();
    let mut password = String::new();
    if io::stdin().read_line(&mut password).is_err() {
        println!("Authentication Fault.");
        return;
    }
    let password = password.trim();
    
    if agent.authenticate(password) {
        println!("LFI> [IDENTITY VERIFIED] Access to higher functions granted.\n");
    } else {
        println!("LFI> [AUTHENTICATION FAILURE] Restricted mode active. Internal reasoning gated.\n");
    }

    loop {
        print!("User> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Error reading input.");
            break;
        }

        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("LFI> Goodbye. Terminating sovereign session.");
            break;
        }

        if input.is_empty() {
            continue;
        }

        match agent.chat(input) {
            Ok(response) => {
                println!("LFI> {}", response);
            }
            Err(e) => {
                println!("LFI> [Cognitive Fault] {:?}", e);
            }
        }
        println!();
    }
}
