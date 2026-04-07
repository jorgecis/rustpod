use clap::Parser;

mod cli;
mod crun;
mod netavark;
mod registry;

use cli::{Cli, Commands, ContainerCommands, ImageCommands};

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Pull { image, tag } => {
            println!("Pulling image {}:{}", image, tag);
            let _ = registry::pull_image(image, tag).map_err(|e| {
                eprintln!("{}", e);
                std::process::exit(1);
            });
        }
        Commands::Run { image, cmd, .. } => {
            let id = format!(
                "rustpod-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );
            let _ = run_container(&id, image, cmd).map_err(|e| {
                eprintln!("{}", e);
                std::process::exit(1);
            });
        }
        Commands::Ps => {
            let _ = crun::list_containers().map_err(|e| eprintln!("{}", e));
        }
        Commands::Images => {
            let _ = registry::list_images().map_err(|e| eprintln!("{}", e));
        }
        Commands::Rm { containers } => {
            let _ = crun::rm_containers(containers).map_err(|e| eprintln!("{}", e));
        }
        Commands::Rmi { images } => {
            let _ = registry::remove_images(images).map_err(|e| eprintln!("{}", e));
        }
        Commands::Login { server } => {
            let _ = registry::login(server.clone()).map_err(|e| eprintln!("{}", e));
        }
        Commands::Logout { server } => {
            let _ = registry::logout(server.clone()).map_err(|e| eprintln!("{}", e));
        }
        Commands::Exec { container, cmd } => {
            let _ = crun::exec_container(container, cmd).map_err(|e| eprintln!("{}", e));
        }
        Commands::Logs { container } => {
            let _ = crun::logs_container(container).map_err(|e| eprintln!("{}", e));
        }
        Commands::Start { containers } => {
            let _ = crun::start_containers(containers).map_err(|e| eprintln!("{}", e));
        }
        Commands::Stop { containers } => {
            let _ = crun::stop_containers(containers).map_err(|e| eprintln!("{}", e));
        }
        Commands::Image { cmd } => match cmd {
            ImageCommands::Pull { image, tag } => {
                let _ = registry::pull_image(image, tag).map_err(|e| eprintln!("{}", e));
            }
            ImageCommands::Ls => {
                let _ = registry::list_images().map_err(|e| eprintln!("{}", e));
            }
            ImageCommands::Rm { images } => {
                let _ = registry::remove_images(images).map_err(|e| eprintln!("{}", e));
            }
            _ => println!("Not implemented yet: {:?}", cmd),
        },
        Commands::Container { cmd } => match cmd {
            ContainerCommands::Run {
                image,
                cmd: run_cmd,
                ..
            } => {
                let id = format!(
                    "rustpod-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                );
                let _ = run_container(&id, image, run_cmd).map_err(|e| eprintln!("{}", e));
            }
            ContainerCommands::Ls => {
                let _ = crun::list_containers().map_err(|e| eprintln!("{}", e));
            }
            ContainerCommands::Rm { containers } => {
                let _ = crun::rm_containers(containers).map_err(|e| eprintln!("{}", e));
            }
            ContainerCommands::Start { containers } => {
                let _ = crun::start_containers(containers).map_err(|e| eprintln!("{}", e));
            }
            ContainerCommands::Stop { containers } => {
                let _ = crun::stop_containers(containers).map_err(|e| eprintln!("{}", e));
            }
            ContainerCommands::Exec { container, cmd } => {
                let _ = crun::exec_container(container, cmd).map_err(|e| eprintln!("{}", e));
            }
            ContainerCommands::Logs { container } => {
                let _ = crun::logs_container(container).map_err(|e| eprintln!("{}", e));
            }
            _ => println!("Not implemented yet: {:?}", cmd),
        },
        _ => println!("Not implemented yet: {:?}", cli.command),
    }
}

fn run_container(id: &str, image: &str, cmd: &[String]) -> Result<(), String> {
    // 1. Verify and resolve image
    let tag = "latest"; // Simplified for now
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let rootfs_path = std::path::Path::new(&home)
        .join(".rustpod")
        .join("images")
        .join(image)
        .join(tag);

    if !rootfs_path.exists() {
        println!("Image not found locally, pulling...");
        registry::pull_image(image, tag)?;
    }

    // 2. Prepare bundle directory
    let bundle_dir = std::path::Path::new("/run/rustpod/bundles").join(id);
    std::fs::create_dir_all(&bundle_dir)
        .map_err(|e| format!("Failed to create bundle dir: {}", e))?;

    // 3. Create Network Namespace
    let netns_name = format!("rustpod-{}", id);
    let netns_path = format!("/var/run/netns/{}", netns_name);

    // We try to add the netns, ignore error if it exists
    let _ = std::process::Command::new("ip")
        .arg("netns")
        .arg("add")
        .arg(&netns_name)
        .output();

    // 4. Setup Network with Netavark
    println!("Setting up network...");
    let mut actual_netns: Option<String> = None;
    match netavark::setup_network(id, &netns_path) {
        Ok(netavark_result) => {
            println!("Network configured: {}", netavark_result);
            actual_netns = Some(netns_path.clone());
        }
        Err(e) => {
            println!(
                "Warning: Skipping custom networking (using host). Reason: {}",
                e
            );
            // Clean up the unused netns right away manually
            let _ = std::process::Command::new("ip")
                .arg("netns")
                .arg("delete")
                .arg(&netns_name)
                .output();
        }
    }

    // 5. Generate OCI Spec
    println!("Generating OCI spec...");
    crun::generate_spec(&bundle_dir, &rootfs_path, cmd, actual_netns.as_deref())?;

    // 6. Run container using crun
    println!("Running container: {}", id);
    let run_res = crun::run_container(&bundle_dir, id);

    // 7. Cleanup Network
    // Note: A full implementation would call `netavark teardown` and remove the netns.
    println!("Cleaning up...");
    if actual_netns.is_some() {
        let _ = std::process::Command::new("ip")
            .arg("netns")
            .arg("delete")
            .arg(&netns_name)
            .output();
    }
    let _ = std::fs::remove_dir_all(&bundle_dir);

    run_res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
