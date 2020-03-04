#![allow(non_snake_case)]

use anyhow::Result;
use dirs;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

use base64::encode;
use std::fs::{create_dir, remove_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::str;
use std::collections::HashMap;
use std::vec::Vec;

use bollard::Docker;
use bollard::container::ListContainersOptions;
use tokio::runtime::Runtime;

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
    containerdConfigPatches: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct DockerLogin {
    Username: String,
    Secret: String,
}

pub struct Kind {
    pub name: String,
    pub ecr_repo: Option<String>,
    config_dir: String,
    local_registry: Option<String>,
}

impl Kind {
    fn get_kind_config(&self, ecr: &Option<String>, local_reg: &Option<String>) -> Result<String> {
        let mut cc = ClusterConfig {
            kind: String::from("Cluster"),
            apiVersion: String::from("kind.x-k8s.io/v1alpha4"),
            nodes: vec![],
            containerdConfigPatches: vec![],
        };

        match ecr {
            Some(ecr) => {
                let docker_path = self.create_docker_ecr_config_file(ecr.to_string());
                match docker_path {
                    Ok(docker_path) => {
                        cc.nodes = vec![Node {
                            role: String::from("control-plane"),
                            extraMounts: vec![ExtraMount {
                                containerPath: String::from("/var/lib/kubelet/config.json"),
                                hostPath: self.create_docker_ecr_config_file(docker_path)?,
                            }],
                        }];
                    },
                    _ => {},
                }
            },
            None => {},
        }

        match local_reg {
            Some(ip) => cc.containerdConfigPatches = vec![Kind::get_containerd_config_patch_to_local_registry(&ip)],
            None => {},
        }

        let kind_cluster_config = serde_yaml::to_string(&cc)?;
        
        let kind_config_path = format!("{}/kind_config", self.config_dir);
        let mut kind_config = File::create(&kind_config_path)?;
        kind_config.write_all(&kind_cluster_config.into_bytes())?;

        Ok(kind_config_path)
    }

    fn get_containerd_config_patch_to_local_registry(ip: &str) -> String {
        format!(r#"|
[plugins."io.containerd.grpc.v1.cri".registry.mirrors."localhost:5000"]
  endpoint = ["http://{}:5000"]"#, ip.trim())
    }

    /// Gets the Kind cluster name from the Docker container name.
    fn get_cluster_name(container_name: &str) -> Option<String> {
        if !container_name.ends_with("-control-plane") {
            None
        } else {
            let parts: Vec<&str> = container_name.split("-control-plane").collect();
            let part = parts.get(0).unwrap().to_string();

            if &part[0..1] == "/" {
                Some(String::from(&part[1..]))
            } else {
                Some(part)
            }
        }
    }

    // Removes every entry in ~/.nomake that does not have a corresponding kind docker container.
    async fn async_get_containers() -> Result<Vec<String>> {
        let docker = Docker::connect_with_local_defaults()?;
        let mut filter = HashMap::new();
        filter.insert(String::from("status"), vec![String::from("running")]);
        let containers = &docker.list_containers(Some(ListContainersOptions{
            all: true,
            filters: filter,
            ..Default::default()
        })).await?;

        let mut kind_containers = Vec::new();
        for container in containers {
            if container.image.starts_with("kindest/node") {
                let name = String::from(container.names.get(0).unwrap());
                match Kind::get_cluster_name(&name) {
                    Some(name) => kind_containers.push(name),
                    None => continue,
                }
            }
        }

        Ok(kind_containers)
    }

    pub fn get_kind_containers() -> Result<Vec<String>> {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(Kind::async_get_containers())
    }

    fn get_docker_login(registry: &str) -> Result<String> {
        let creds = Kind::get_docker_credentials_from_helper(registry)?;

        let login: DockerLogin = serde_json::from_str(&creds)?;
        let encoded = encode(&format!("{}:{}", login.Username, login.Secret));

        Ok(json!({
                "auths": {
                    registry: {
                        "auth": encoded
                    }
                }
            }
        )
        .to_string())
    }

    fn get_docker_credentials_from_helper(registry: &str) -> Result<String> {
        let mut cmd = Command::new("docker-credential-ecr-login")
            .arg("get")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect(&format!(
                "Could not find docker credentials helper for {}",
                registry
            ));

        cmd.stdin.as_mut().unwrap().write_all(registry.as_bytes())?;
        cmd.wait()?;

        let mut output = String::new();
        cmd.stdout.unwrap().read_to_string(&mut output)?;

        Ok(output)
    }

    fn create_docker_ecr_config_file(&self, ecr: String) -> Result<String> {
        let docker_login = Kind::get_docker_login(&ecr).expect("could not get docker login");

        // save docker_login()
        let docker_config_path = format!("{}/docker_config", self.config_dir);
        let mut docker_config = File::create(&docker_config_path)?;
        docker_config.write_all(&docker_login.into_bytes())?;

        Ok(docker_config_path)
    }

    fn create_dirs(cluster_name: &str) -> Result<()> {
        let home = Kind::get_config_dir()?;

        if !Path::new(&home).exists() {
            create_dir(&home)?;
        }
        create_dir(format!("{}/{}", &home, cluster_name))?;

        Ok(())
    }

    pub fn get_config_dir() -> Result<String> {
        let home = String::from(
            dirs::home_dir()
                .expect("User does not have a home")
                .to_str()
                .unwrap(),
        );

        Ok(format!("{}/.nomake", home))
    }

    pub fn get_kube_config(self) -> String {
        format!("{}/kubeconfig", self.config_dir)
    }

    pub fn configure_private_registry(&mut self, reg: Option<String>) {
        self.ecr_repo = reg;
    }

    fn start_local_registry() -> Option<String> {
        // the following command returns a handle to the child, but it is spawned as a different process.
        // let _registry = Command::new("docker")
        //     .args(&["run", "--restart=always", "-p", "5000:5000", "--name", "local-registry", "registry:2"])
        //     .spawn()
        //     .expect("Could not start local Docker registry");

        let ip = Command::new("docker")
            .args(vec!["inspect", "-f", "'{{.NetworkSettings.IPAddress}}'", "local-registry"])
            .output()
            .expect("Could not get IP from local registry");

        Some(String::from_utf8(ip.stdout).unwrap())
    }

    pub fn use_local_registry(&mut self) {
        self.local_registry = Kind::start_local_registry();
    }

    pub fn create(self) -> Result<()> {
        Kind::create_dirs(&self.name)?;

        let mut args = vec!["create", "cluster"];
        let kubeconfig;

        args.push("--name");
        args.push(&self.name);

        kubeconfig = format!("{}/kubeconfig", self.config_dir);
        args.push("--kubeconfig");
        args.push(&kubeconfig);

        args.push("--config");
        let kind_config = self.get_kind_config(&self.ecr_repo, &self.local_registry).expect("could not not bla bla");
        args.push(&kind_config);

        println!("Running kind with: {:?}", args);

        Command::new("kind").args(args).output()?;

        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let mut args = vec!["delete", "cluster"];
        args.push("--name");
        args.push(&self.name);

        let _cmd = Command::new("kind")
            .args(args)
            .output()
            .expect("could not find kind");

        remove_dir_all(&self.config_dir)?;
        Ok(())
    }

    pub fn new(name: &str) -> Kind {
        let config = Kind::get_config_dir();
        if config.is_err() {
            panic!("User has no home!!");
        }
        let home = config.unwrap();

        Kind {
            name: String::from(name),
            ecr_repo: None,
            config_dir: format!("{}/{}", home, name),
            local_registry: None,
        }
    }
}
