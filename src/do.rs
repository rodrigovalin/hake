#![allow(non_snake_case)]
///
/// Digital Ocean Kubernetes
///
use reqwest;
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;
use std::io;
use std::vec::Vec;

use anyhow::Result;

use dirs;
use std::fs::{create_dir, remove_dir_all, File};
use std::io::prelude::*;
use std::path::Path;
use std::{thread, time};

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize)]
struct NodePool {
    size: String,
    count: u16,
    name: String,
}

#[derive(Serialize)]
struct Cluster {
    name: String,
    region: String,
    version: String,
    node_pools: Vec<NodePool>,
}

#[derive(Serialize, Deserialize, Debug)]
struct KubernetesCluster {
    id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    kubernetes_cluster: KubernetesCluster,
}

pub fn create(name: &str) {
    // let code = "41c36e305a77cdba070c41bd535b9110ce800a3b9837fe2c418667138d3f8254"; //readonly
    let code = "485cd763a427046b82221ba55f95ee484b15c97fdfa386364dc1caec68f82c8e";
    let cluster_dir = format!("{}/{}", get_config_dir(), name);

    if Path::new(&cluster_dir).exists() {
        println!("Cluster with name {} already exists", name);
        return ();
    }

    create_dir(cluster_dir);
    // size: s-6vcpu-16gb

    let new_cluster = Cluster {
        name: String::from(name),
        region: String::from("lon1"),
        version: String::from("1.17.5-do.0"),
        node_pools: vec![NodePool {
            size: String::from("s-6vcpu-16gb"),
            count: 2,
            name: String::from("this-nodepool"),
        }],
    };

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("https://api.digitalocean.com/v2/kubernetes/clusters")
        .bearer_auth(code)
        .header(CONTENT_TYPE, "application/json")
        .json(&new_cluster)
        .send()
        .unwrap();

    if resp.status() != StatusCode::CREATED {
        println!("Could not create cluster");
        return;
    }

    // println!("{:?}", resp.text());

    let json_response: Response = resp.json().unwrap();
    println!("{:?}", json_response);

    let home = String::from(
        dirs::home_dir()
            .expect("User does not have a home")
            .to_str()
            .unwrap(),
    );

    let mut out =
        File::create(format!("{}/.hake/{}/kubeconfig", home, name)).expect("failed to create file");

    let url = format!(
        "https://api.digitalocean.com/v2/kubernetes/clusters/{}/kubeconfig",
        json_response.kubernetes_cluster.id
    );

    // need to wait for the server to be "prepared"
    let ten_secs = time::Duration::from_secs(10);
    thread::sleep(ten_secs);

    let mut resp = client
        .get(&url)
        .bearer_auth(code)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .unwrap();

    io::copy(&mut resp, &mut out).expect("failed to copy content");

    let mut cluster_uuid = File::create(format!("{}/.hake/{}/cluster_uuid", home, name)).unwrap();

    cluster_uuid
        .write_all(&json_response.kubernetes_cluster.id.as_bytes())
        .unwrap();
}

pub fn config() -> String {
    let home = String::from(
        dirs::home_dir()
            .expect("User does not have a home")
            .to_str()
            .unwrap(),
    );

    format!("{}/.hake/hake-default/kubeconfig", home)
}

fn get_config_dir() -> String {
    let home = String::from(
        dirs::home_dir()
            .expect("User does not have a home")
            .to_str()
            .unwrap(),
    );

    format!("{}/.hake", home)
}

pub fn delete(name: &str) -> Result<()> {
    let doid = format!("{}/{}/cluster_uuid", get_config_dir(), name);
    let mut file = File::open(doid)?;
    let mut cluster_id = String::new();
    file.read_to_string(&mut cluster_id)?;

    println!("go all the way to here");
    let code = "485cd763a427046b82221ba55f95ee484b15c97fdfa386364dc1caec68f82c8e";
    let client = reqwest::blocking::Client::new();
    client
        .delete(&format!(
            "https://api.digitalocean.com/v2/kubernetes/clusters/{}",
            cluster_id
        ))
        .bearer_auth(code)
        .send()?;

    remove_dir_all(format!("{}/{}", get_config_dir(), name))?;

    Ok(())
}
