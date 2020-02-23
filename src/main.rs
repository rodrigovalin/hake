#![allow(non_snake_case)]

use anyhow::Result;

use serde_derive::{Serialize, Deserialize};
use serde_json::json;
use base64::encode;
use std::io::{Read, Write};

use std::process::{Command, Stdio};
use std::str;

static REGISTRY: &str = "268558157000.dkr.ecr.us-east-1.amazonaws.com";

// {
//     "kind": "Cluster",
//     "apiVersion": "kind.sigs.k8s.io/v1alpha3",
//     "nodes": [{"role": "control-plane", "extraMounts": [{"containerPath": "/var/lib/kubelet/config.json", "hostPath": config_file}]}]
//
//
// {
//      "role": "control-plane",
//      "extraMounts": [{"containerPath": "/var/lib/kubelet/config.json", "hostPath": config_file}]}}

// {"containerPath": "/var/lib/kubelet/config.json", "hostPath": config_file}

#[derive(Serialize, Deserialize, Debug)]
struct ExtraMount {
    containerPath: String,
    hostPath: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Node {
    role: String,
    extraMounts: Vec<ExtraMount>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClusterConfig {
    kind: String,
    apiVersion: String,
    nodes: Vec<Node>,
}

#[derive(Deserialize, Debug)]
struct DockerLogin {
    Username: String,
    Secret: String,
}

fn main() {
    println!("{}", get_docker_login().expect("could not get docker login"));
    println!("{}", get_kind_config(String::from("aoe")).expect("no kind"));

    let cmd = Command::new("kind")
        .arg("create")
        .arg("cluster")
        .output()
        .expect("could not find kind");


    let output = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    let stderr = str::from_utf8(cmd.stderr.as_slice()).unwrap();

    println!("hmmmm -> {}", output);
    println!("hmmmm -> {}", stderr);
}

/// {"role": "control-plane", "extraMounts": [{"containerPath": "/var/lib/kubelet/config.json", "hostPath": config_file}]}
fn get_kind_config(dockerconfig: String) -> Result<String> {
    let cc = ClusterConfig {
        kind: String::from("kind"),
        apiVersion: String::from("kind.sigs.k8s.io/v1alpha3"),
        nodes: vec![
            Node {
                role: String::from("control-plane"),
                extraMounts: vec![
                    ExtraMount {
                        containerPath: String::from("/var/lib/kubelet/config.json"),
                        hostPath: String::from("blabla"),
                    }
                ]
            }
        ]
    };

    // println!("{:?}", cc);

    Ok(serde_yaml::to_string(&cc)?)
}

fn get_docker_login() -> Result<String> {
    let creds = get_docker_credentials_from_helper()?;

    let login: DockerLogin = serde_json::from_str(&creds)?;
    let encoded = encode(&format!("{}:{}", login.Secret, login.Username));

    Ok(
        json!({
            "auths": {
                REGISTRY: {
                    "auth": encoded
                }
            }
        }
    ).to_string())
}

fn get_docker_credentials_from_helper() -> Result<String> {
    let mut cmd = Command::new("docker-credential-ecr-login")
        .arg("get")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // TODO: read from config file or env variable
    cmd.stdin.as_mut().unwrap().write_all(REGISTRY.as_bytes())?;
    cmd.wait()?;
    
    let mut output = String::new();
    cmd.stdout.unwrap().read_to_string(&mut output)?;

    Ok(output)
}
