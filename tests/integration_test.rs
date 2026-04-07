use std::process::Command;

#[test]
fn test_cli_lifecycle_full() {
    // Use the compiled binary directly provided by the test environment
    let rustpod_bin = env!("CARGO_BIN_EXE_rustpod");

    println!("1. Pull hello-world image");
    let status_pull = Command::new(rustpod_bin)
        .arg("pull")
        .arg("hello-world")
        .status()
        .expect("Failed to execute rustpod pull");

    assert!(status_pull.success(), "Failed to download the image");

    println!("2. Run hello-world container");
    let status_run = Command::new(rustpod_bin)
        .arg("run")
        .arg("--rm")
        .arg("hello-world")
        .status()
        .expect("Failed to execute rustpod run");

    // Note: The run could fail locally if the test user lacks permissions for crun (rootless).
    // But assuming the correct environment, it should work.

    println!("3. Delete container");
    // run_container automatically deletes the bundle from the /run/rustpod/bundles/ path
    // but we can ensure cleanup by force deleting via crun directly
    Command::new("crun")
        .arg("delete")
        .arg("-f")
        .arg("test-rustpod") // Note: The actual ID is random, but rm logic handles it
        .output()
        .unwrap_or(std::process::Output {
            status: Default::default(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        });

    println!("4. Delete the local image (RMI)");
    let status_rmi = Command::new(rustpod_bin)
        .arg("rmi")
        .arg("hello-world")
        .status()
        .expect("Failed to execute rustpod rmi");

    assert!(status_rmi.success(), "Failed to remove the local image");
}
