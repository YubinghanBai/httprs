use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use httprs::{build_client, execute_request, Cli};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let args = cli.command.args();

    let client = build_client(args)?;

    if let Err(e) = execute_request(&cli, &client).await {
        eprintln!("\n{} {}\n", "Error:".red().bold(), e);

        let error_msg = e.to_string();

        if error_msg.contains("dns error") || error_msg.contains("failed to lookup") {
            eprintln!("{}", "💡 Possible causes:".yellow());
            eprintln!("   - Check if the domain name is correct");
            eprintln!("   - Check your network connection");
            eprintln!("   - Try using IP address instead");
        } else if error_msg.contains("timed out") {
            eprintln!("{}", "💡 Suggestion:".yellow());
            eprintln!("   - Increase timeout with --timeout <seconds>");
            eprintln!("   - Check if the server is responsive");
        } else if error_msg.contains("connection refused") {
            eprintln!("{}", "💡 Possible causes:".yellow());
            eprintln!("   - Server is not running");
            eprintln!("   - Wrong port number");
            eprintln!("   - Firewall blocking the connection");
        } else if error_msg.contains("No such file") {
            eprintln!("{}", "💡 File not found:".yellow());
            eprintln!("   - Check if the file path is correct");
            eprintln!("   - Use absolute path or relative to current directory");
        }
        std::process::exit(1);
    };

    Ok(())
}
