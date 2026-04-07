use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "rustpod")]
#[command(about = "A minimal container engine in Rust", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pull an image from a registry
    Pull {
        /// The image name
        image: String,
        /// The image tag
        #[arg(short, long, default_value = "latest")]
        tag: String,
    },
    /// Run a command in a new container
    Run {
        #[arg(short = 'i', long)]
        interactive: bool,
        #[arg(short = 't', long)]
        tty: bool,
        #[arg(long)]
        rm: bool,
        /// The image to run
        image: String,
        /// The command to execute inside the container
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
    /// List containers (alias for `container ls`)
    Ps,
    /// List images in local storage (alias for `image ls`)
    Images,
    /// Remove one or more containers
    Rm {
        containers: Vec<String>,
    },
    /// Remove one or more images
    Rmi {
        images: Vec<String>,
    },
    /// Login to a container registry
    Login {
        server: Option<String>,
    },
    /// Logout of a container registry
    Logout {
        server: Option<String>,
    },
    /// Execute a command in a running container
    Exec {
        container: String,
        cmd: Vec<String>,
    },
    /// Fetch the logs of a container
    Logs {
        container: String,
    },
    /// Start one or more containers
    Start {
        containers: Vec<String>,
    },
    /// Stop one or more containers
    Stop {
        containers: Vec<String>,
    },
    /// Manage containers
    Container {
        #[command(subcommand)]
        cmd: ContainerCommands,
    },
    /// Manage images
    Image {
        #[command(subcommand)]
        cmd: ImageCommands,
    },
    /// Manage networks
    Network {
        #[command(subcommand)]
        cmd: NetworkCommands,
    },
    /// Manage volumes
    Volume {
        #[command(subcommand)]
        cmd: VolumeCommands,
    },
    /// Manage pods
    Pod {
        #[command(subcommand)]
        cmd: PodCommands,
    },
    /// Manage system
    System {
        #[command(subcommand)]
        cmd: SystemCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ContainerCommands {
    /// List containers
    Ls,
    /// Create but do not start a container
    Create { image: String },
    /// Run a command in a new container
    Run { 
        #[arg(short = 'i', long)]
        interactive: bool,
        #[arg(short = 't', long)]
        tty: bool,
        #[arg(long)]
        rm: bool,
        image: String, 
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String> 
    },
    /// Stop one or more containers
    Stop { containers: Vec<String> },
    /// Start one or more containers
    Start { containers: Vec<String> },
    /// Remove one or more containers
    Rm { containers: Vec<String> },
    /// Display the running processes of a container
    Top { container: String },
    /// View logs for a container
    Logs { container: String },
    /// Execute command in container
    Exec { container: String, cmd: Vec<String> },
    /// Inspect a container
    Inspect { container: String },
}

#[derive(Subcommand, Debug)]
pub enum ImageCommands {
    /// List images
    Ls,
    /// Pull an image
    Pull {
        image: String,
        #[arg(short, long, default_value = "latest")]
        tag: String,
    },
    /// Push an image
    Push { image: String },
    /// Remove an image
    Rm { images: Vec<String> },
    /// Build an image using instructions from Containerfile
    Build { context: String },
    /// Inspect an image
    Inspect { image: String },
    /// Show history of an image
    History { image: String },
}

#[derive(Subcommand, Debug)]
pub enum NetworkCommands {
    /// List networks
    Ls,
    /// Create a network
    Create { name: String },
    /// Remove a network
    Rm { networks: Vec<String> },
    /// Inspect a network
    Inspect { network: String },
}

#[derive(Subcommand, Debug)]
pub enum VolumeCommands {
    /// List volumes
    Ls,
    /// Create a volume
    Create { name: String },
    /// Remove a volume
    Rm { volumes: Vec<String> },
    /// Inspect a volume
    Inspect { volume: String },
}

#[derive(Subcommand, Debug)]
pub enum PodCommands {
    /// List pods
    Ls,
    /// Create a pod
    Create { name: String },
    /// Remove a pod
    Rm { pods: Vec<String> },
}

#[derive(Subcommand, Debug)]
pub enum SystemCommands {
    /// Display podman system information
    Info,
    /// Remove unused data
    Prune,
}
