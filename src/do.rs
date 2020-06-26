#![allow(non_snake_case)]
///
/// Digital Ocean Kubernetes
///
use reqwest;
use reqwest::header;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::StatusCode;

use anyhow::{anyhow, Result};
use console::Style;

use std::collections::HashSet;
use std::fs::{create_dir, remove_dir_all, File};
use std::io::prelude::*;
use std::vec::Vec;
use std::{env, io, thread, time};

use serde_derive::{Deserialize, Serialize};

const ENV_DO_PROVIDER: &str = "HAKE_PROVIDER_DIGITALOCEAN_API_KEY";

#[derive(Serialize, Deserialize, Debug)]
struct NodeStatus {
    state: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Node {
    id: String,
    name: String,
    status: NodeStatus,
    droplet_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct NodePool {
    id: Option<String>,
    name: String,
    size: String,
    count: u16,
    tags: Option<Vec<String>>,
    nodes: Vec<Node>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct KubernetesCluster {
    id: Option<String>,
    name: String,
    region: String,
    version: String,
    cluster_subnet: Option<String>,
    service_subnet: Option<String>,
    vpc_uuid: Option<String>,
    ipv4: Option<String>,
    endpoint: Option<String>,
    tags: Option<Vec<String>>,
    node_pools: Vec<NodePool>,
}

#[derive(Serialize, Deserialize, Debug)]
struct KubernetesClusterResponse {
    kubernetes_cluster: KubernetesCluster,
}

#[derive(Serialize, Deserialize, Debug)]
struct LoadBalancer {
    // This is Option because it is not mandatory when creating the cluster
    id: Option<String>,
    name: String,
    ip: Option<String>,
    algorithm: String,
    status: Option<String>,
    tag: Option<String>,
    droplet_ids: Vec<u32>,
}

#[derive(Serialize, Deserialize, Default)]
struct LoadBalancerListResponse {
    load_balancers: Vec<LoadBalancer>,
}

#[derive(Debug)]
struct Metadata {
    region: String,
    version: String,
    nodepool_size: String,
    nodepool_count: u16,
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            region: "lon1".to_string(),
            version: "1.17.6-do.0".to_string(),
            nodepool_size: "s-6vcpu-16gb".to_string(),
            nodepool_count: 2,
        }
    }
}

impl Metadata {
    pub fn from_string(data: &str) -> Metadata {
        let mut metadata = Metadata::default();
        let fields: Vec<&str> = data.split("&").collect();

        // there should be a more idiomatic way of doing this!
        for field in fields {
            let split_field = field.split("=").collect::<Vec<&str>>();

            if split_field.len() != 2 {
                continue;
            }

            let value = String::from(split_field[1]);
            match split_field[0] {
                "region" => metadata.region = value,
                "version" => metadata.version = value,
                "nodepool.size" => metadata.nodepool_size = value,
                "nodepool.count" => metadata.nodepool_count = value.parse::<u16>().unwrap(),
                _ => {}
            }
        }

        metadata
    }
}

pub fn create(name: &str, metadata: Option<String>) -> Result<()> {
    let provider_metadata = metadata.unwrap_or("".to_string());
    let cluster_spec = Metadata::from_string(&provider_metadata);

    let new_cluster = KubernetesCluster {
        id: None,
        name: String::from(name),
        region: cluster_spec.region,
        version: cluster_spec.version,
        node_pools: vec![NodePool {
            size: cluster_spec.nodepool_size,
            count: cluster_spec.nodepool_count,
            name: format!("nodepool-{}", &name),
            ..Default::default()
        }],
        ..Default::default()
    };

    let client = get_do_api_client()?;
    let resp = client
        .post("https://api.digitalocean.com/v2/kubernetes/clusters")
        .header(CONTENT_TYPE, "application/json")
        .json(&new_cluster)
        .send()?;

    if resp.status() != StatusCode::CREATED {
        println!("{:?}", &resp.text()?.to_string());
        return Err(anyhow!("Could not create cluster:"));
    }

    let json_response: KubernetesClusterResponse = resp.json()?;

    let cluster_id = json_response.kubernetes_cluster.id.unwrap();
    let cyan = Style::new().cyan();
    println!("Cluster created with id: {}", cyan.apply_to(&cluster_id));

    let cluster_dir = format!("{}/{}", crate::get_config_dir(), name);
    create_dir(&cluster_dir)?;

    let url = format!(
        "https://api.digitalocean.com/v2/kubernetes/clusters/{}/kubeconfig",
        &cluster_id
    );

    // need to wait for the server to be "prepared"
    thread::sleep(time::Duration::from_secs(10));

    let mut resp = client
        .get(&url)
        .header(CONTENT_TYPE, "application/json")
        .send()?;

    let mut out =
        File::create(format!("{}/kubeconfig", &cluster_dir)).expect("failed to create file");
    io::copy(&mut resp, &mut out).expect("failed to copy content");

    let mut cluster_uuid = File::create(format!("{}/cluster_uuid", &cluster_dir))?;

    cluster_uuid.write_all(&cluster_id.as_bytes())?;

    Ok(())
}

// Return a list of droplets for a given cluster
fn get_droplets_ids_for_cluster(cluster_id: &str) -> Result<Vec<u32>> {
    let client = get_do_api_client()?;
    let resp = client
        .get(&format!(
            "https://api.digitalocean.com/v2/kubernetes/clusters/{}",
            cluster_id
        ))
        .header(ACCEPT, "application/json")
        .send()?;

    let json_response: KubernetesClusterResponse = resp.json()?;

    let mut droplet_ids: Vec<u32> = vec![];
    for node_pool in json_response.kubernetes_cluster.node_pools.iter() {
        for node in node_pool.nodes.iter() {
            if let Some(id) = &node.droplet_id {
                droplet_ids.push(id.parse::<u32>()?)
            }
        }
    }

    Ok(droplet_ids)
}

fn get_api_token() -> Result<String> {
    Ok(env::var(ENV_DO_PROVIDER)?)
}

fn auth_headers() -> Result<reqwest::header::HeaderMap> {
    let api_key = get_api_token()?;
    let bearer_auth = format!("Bearer {}", &api_key);

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&bearer_auth)?,
    );

    Ok(headers)
}

