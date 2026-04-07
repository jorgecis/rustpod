use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn generate_spec(
    bundle_dir: &Path,
    rootfs_path: &Path,
    cmd: &[String],
    netns_path: Option<&str>,
) -> Result<(), String> {
    // 1. Generate default spec using crun
    let output = Command::new("crun")
        .arg("spec")
        .current_dir(bundle_dir)
        .output()
        .map_err(|e| format!("Failed to run crun spec: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "crun spec error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let config_path = bundle_dir.join("config.json");
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config.json: {}", e))?;

    let mut spec: Value = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config.json: {}", e))?;

    // 2. Modify rootfs path
    if let Some(root) = spec.get_mut("root") {
        root["path"] = Value::String(rootfs_path.to_string_lossy().to_string());
        // Do not make it readonly for now to ease testing
        root["readonly"] = Value::Bool(false);
    }

    // 3. Modify command args
    if let Some(process) = spec.get_mut("process") {
        if !cmd.is_empty() {
            let args_vec: Vec<Value> = cmd.iter().map(|s| Value::String(s.clone())).collect();
            process["args"] = Value::Array(args_vec);
        }
        process["terminal"] = Value::Bool(true); // Assuming interactive
    }

    // 4. Modify network namespace if provided
    if let Some(netns) = netns_path
        && let Some(linux) = spec.get_mut("linux")
            && let Some(namespaces) = linux.get_mut("namespaces").and_then(|n| n.as_array_mut()) {
                // Find network namespace and update its path
                let mut found_net = false;
                for ns in namespaces.iter_mut() {
                    if ns.get("type").and_then(|t| t.as_str()) == Some("network") {
                        ns["path"] = Value::String(netns.to_string());
                        found_net = true;
                        break;
                    }
                }

                if !found_net {
                    let mut new_ns = serde_json::Map::new();
                    new_ns.insert("type".to_string(), Value::String("network".to_string()));
                    new_ns.insert("path".to_string(), Value::String(netns.to_string()));
                    namespaces.push(Value::Object(new_ns));
                }
            }

    // Write back
    let new_config_content = serde_json::to_string_pretty(&spec)
        .map_err(|e| format!("Failed to serialize modified config: {}", e))?;
    fs::write(&config_path, new_config_content)
        .map_err(|e| format!("Failed to write modified config.json: {}", e))?;

    Ok(())
}

pub fn run_container(bundle_dir: &Path, container_id: &str) -> Result<(), String> {
    // Run container interactively
    let mut child = Command::new("crun")
        .arg("run")
        .arg(container_id)
        .current_dir(bundle_dir)
        .spawn()
        .map_err(|e| format!("Failed to start crun: {}", e))?;

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for crun: {}", e))?;
    if !status.success() {
        return Err(format!("crun exited with status: {}", status));
    }

    Ok(())
}

pub fn list_containers() -> Result<(), String> {
    let output = Command::new("crun")
        .arg("list")
        .output()
        .map_err(|e| format!("Failed to run crun list: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "crun list error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Pass strictly through output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

pub fn start_containers(containers: &[String]) -> Result<(), String> {
    for container in containers {
        let status = Command::new("crun")
            .arg("start")
            .arg(container)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            eprintln!("Failed to start container {}", container);
        } else {
            println!("Started {}", container);
        }
    }
    Ok(())
}

pub fn stop_containers(containers: &[String]) -> Result<(), String> {
    for container in containers {
        let status = Command::new("crun")
            .arg("kill")
            .arg(container)
            .arg("SIGTERM")
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            eprintln!("Failed to stop container {}", container);
        } else {
            println!("Stopped {}", container);
        }
    }
    Ok(())
}

pub fn rm_containers(containers: &[String]) -> Result<(), String> {
    for container in containers {
        let status = Command::new("crun")
            .arg("delete")
            .arg("-f")
            .arg(container)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            eprintln!("Failed to remove container {}", container);
        } else {
            let bundle_dir = std::path::Path::new("/run/rustpod/bundles").join(container);
            let _ = std::fs::remove_dir_all(&bundle_dir);
            // Optionally delete netns here
            println!("Removed {}", container);
        }
    }
    Ok(())
}

pub fn exec_container(container: &str, cmd: &[String]) -> Result<(), String> {
    let mut child = Command::new("crun")
        .arg("exec")
        .arg("-t")
        .arg(container)
        .args(cmd)
        .spawn()
        .map_err(|e| format!("Failed to start crun exec: {}", e))?;

    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for crun: {}", e))?;
    if !status.success() {
        return Err(format!("crun exec exited with status: {}", status));
    }

    Ok(())
}

pub fn logs_container(container: &str) -> Result<(), String> {
    println!(
        "Logs for container {}: (Not implemented natively, requires conmon)",
        container
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mock_spec_modification() {
        // We can't safely test actual crun executable in minimal CI, but we ensure module compiles
    }
}
