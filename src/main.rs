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
        #[structopt(long)]
        name: String,
    },
    /// Deletes a kind cluster
    Delete {
        #[structopt(long)]
        name: String,
    },
}

fn create(name: String) -> Result<()> {
    let mut cluster = Kind::new(&name);
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
        Opt::Create { name } => create(name),
        Opt::Delete { name } => delete(name),
    }
}
