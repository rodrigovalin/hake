use anyhow::Result;
mod kind;

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

fn main() -> Result<()> {
    let matches = Opt::from_args();
    println!("{:?}", matches);

    match matches {
        Opt::Create { name, ecr } => create(name, ecr),
        Opt::Delete { name } => delete(name),
    }
}
