# nomake

`nomake` is a small CLI utility to start a Kubernetes cluster for testing
purposes, with a few convenience features. It supports configuring access to ECR
or to a local registry. `nomake` uses [Kind](https://kind.sigs.k8s.io/) to start
the local cluster.

The only requirement for `nomake` is the `kind` binary to be in your `$PATH`.

## Usage

The simplest way of using `nomake` is to create a simple cluster.

``` sh
# creates a simple cluster
$ nomake create
# and to configure kubectl
$ eval $(nomake config --env) # this exports KUBECONFIG
# checks that everything is working
$ kubectl get namespaces
NAME                 STATUS   AGE
default              Active   66s
...
# removes the cluster at the end
$ nomake delete
```

## Configuring access to ECR

`nomake` can configure access to a private ECR repo. It requires the
[ecr-login-helper](https://github.com/awslabs/amazon-ecr-credential-helper) to
be in your PATH.

``` sh
$ nomake create --ecr xxx.ecr.region.amazonaws.com
$ eval $(nomake config --env)
$ kubectl create deployment example --image xxx.ecr.region.amazonaws.com/xxx
```

## Configuring access to a local registry

`nomake` can use a local registry to speed up local development. To start the
local cluster follow the instructions
[here](https://kind.sigs.k8s.io/docs/user/local-registry/) and then:

``` sh
$ nomake create --use-local-registry "kind-registry"
$ eval $(nomake config --env)
$ kubectl create deployment example --image localhost:5000/xxx
```

## What else?

This is an exercise to learn [Rust](https://www.rust-lang.org/) which is
probably the most interesting programming language from the last couple of
years. Also I wanted to try something different than shell-scripting for this
kind of task.
