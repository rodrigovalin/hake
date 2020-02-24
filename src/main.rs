use anyhow::Result;
mod kind;

use crate::kind::Kind;

fn main() -> Result<()> {
    println!("Starting Kind");
    Kind::create()?;

    println!("Stopping Kind");
    Kind::delete()
}
