---
aliases:
- Use a container-orchestration system
lastmod: '2022-09-09 07:43:57'
title: Kubernetes - DevOps Engine
---

It’s a platform that allows you to run and orchestrate container workloads. [Kubernetes](https://stackoverflow.blog/2020/05/29/why-kubernetes-getting-so-popular/) has become the de-facto standard** for your cloud-native apps to (auto-) [scale-out](https://stackoverflow.com/a/11715598/5246670) and deploys your open-source zoo fast, cloud-provider-independent. No lock-in here. You could use [open-shift](https://www.openshift.com) or [OKD](https://www.okd.io/). With the latest version, they added the <a href="https://operatorhub.io/">OperatorHub</a> which you can install as of today 182 items with just a few clicks. Also, check out [[Stackable Kubernetes Cluster (Lego Blocks)]] which were created to mitigate exactly that.

Some more reasons for Kubernetes are the **move from infrastructure as code** towards **infrastructure as data**, specifically as [[YAML]]. All the resources in Kubernetes that include Pods, Configurations, Deployments, Volumes, etc., can simply be expressed in a YAML file. Developers quickly write applications that run across multiple operating environments. Costs can be reduced by scaling down (even to zero with, e.g. [Knative][63]) and also by using plain python or other programming languages instead of paying for a service on Azure, AWS, or Google Cloud. Its management makes it easy through its modularity and abstraction, also with the use of Containers ([[Docker]] or [Rocket][65]), you can monitor all your applications in one place.

To get hands-on with Kubernetes you can install [Docker Desktop](https://www.docker.com/products/docker-desktop) with Kubernetes included. All of [my examples](http://code.sspaeti.com) are built on top of it and run on any cloud as well as locally. For a more sophisticated set-up in terms of Apache Spark, I suggest reading the blog post from [Data Mechanics](https://www.datamechanics.co/) about [Setting up, Managing & Monitoring Spark on Kubernetes](https://www.datamechanics.co/blog-post/setting-up-managing-monitoring-spark-on-kubernetes). If you are more of a video guy, [An introduction to Apache Spark on Kubernetes](https://youtu.be/qcvNZvFZIP4?t=31) contains the same content but adds still even on top of it.

As said above, if setting up Kubernetes is too hard, there are [[Stackable Kubernetes Cluster (Lego Blocks)]], where you can choose existing open-source tools to pick from.

---
References: [[YAML]]
Tags: #🗃/🌳 #publish