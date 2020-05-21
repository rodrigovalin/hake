// adds a "capability", which is a super naive implementation
// to add things to the kube cluster.
use anyhow::Result;
use std::process::Command;

pub fn cert_manager() -> Result<()> {
    Command::new("kubectl")
        .arg("apply")
        .arg("--validate=false")
        .arg("-f")
        .arg("https://github.com/jetstack/cert-manager/releases/download/v0.15.0/cert-manager.yaml")
        .output()?;

    Ok(())
}

pub fn ingress_nginx() -> Result<()> {
    run_kubectl("apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/master/deploy/static/provider/kind/deploy.yaml")
}

fn run_kubectl(command: &str) -> Result<()> {
    Command::new("kubectl")
        .args(command.split(" ").collect::<Vec<&str>>())
        .output()?;

    Ok(())
}
