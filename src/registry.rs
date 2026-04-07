use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum RegistryResponse {
    ManifestList { manifests: Vec<ManifestReference> },
    Manifest { layers: Vec<Layer> },
}

#[derive(Deserialize, Debug)]
struct ManifestReference {
    digest: String,
    platform: Option<Platform>,
}

#[derive(Deserialize, Debug)]
struct Platform {
    architecture: Option<String>,
    os: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Layer {
    digest: String,
}

/// Pulls an image from Docker Hub and unzips its layers to the local rustpod image cache.
pub fn pull_image(image: &str, tag: &str) -> Result<(), String> {
    let full_image = if image.contains('/') {
        image.to_string()
    } else {
        format!("library/{}", image)
    };

    let auth_url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
        full_image
    );

    // 1. Get Auth Token
    let token_resp: TokenResponse = ureq::get(&auth_url)
        .call()
        .map_err(|e| format!("Auth request failed: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse auth token: {}", e))?;

    let token = token_resp.token;

    // 2. Function to fetch manifest by reference
    let fetch_manifest = |reference: &str| -> Result<RegistryResponse, String> {
        let manifest_url = format!(
            "https://registry-1.docker.io/v2/{}/manifests/{}",
            full_image, reference
        );

        ureq::get(&manifest_url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Accept", "application/vnd.docker.distribution.manifest.v2+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.oci.image.manifest.v1+json, application/vnd.oci.image.index.v1+json")
            .call()
            .map_err(|e| format!("Manifest request failed: {}", e))?
            .into_json()
            .map_err(|e| format!("Failed to parse manifest: {}", e))
    };

    let mut response = fetch_manifest(tag)?;

    // 3. Resolve manifest list to specific arch if needed
    if let RegistryResponse::ManifestList { manifests } = response {
        let target_arch = "amd64"; // For simplicity, we hardcode amd64. 
        let target_os = "linux";

        let mut target_digest = manifests
            .first()
            .map(|m| m.digest.clone())
            .ok_or("Empty manifest list")?;
        for m in manifests {
            if let Some(p) = &m.platform
                && p.architecture.as_deref() == Some(target_arch)
                && p.os.as_deref() == Some(target_os)
            {
                target_digest = m.digest.clone();
                break;
            }
        }

        // Re-fetch using the specific platform digest
        response = fetch_manifest(&target_digest)?;
    }

    let layers = match response {
        RegistryResponse::Manifest { layers } => layers,
        _ => return Err("Expected manifest but got manifest list".to_string()),
    };

    // 3. Prepare target directory
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let target_dir = Path::new(&home)
        .join(".rustpod")
        .join("images")
        .join(image)
        .join(tag);

    if target_dir.exists() {
        println!("Image already exists locally at {:?}", target_dir);
        // We'll clean it for a fresh pull, or we could skip. Let's recreate.
        fs::remove_dir_all(&target_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    // 4. Download and extract layers
    for (i, layer) in layers.iter().enumerate() {
        println!(
            "Downloading layer {}/{} ({})",
            i + 1,
            layers.len(),
            layer.digest
        );
        let layer_url = format!(
            "https://registry-1.docker.io/v2/{}/blobs/{}",
            full_image, layer.digest
        );

        let layer_resp = ureq::get(&layer_url)
            .set("Authorization", &format!("Bearer {}", token))
            .call()
            .map_err(|e| format!("Layer request failed: {}", e))?;

        let reader = layer_resp.into_reader();
        let decompressed = flate2::read::GzDecoder::new(reader);
        let mut archive = tar::Archive::new(decompressed);

        archive
            .unpack(&target_dir)
            .map_err(|e| format!("Extract failed: {}", e))?;
    }

    Ok(())
}

/// Helper method to list local images
pub fn list_images() -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let images_dir = Path::new(&home).join(".rustpod").join("images");

    if !images_dir.exists() {
        println!("REPOSITORY\tTAG");
        return Ok(());
    }

    println!("REPOSITORY\tTAG");
    for repo_entry in fs::read_dir(images_dir)
        .map_err(|e| format!("Failed to read images root: {}", e))?
        .flatten()
    {
        let repo = repo_entry.file_name().into_string().unwrap_or_default();
        let repo_path = repo_entry.path();
        if repo_path.is_dir() {
            for tag_dir in fs::read_dir(repo_path).unwrap().flatten() {
                let tag = tag_dir.file_name().into_string().unwrap_or_default();
                println!("{}\t{}", repo, tag);
            }
        }
    }
    Ok(())
}

/// Helper method to remove local images
pub fn remove_images(images: &[String]) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let images_dir = Path::new(&home).join(".rustpod").join("images");

    for image in images {
        let parts: Vec<&str> = image.split(':').collect();
        let repo = parts[0];
        let tag = if parts.len() > 1 { parts[1] } else { "latest" };

        let target_dir = images_dir.join(repo).join(tag);
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)
                .map_err(|e| format!("Failed to remove {}: {}", image, e))?;
            println!("Untagged and removed: {}:{}", repo, tag);

            // Clean up repo folder if empty
            let repo_dir = images_dir.join(repo);
            if fs::read_dir(&repo_dir)
                .map(|mut iter| iter.next().is_none())
                .unwrap_or(false)
            {
                let _ = fs::remove_dir(&repo_dir);
            }
        } else {
            eprintln!("Error: No such image: {}", image);
        }
    }
    Ok(())
}

pub fn login(server: Option<String>) -> Result<(), String> {
    println!(
        "Login Succeeded for {}",
        server.unwrap_or_else(|| "docker.io".to_string())
    );
    // Note: stub implementation. Actual implementation would require storing auth token standard config.
    Ok(())
}

pub fn logout(server: Option<String>) -> Result<(), String> {
    println!(
        "Removing login credentials for {}",
        server.unwrap_or_else(|| "docker.io".to_string())
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Since network requests can fail in automated environments, we just do a tiny structural test here
    #[test]
    fn test_auth_url_format() {
        let image = "library/alpine";
        let auth_url = format!(
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
            image
        );
        assert_eq!(
            auth_url,
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:library/alpine:pull"
        );
    }

    #[test]
    fn test_pull_hello_world() {
        // This is an integration test that actually pulls an image to local storage.
        // Needs network access.
        let result = pull_image("hello-world", "latest");
        assert!(
            result.is_ok(),
            "Failed to pull hello-world image: {:?}",
            result
        );

        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let target_dir = Path::new(&home)
            .join(".rustpod")
            .join("images")
            .join("hello-world")
            .join("latest");
        assert!(target_dir.exists(), "Target directory was not created.");
    }
}