fn get_do_api_client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .default_headers(auth_headers()?)
        .build()?)
}

fn get_load_balancer_pointing_at_droplet_id(
    droplet_ids: HashSet<u32>,
) -> Result<Vec<LoadBalancer>> {
    let client = get_do_api_client()?;
    let resp = client
        .get("https://api.digitalocean.com/v2/load_balancers")
        .header(ACCEPT, "application/json")
        .send()?;

    let load_balancers: LoadBalancerListResponse = resp.json()?;

    Ok(load_balancers
        .load_balancers
        .into_iter()
        .filter(|lb| lb.droplet_ids.iter().cloned().collect::<HashSet<u32>>() == droplet_ids)
        .collect())
}

fn delete_load_balancer(lb: LoadBalancer) -> Result<()> {
    let lb_id = lb.id.expect("Got an empty id for load_balancer");
    let cyan = Style::new().cyan();
    println!("Removing Load Balancer: {}", cyan.apply_to(&lb_id));

    let client = get_do_api_client()?;
    let resp = client
        .delete(&format!(
            "https://api.digitalocean.com/v2/load_balancers/{}",
            lb_id
        ))
        .send()?;

    if resp.status() == StatusCode::NO_CONTENT {
        Ok(())
    } else {
        Err(anyhow!(
            "Could not remove Load Balancer with id: {}. Status code is: {}",
            lb_id,
            resp.status()
        ))
    }
}

fn delete_residuals(cluster_id: &str) -> Result<()> {
    let droplet_ids: HashSet<u32> = get_droplets_ids_for_cluster(&cluster_id)?
        .into_iter()
        .collect();

    let lbs = get_load_balancer_pointing_at_droplet_id(droplet_ids);
    match lbs {
        Ok(lbs) => {
            for lb in lbs {
                delete_load_balancer(lb)?;
            }
        }
        _ => {}
    }

    Ok(())
}

pub fn delete(name: &str) -> Result<()> {
    let config_dir = crate::get_config_dir();

    let doid = format!("{}/{}/cluster_uuid", config_dir, name);
    let mut file = File::open(doid)?;
    let mut cluster_id = String::new();
    file.read_to_string(&mut cluster_id)?;

    delete_residuals(&cluster_id)?;

    let cyan = Style::new().cyan();
    println!("Removing Cluster: {}", cyan.apply_to(&cluster_id));
    let client = get_do_api_client()?;
    let resp = client
        .delete(&format!(
            "https://api.digitalocean.com/v2/kubernetes/clusters/{}",
            cluster_id
        ))
        .send()?;

    if resp.status() != StatusCode::NO_CONTENT {
        return Err(anyhow!(
            "Could not remove Cluster with id: {}. Status code is: {}",
            &cluster_id,
            resp.status()
        ));
    }

    remove_dir_all(format!("{}/{}", config_dir, name))?;

    Ok(())
}
