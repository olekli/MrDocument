use std::fs::{self, File};
use std::io::{Write};
use std::path::PathBuf;
use std::process::Command;
use clap::Parser;

fn setup_mrdocument_service(install_path: PathBuf, api_key: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !install_path.exists() {
        fs::create_dir_all(&install_path)?;
    }

    let api_key_path = install_path.join(".openai-api-key");
    let mut api_key_file = File::create(&api_key_path)?;
    write!(api_key_file, "{}", std::fs::read_to_string(api_key)?)?;

    let home_dir = dirs::home_dir().ok_or("Could not determine home directory")?;

    let plist_filename = "com.drrust.mrdocument.plist";
    let plist_path = install_path.join(plist_filename);
    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
   "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.drrust.mrdocument</string>

    <key>ProgramArguments</key>
    <array>
        <string>{mrdocument_path}</string>
        <string>{install_path}</string>
    </array>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>

    <key>StandardErrorPath</key>
    <string>{log_path}</string>

    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
"#,
        mrdocument_path = home_dir.join(".cargo/bin/mrdocument").display(),
        install_path = install_path.display(),
        log_path = install_path.join("log").display(),
    );

    let mut plist_file = File::create(&plist_path)?;
    plist_file.write_all(plist_content.as_bytes())?;

    let launch_agents_dir = home_dir.join("Library/LaunchAgents");
    if !launch_agents_dir.exists() {
        fs::create_dir_all(&launch_agents_dir)?;
    }
    let destination_plist_path = launch_agents_dir.join(plist_filename);
    fs::rename(&plist_path, &destination_plist_path)?;

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            &destination_plist_path,
            fs::Permissions::from_mode(0o644),
        )?;
    }

    let output = Command::new("launchctl")
        .arg("load")
        .arg(&destination_plist_path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to load launchd service: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("Service com.drrust.mrdocument has been set up and launched.");

    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    install_path: String,
    #[arg(long)]
    api_key_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    setup_mrdocument_service(args.install_path.into(), args.api_key_path.into())
}
