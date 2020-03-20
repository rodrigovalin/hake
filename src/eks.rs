use rusoto_core::Region;
use rusoto_eks::{Eks, EksClient, ListClustersRequest};

use std::default::Default;

async fn create() {
    let client = EksClient::new(Region::UsEast1);
    let list_cluster_request: ListClustersRequest = Default::default();

    match client.list_clusters(list_cluster_request).await {
        Ok(output) => match output.clusters {
            Some(clusters) => {
                println!("{} clusters were found", clusters.len());
                for cluster in clusters {
                    println!("{}", cluster);
                }
            }
            None => println!("no clusters found")
        },
        Err(_) => println!("got into error state")
    }
}

#[cfg(test)]
mod tests {
    use crate::eks::create;
    use tokio::runtime::Runtime;
    #[test]
    fn test0() {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(create());
    }
}
