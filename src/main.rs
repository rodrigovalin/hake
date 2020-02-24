use anyhow::Result;
mod kind;

use crate::kind::Kind;

fn main() -> Result<()> {
    println!("Starting Kind");
    let mut cluster = Kind::new("my-kind", "268558157000.dkr.ecr.us-east-1.amazonaws.com");
    cluster.create()?;

    println!("stopping ");
    //cluster.delete()?;
    Ok(())
}
