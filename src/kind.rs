#![allow(non_snake_case)]

use anyhow::Result;
use dirs;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

use base64::encode;
use std::collections::HashMap;
use std::fs::{create_dir, remove_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::str;
use std::vec::Vec;

use bollard::container::ListContainersOptions;
use bollard::Docker;
use tokio::runtime::Runtime;

use regex::Regex;

#[derive(Serialize, Deserialize, Debug)]
struct ExtraMount {
    containerPath: String,
    hostPath: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PortMapping{
    containerPort: u32,
    hostPort: u32,
    protocol: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Node {
    role: String,
    extraMounts: Vec<ExtraMount>,
    extraPortMappings: Vec<PortMapping>,
    kubeadmConfigPatches: Vec<String>,
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
    extra_port_mapping: Option<String>,
    verbose: bool,
}

impl Kind {
    fn kind_node(role: &str, container_path: Option<&str>, host_path: Option<&str>) -> Vec<Node> {
        let extraPortMappings: Vec<PortMapping> = Vec::new();
        let kubeadmConfigPatches: Vec<String> = Vec::new();
        vec![Node {
            role: String::from(role),
            extraMounts: vec![ExtraMount {
                containerPath: String::from(container_path.unwrap_or_default()),
                hostPath: String::from(host_path.unwrap_or_default()),
            }],
            extraPortMappings: extraPortMappings,
            kubeadmConfigPatches: kubeadmConfigPatches,
        }]
    }

    fn init_config_ingress_ready() -> String {
        String::from(r#"kind: InitConfiguration
nodeRegistration:
  kubeletExtraArgs:
    node-labels: "ingress-ready=true""#)
    }

    fn get_kind_cluster_config(&self, ecr: &Option<String>, local_reg: &Option<String>) -> ClusterConfig {
        let mut cc = ClusterConfig {
            kind: String::from("Cluster"),
            apiVersion: String::from("kind.x-k8s.io/v1alpha4"),
            nodes: vec![],
            containerdConfigPatches: vec![],
        };

        if let Some(ecr) = ecr {
            if let Ok(docker_path) = self.create_docker_ecr_config_file(ecr) {
                cc.nodes = Kind::kind_node(
                    "control-plane",
                    Some("/var/lib/kubelet/config.json"),
                    Some(&docker_path),
                );
            }
        }

        if let Some(local_reg) = local_reg {
            cc.containerdConfigPatches = vec![Kind::get_containerd_config_patch_to_local_registry(
                local_reg,
            )];
        }

        cc
    }

    fn get_containerd_config_patch_to_local_registry(ip: &str) -> String {
        format!(
            r#"
[plugins."io.containerd.grpc.v1.cri".registry.mirrors."localhost:5000"]
  endpoint = ["http://{}:5000"]"#,
            ip.trim()
        )
    }

    /// Gets the Kind cluster name from the Docker container name.
    fn get_cluster_name(container_name: &str) -> Option<String> {
        if !container_name.ends_with("-control-plane") {
            None
        } else {
            Some(container_name.replace("-control-plane", "").replace("/", ""))
        }
    }

    // Removes every entry in ~/.hake that does not have a corresponding kind docker container.
    async fn async_get_containers() -> Result<Vec<String>> {
        let docker = Docker::connect_with_local_defaults()?;
        let mut filter = HashMap::new();
        filter.insert(String::from("status"), vec![String::from("running")]);
        let containers = &docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: filter,
                ..Default::default()
            }))
            .await?;

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
            .unwrap_or_else(|_| panic!("Could not find docker credentials helper for {}", registry));

        cmd.stdin.as_mut().unwrap().write_all(registry.as_bytes())?;
        cmd.wait()?;

        let mut output = String::new();
        cmd.stdout.unwrap().read_to_string(&mut output)?;

        Ok(output)
    }

    fn create_docker_ecr_config_file(&self, ecr: &str) -> Result<String> {
        let docker_login = Kind::get_docker_login(ecr).expect("could not get docker login");

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

        Ok(format!("{}/.hake", home))
    }

    pub fn get_kube_config(self) -> String {
        format!("{}/kubeconfig", self.config_dir)
    }

    pub fn configure_private_registry(&mut self, reg: Option<String>) {
        self.ecr_repo = reg;
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    fn find_local_registry(container_name: &str) -> Option<String> {
        let ip = Command::new("docker")
            .arg("inspect")
            .arg("-f")
            .arg("{{.NetworkSettings.IPAddress}}")
            .arg(container_name)
            .output()
            .expect(&format!("Could not get IP from {} container", container_name));

        Some(String::from_utf8(ip.stdout).unwrap().trim().to_string())
    }

    pub fn use_local_registry(&mut self, container_name: &str) {
        self.local_registry = Kind::find_local_registry(container_name);
    }

    pub fn extra_port_mapping(&mut self, extra_port_mapping: &str) {
        self.extra_port_mapping = Some(String::from(extra_port_mapping));
    }

    /// receives a string like: 80:80:TCP or 80:80 or 80
    fn parse_extra_port_mappings(epm: &str) -> Option<PortMapping> {
        let container_port = 80;
        let host_port = 80;
        let mut proto = "TCP";

        let re0 = Regex::new(r"^(\d{2}):(\d{2}):(TCP|HTTP)$").unwrap();
        let _re1 = Regex::new(r"^(\d{2}):(\d{2})$").unwrap();
        let _re2 = Regex::new(r"^(\d{2})$").unwrap();

        if re0.is_match(epm) {
            let cap = re0.captures(epm).unwrap();
            Some(PortMapping {
                containerPort: cap[1].parse::<u32>().unwrap(),
                hostPort: cap[2].parse::<u32>().unwrap(),
                protocol: String::from(&cap[3]),
            })
        } else {
            None
        }
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
        let mut kind_config = self
            .get_kind_cluster_config(&self.ecr_repo, &self.local_registry);
        if let Some(extra_port_mapping) = self.extra_port_mapping {
            let epm = Kind::parse_extra_port_mappings(&extra_port_mapping);
            if let Some(epm) = epm {
                if kind_config.nodes.len() == 0 {
                    kind_config.nodes = Kind::kind_node("control-plane", None, None);
                }
                kind_config.nodes[0].extraPortMappings = vec![epm];
            }
        }


        let kind_cluster_config = serde_yaml::to_string(&kind_config)?;

        let kind_config_path = format!("{}/kind_config", self.config_dir);
        let mut kind_config = File::create(&kind_config_path)?;
        kind_config.write_all(&kind_cluster_config.into_bytes())?;

        // point the config file to the one we just saved
        args.push(&kind_config_path);

        Kind::run(&args, self.verbose)?;

        let config_dir = Kind::get_config_dir()?;
        let config_dir = format!("{}/{}/kind_args", config_dir, &self.name);
        let mut saved_args = File::create(config_dir)?;
        saved_args.write_all(args.join(" ").as_bytes())?;

        Ok(())
    }

    pub fn run(args: &Vec<&str>, verbose: bool) -> Result<()> {
        let mut command = Command::new("kind");
        command.args(args);
        if verbose {
            command.spawn()?.wait()?;
        } else {
            command.output()?;
        }

        Ok(())
    }

    pub fn recreate(name: &str, verbose: bool) -> Result<()> {
        let config_dir = format!("{}/{}", Kind::get_config_dir()?, name);
        let args_file = format!("{}/kind_args", config_dir);

        let mut contents = String::new();
        let mut saved_args = File::open(args_file)?;
        saved_args.read_to_string(&mut contents)?;

        Kind::delete_cluster(name)?;

        let args: Vec<&str> = contents.split_ascii_whitespace().collect();
        Kind::run(&args, verbose)?;

        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        Kind::delete_cluster(&self.name)?;

        remove_dir_all(&self.config_dir)?;

        Ok(())
    }

    fn delete_cluster(name: &str) -> Result<()> {
        let mut args = vec!["delete", "cluster"];
        args.push("--name");
        args.push(name);

        let _cmd = Command::new("kind")
            .args(args)
            .output()?;

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
            extra_port_mapping: None,
            verbose: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::kind::Kind;

    #[test]
    fn test_new() {
        // TODO: test configuration on home directory.
        let k = Kind::new("test");

        let home = dirs::home_dir()
            .unwrap();

        assert_eq!(k.name, "test");
        assert_eq!(k.ecr_repo, None);
        assert_eq!(k.config_dir, format!("{}/.hake/test", home.to_str().unwrap()));
        assert_eq!(k.local_registry, None);
    }

    #[test]
    fn test_get_cluster_name() {
        assert_eq!(Kind::get_cluster_name("not-us"), None);
        assert_eq!(Kind::get_cluster_name("this-is-us-control-plane"), Some(String::from("this-is-us")));
        assert_eq!(Kind::get_cluster_name("/this-is-us-control-plane"), Some(String::from("this-is-us")));
    }
}
