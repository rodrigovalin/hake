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

pub struct Kind {
    pub name: String,
    pub ecr_repo: Option<String>,
    config_dir: String,
}

impl Kind {
    fn get_kind_config(dockerconfig: &str) -> Result<String> {
        let cc = ClusterConfig {
            kind: String::from("Cluster"),
            apiVersion: String::from("kind.sigs.k8s.io/v1alpha3"),
            nodes: vec![Node {
                role: String::from("control-plane"),
                extraMounts: vec![ExtraMount {
                    containerPath: String::from("/var/lib/kubelet/config.json"),
                    hostPath: String::from(dockerconfig),
                }],
            }],
        };

        Ok(serde_yaml::to_string(&cc)?)
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

    fn create_kind_config(&self, ecr: String) -> Result<String> {
        let docker_login = Kind::get_docker_login(&ecr).expect("could not get docker login");

        // save docker_login()
        let docker_config_path = format!("{}/docker_config", self.config_dir);
        let mut docker_config = File::create(&docker_config_path)?;
        docker_config.write_all(&docker_login.into_bytes())?;

        let kind_cluster_config = Kind::get_kind_config(&docker_config_path).expect("no kind");

        let kind_config_path = format!("{}/kind_config", self.config_dir);
        let mut kind_config = File::create(&kind_config_path)?;
        kind_config.write_all(&kind_cluster_config.into_bytes())?;

        Ok(kind_config_path)
    }

    fn create_dirs(cluster_name: &str) -> Result<()> {
        let home = Kind::get_config_dir()?;

        if !Path::new(&home).exists() {
            create_dir(&home)?;
        }
        create_dir(format!("{}/{}", &home, cluster_name))?;

        Ok(())
    }

    fn get_config_dir() -> Result<String> {
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

    pub fn create(&mut self) -> Result<()> {
        Kind::create_dirs(&self.name)?;

        let mut args = vec!["create", "cluster"];
        let kubeconfig;

        args.push("--name");
        args.push(&self.name);

        kubeconfig = format!("{}/kubeconfig", self.config_dir);
        args.push("--kubeconfig");
        args.push(&kubeconfig);

        let config;
        match &self.ecr_repo {
            Some(ecr) => {
                config = self.create_kind_config(ecr.to_string())?.clone();
                args.push("--config");
                args.push(&config);
            }
            None => {}
        }

        Command::new("kind").args(args).output()?;

        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let mut args = vec!["delete", "cluster"];
        if self.name != "" {
            args.push("--name");
            args.push(&self.name);
        }

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
        }
    }
}
