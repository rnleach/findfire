use std::error::Error;

use satfire::ClusterDatabase;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";

fn main() -> Result<(), Box<dyn Error>> {
    let _cluster_db = ClusterDatabase::connect(DATABASE_FILE)?;

    println!("Hello world.");

    Ok(())
}
