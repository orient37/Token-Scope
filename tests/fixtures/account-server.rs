// TEST ONLY: stdio protocol fixture. Never reads Codex credentials or the network.
use std::io::{self, BufRead, Write};
fn main() {
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.contains("\"method\":\"initialize\"") {
            println!("{{\"id\":1,\"result\":{{}}}}");
        } else if line.contains("\"method\":\"account/read\"") {
            assert!(line.contains("\"refreshToken\":false"));
            let path = std::env::var("TOKENSCOPE_ACCOUNT_FIXTURE_FILE").unwrap();
            println!("{}", std::fs::read_to_string(path).unwrap());
        } else if line.contains("\"method\":\"account/rateLimits/read\"") {
            println!("{{\"id\":2,\"result\":{{\"rateLimitsByLimitId\":{{}}}}}}");
        }
        io::stdout().flush().unwrap();
    }
}
