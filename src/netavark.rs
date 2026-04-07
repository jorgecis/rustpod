use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Serialize)]
pub struct NetavarkConfig {
    pub container_id: String,
    pub container_name: String,
    pub port_mappings: Vec<PortMapping>,
    pub networks: std::collections::HashMap<String, NetworkOptions>,
    pub network_info: std::collections::HashMap<String, NetworkInfo>,
}

#[derive(Serialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

#[derive(Serialize)]
pub struct NetworkOptions {
    pub interface_name: String,
}

#[derive(Serialize)]
pub struct NetworkInfo {
    pub name: String,
    pub driver: String,
    pub network_interface: String,
    pub subnets: Vec<Subnet>,
}

#[derive(Serialize)]
pub struct Subnet {
    pub subnet: String,
    pub gateway: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct NetavarkResult {
    // Netavark returns details about the configured IPs, etc.
    pub podman: Option<NetworkStatus>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct NetworkStatus {
    pub interfaces: Option<std::collections::HashMap<String, InterfaceInfo>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct InterfaceInfo {
    pub subnets: Option<Vec<SubnetInfo>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SubnetInfo {
    pub ipnet: String,
}

pub fn setup_network(container_id: &str, netns_path: &str) -> Result<String, String> {
    let mut networks = std::collections::HashMap::new();
    networks.insert(
        "rustpod-net".to_string(),
        NetworkOptions {
            interface_name: "eth0".to_string(),
        },
    );

    let mut network_info = std::collections::HashMap::new();
    network_info.insert(
        "rustpod-net".to_string(),
        NetworkInfo {
            name: "rustpod-net".to_string(),
            driver: "bridge".to_string(),
            network_interface: "rustpod0".to_string(),
            subnets: vec![Subnet {
                subnet: "10.88.0.0/16".to_string(),
                gateway: "10.88.0.1".to_string(),
            }],
        },
    );

    let config = NetavarkConfig {
        container_id: container_id.to_string(),
        container_name: container_id.to_string(),
        port_mappings: vec![],
        networks,
        network_info,
    };

    let config_json = serde_json::to_string(&config).map_err(|e| e.to_string())?;

    // the config dir for netavark is typically /var/lib/netavark or /etc/cni/net.d
    // the command: netavark setup <config-dir> <netns>
    let mut child = Command::new("netavark")
        .arg("setup")
        .arg("/var/lib/rustpod/netavark")
        .arg(netns_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn netavark: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(config_json.as_bytes())
            .map_err(|e| format!("Failed to write to netavark stdin: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for netavark: {}", e))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Netavark error: {}", err_str));
    }

    let out_str = String::from_utf8_lossy(&output.stdout);
    Ok(out_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = NetavarkConfig {
            container_id: "test-id".to_string(),
            container_name: "test-name".to_string(),
            port_mappings: vec![],
            networks: std::collections::HashMap::new(),
            network_info: std::collections::HashMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-id"));
    }
}
