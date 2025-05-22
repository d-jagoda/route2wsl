# route2wsl

route2wsl is a windows service that configures Windows IPv4 routing table to forward specific IP traffic through WSL.

## 🚀 Use cases

### WSL static ip

- Set a static ip on WSL (for example: 10.2.0.3)

```bash
ip link add br01 type bridge
ip addr add 10.2.0.3/16 dev br01
ip link set br01 up
```

If you want this persisted across reboots, then include the commands in `/etc/wsl.conf`

```ini
[boot]
command = ip link add br01 type bridge & ip addr add 10.2.0.3/16 dev br01 & ip link set br01 up
```

- In Windows, install `route2wsl` for 10.2.0.3/24

```cmd
D:\temp\route2wsl-x86_64>route2wsl install -r 10.2.0.3/24
Service installed!
Starting service
Service started
```

- Test it out

```cmd
D:\temp\route2wsl-x86_64>ping 10.2.0.3

Pinging 10.2.0.3 with 32 bytes of data:
Reply from 10.2.0.3: bytes=32 time<1ms TTL=64
Reply from 10.2.0.3: bytes=32 time=1ms TTL=64
```

### Accessing MicroK8s on WSL2 from Windows

The following setup allows you to access kubernetes pod IPs, service IPs and DNS names from Windows. Note that the first time you access a MicroK8s network resource from Windows, it will take a few seconds until the route is completely resolved. Let me know if you find a way to improve that.

1 . Find the pod CIDR and service CIDR (defaults are 10.1.0.0/16 and 10.152.183.0/24 respectively)

2 . Create a script called `/etc/setup_network.sh` to setup the network in WSL and add the following content. Ensure that the service CIDR in the script matches your MicroK8s configuration.

```sh
# Add a network device for our static IP
ip link add br01 type bridge
# Set the static IP
ip addr add 10.2.0.3/16 dev br01
ip link set br01 up
# Add a route for the service-CIDR to the network device we added. Calico will pick this up. We don't need to do this for the pod-CIDR because Calico would do it automatically.
ip route add 10.152.183.0/24 dev br01
# Makes the Linux system behave like a router at the Ethernet (Layer 2) level. Without this, WSL (Hyper-V firewall) in Windows will not route to WSL
echo 1 > /proc/sys/net/ipv4/conf/all/proxy_arp
```
3 . Call the script when WSL starts by including it in `/etc/wsl.conf` and restart WSL to ensure the script gets run(`WSL --shutdown` followed by WSL from a Windows command shell)

```ini
[boot]
command = sh /etc/setup_network.sh
```

4 . In Windows, install `route2wsl` for the pod CIDR, service CIDR, and 10.2.0.0/16 ip address ranges to make it possible to reach kubernetes pods, services and lastly to be able to address WSL with the static IP of `10.2.0.3`

```cmd
D:\temp\route2wsl-x86_64>route2wsl install -r 10.1.0.0/16 -r 10.152.183.0/24 -r 10.2.0.3/24
Service installed!
Starting service
Service started
```

5 . Test out your routes by pinging a pod IP from windows
```cmd
ping 10.1.191.154
```

6 . If you have DNS setup with Microk8s, you can add a rule on Windows using powershell to use CoreDNS from Windows for `.cluster.local` name suffix
```powershell
Add-DnsClientNrptRule -Namespace ".cluster.local" -NameServers "10.152.183.10" -DisplayName "Win Kube Access Rule"
```

7 . Test out the DNS setup from Windows
```cmd
ping kubernetes.default.svc.cluster.local

Pinging kubernetes.default.svc.cluster.local [10.152.183.1] with 32 bytes of data:
```

8 . View kube config by running `microk8s config` in WSL and carefully copy the cluster/context and user sections to the one in Windows at `%userprofile%/.kube/config`. Make sure that you use the WSL static ip `10.2.0.3:16443` for server instead of the one shown by `microk8s config` command

9 . Test out kubectl on Windows against microk8s running in WSL. This will take a few seconds the first time you run it.
```cmd
kubectl get all --all-namespaces -o wide --context microk8s

NAMESPACE        NAME                                             READY   STATUS    RESTARTS      AGE   IP              NODE       NOMINATED NODE   READINESS GATES
default          pod/ubuntu-dep-7678fc7dcf-5m8v8                  1/1     Running   2 (23h ago)   47h   10.1.191.141    djagoda-1   <none>           <none>
kube-system      pod/calico-kube-controllers-5947598c79-wt4fw     1/1     Running   2 (23h ago)   47h   10.1.191.146    djagoda-1   <none>           <none>
kube-system      pod/calico-node-9h4vc                            1/1     Running   2 (23h ago)   47h   172.23.126.80   djagoda-1   <none>           <none>
kube-system      pod/coredns-79b94494c7-7t2zs                     1/1     Running   2 (23h ago)   47h   10.1.191.142    djagoda-1   <none>           <none>
kube-system      pod/dashboard-metrics-scraper-5bd45c9dd6-rwnm2   1/1     Running   2 (23h ago)   47h   10.1.191.145    djagoda-1   <none>           <none>
kube-system      pod/kubernetes-dashboard-57bc5f89fb-m66l8        1/1     Running   2 (23h ago)   47h   10.1.191.143    djagoda-1   <none>           <none>
kube-system      pod/metrics-server-7dbd8b5cc9-pfzbk              1/1     Running   2 (23h ago)   47h   10.1.191.144    djagoda-1   <none>           <none>
```

## 🧩 How It Works

route2wsl is a very simple tool that detects the Hyper-V Virtual Ethernet Adapter used by WSL, and creates rules to route network traffic to that network adapter.

It does this with the following steps:

- Scans for the WSL VM every 30 second
- Resolve the network interface used by WSL
- Resolves the IP address of the network interface - does this every 30 seconds
- Adds rules to the routing table if the network interface has changed since last time it was configured.

## 🛠️ Building This Rust Project

### Prerequisites

Before building, ensure the following tools are installed:

1. **Rust Toolchain**

2. (Optional) **Git**

---

### Build Instructions

Once the environment is ready, clone and build the project:

```
git clone https://github.com/d-jagoda/route2wsl.git
cd route2wsl
cargo build --release
```