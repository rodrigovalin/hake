use anyhow::Result;
mod kind;

use std::fs;
use std::path::Path;
use std::vec::Vec;

use crate::kind::Kind;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "Kind")]
/// The kind bla
enum Opt {
    /// Creates a kind cluster
    Create {
        /// Name of the cluster
        #[structopt(long)]
        name: String,

        /// Configures access to an ECR private registry
        #[structopt(long)]
        ecr: Option<String>,

        /// Configure access to local Docker registry
        #[structopt(long)]
        use_local_registry: bool,
    },
    /// Deletes a kind cluster
    Delete {
        /// Name of the cluster
        #[structopt(long)]
        name: String,
    },
    /// Get cluster configuration
    Config {
        /// name of the cluster
        #[structopt(long)]
        name: String,
    },
    /// Display list of known clusters
    List,
    /// Removes clusters that are not reachable anymore
    Clean {
        /// Force removal of directories
        #[structopt(long)]
        force: bool,
    },
}

fn create(name: String, ecr: Option<String>, use_local_registry: bool) -> Result<()> {
    let mut cluster = Kind::new(&name);
    cluster.configure_private_registry(ecr);

    if use_local_registry {
        cluster.use_local_registry();
    }

    cluster.create()
}

fn delete(name: String) -> Result<()> {
    let cluster = Kind::new(&name);
    println!("deleting cluster");
    cluster.delete()
}

fn config(name: String) -> Result<()> {
    let cluster = Kind::new(&name);
    println!("{}", cluster.get_kube_config());

    Ok(())
}

fn all_clusters() -> Vec<String> {
    let mut clusters = Vec::new();

    if let Ok(config) = Kind::get_config_dir() {
        let config = Path::new(&config);
        for entry in fs::read_dir(config).expect("could not read dir") {
            let entry = entry.unwrap();
            let entry = entry.file_name().to_str().unwrap().to_string();
            clusters.push(entry);
        }
    }

    clusters
}

fn list() {
    for cluster in all_clusters() {
        println!("{}", cluster);
    }
}

fn clean(force: bool) -> Result<()> {
    let kc = Kind::get_kind_containers()?;
    let clusters = all_clusters();

    for cluster in clusters {
        if !kc.iter().any(|c| *c == cluster) {
            let dir = format!("{}/{}", Kind::get_config_dir()?, cluster);
            if force {
                println!("Removing {}", dir);
                fs::remove_dir_all(dir)?
            } else {
                println!("Not removing {}. Use --force", dir);
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let matches = Opt::from_args();

    match matches {
        Opt::Create {
            name,
            ecr,
            use_local_registry,
        } => create(name, ecr, use_local_registry),
        Opt::Delete { name } => delete(name),
        Opt::Config { name } => config(name),
        Opt::List => Ok(list()),
        Opt::Clean { force } => clean(force),
    }
}
