use anyhow::Result;
mod kind;

use structopt::StructOpt;
use crate::kind::Kind;

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

fn create(name: String) {
    let mut cluster = Kind::new(&name, "268558157000.dkr.ecr.us-east-1.amazonaws.com");
    cluster.create();
}

fn delete(name: String) {
    let mut cluster = Kind::new(&name, "268558157000.dkr.ecr.us-east-1.amazonaws.com");
    println!("deleting cluster");
    cluster.delete();
}

fn main() -> Result<()> {
    let matches = Opt::from_args();
    println!("{:?}", matches);

    match matches {
        Opt::Create{name} => create(name),
        Opt::Delete{name} => delete(name),
    }

    Ok(())
}
