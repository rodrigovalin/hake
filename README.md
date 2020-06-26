# hake

![hake](meat.png "Fish with no bone")

`hake` is a small CLI utility to start a Kubernetes cluster for testing
purposes, with a few convenience features. It supports configuring access to ECR
or to a local registry. `hake` can use different providers to start a cluster.
Current providers are:

+ [Kind](https://kind.sigs.k8s.io/): local clusters running on top of Docker.
+ [DigitalOcean](https://digitalocean.com): Very convenient and cheap Kubernetes
  cluster hosted by DigitalOcean.

The easiest way to start is to use `hake` to start Kind clusters, in which case,
the `kind` binary needs to exist in `$PATH`.

## Usage

The simplest way of using `hake` is to create a simple Kind local cluster.

``` sh
# creates a simple cluster
$ hake create
# and to configure kubectl
$ eval $(hake config) # this exports KUBECONFIG
# checks that everything is working
$ kubectl get namespaces
NAME                 STATUS   AGE
default              Active   66s
...
# removes the cluster at the end
$ hake delete
```

## Configuring access to ECR

`hake` can configure access to a private ECR repo. It requires the
[ecr-login-helper](https://github.com/awslabs/amazon-ecr-credential-helper) to
be in your PATH.

``` sh
$ hake create --ecr xxx.ecr.region.amazonaws.com
$ eval $(hake config)
$ kubectl create deployment example --image xxx.ecr.region.amazonaws.com/xxx
```

## Configuring access to a local registry

`hake` can use a local registry to speed up local development. To start the
local cluster follow the instructions
[here](https://kind.sigs.k8s.io/docs/user/local-registry/) and then:

``` sh
$ hake create --use-local-registry "kind-registry"
$ eval $(hake config)
$ kubectl create deployment example --image localhost:5000/xxx
```

## DigitalOcean Provider

You can start Kubernetes clusters on DigitalOcean. DigitalOcean is really cheap,
the clusters start in around 5 minutes and they only charge for the worker nodes
and not the masters. I've been using DigitalOcean for more complex testing
scenarios, specially when the testing environment does not fit on my laptop's
16GB of RAM.

### Requirements for DigitalOcean

* Create a `write` API key [here](https://cloud.digitalocean.com/account/api/tokens)

The API Key needs to be expored as a environment variable like:

    export HAKE_PROVIDER_DIGITALOCEAN_API_KEY="my-api-key"

### Metadata

DigitalOcean offering supports multiple configurations for your Kubernetes cluster. To pass
specific parameters use the `--metadata` option, like:

    hake create --provider digitalocean --metadata="region=lon1&version=1.17.6-do.0&nodepool.size=s-4vcpu-8gb&nodepool.count=2"

So far the variables you can change are:

* region
* version
* nodepool.size
* nodepool.count

## What else?

This is an exercise to learn [Rust](https://www.rust-lang.org/) which is
probably the most interesting programming language from the last couple of
years. Also I wanted to try something different than shell-scripting for this
kind of task.
