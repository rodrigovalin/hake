#![allow(non_snake_case)]

use anyhow::Result;

use serde_derive::{Serialize, Deserialize};
use serde_json::json;
use base64::encode;
use std::io::{Read, Write};
use std::fs::File;
use std::env;

use std::process::{Command, Stdio};
use std::str;

static REGISTRY: &str = "268558157000.dkr.ecr.us-east-1.amazonaws.com";

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

fn main() -> Result<()> {
    let docker_login = get_docker_login().expect("could not get docker login");

    // save docker_login
    let mut docker_config = File::create("docker_config")?;
    docker_config.write_all(&docker_login.into_bytes())?;

    let path = env::current_dir()?;
    let host_path = format!("{}/docker_config", &path.to_str().expect("could not get path"));

    let kind_cluster_config = get_kind_config(&host_path).expect("no kind");
    let mut kind_config = File::create("kind_config")?;
    kind_config.write_all(&kind_cluster_config.into_bytes())?;

    let cmd = Command::new("kind")
        .arg("create")
        .arg("cluster")
        .output()
        .expect("could not find kind");


    let output = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    let stderr = str::from_utf8(cmd.stderr.as_slice()).unwrap();

    println!("hmmmm -> {}", output);
    println!("hmmmm -> {}", stderr);

    Ok(())
}

fn get_kind_config(dockerconfig: &str) -> Result<String> {
    let cc = ClusterConfig {
        kind: String::from("kind"),
        apiVersion: String::from("kind.sigs.k8s.io/v1alpha3"),
        nodes: vec![
            Node {
                role: String::from("control-plane"),
                extraMounts: vec![
                    ExtraMount {
                        containerPath: String::from("/var/lib/kubelet/config.json"),
                        hostPath: String::from(dockerconfig),
                    }
                ]
            }
        ]
    };

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
