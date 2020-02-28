use anyhow::Result;
mod kind;

use std::fs;
use std::path::Path;

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
    List,
}

fn create(name: String, ecr: Option<String>) -> Result<()> {
    let mut cluster = Kind::new(&name);
    cluster.configure_private_registry(ecr);

    cluster.create()
}

fn delete(name: String) -> Result<()> {
    let cluster = Kind::new(&name);
    println!("deleting cluster");
    cluster.delete()
}

fn config(name: String) -> Result<()> {
    let cluster = Kind::new(&name);
    Ok(println!("{}", cluster.get_kube_config()))
}

fn list() {
    match Kind::get_config_dir() {
        Ok(config) => {
            let config = Path::new(&config);
            for entry in fs::read_dir(config).expect("could not read dir") {
                let entry = entry.unwrap();
                println!("{}", entry.file_name().to_str().unwrap());
            }
        },
        Err(_) => {}
    };
}

fn main() -> Result<()> {
    let matches = Opt::from_args();

    match matches {
        Opt::Create { name, ecr } => create(name, ecr),
        Opt::Delete { name } => delete(name),
        Opt::Config { name } => config(name),
        Opt::List => Ok(list()),
    }
}
