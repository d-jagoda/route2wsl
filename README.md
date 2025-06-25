# route2wsl

route2wsl is a windows service that configures Windows IPv4 routing table to forward specific IP traffic through WSL.

## What problem does route2wsl solve?

Being able to run a Linux Kernel presents us with an opptortunity to create sophisticated networks, as is the case when running a Kubernetes cluster. Taking Kubernetes again as an example, suppose that my Kubernetes cluster in WSL is configured with a service cluser IP address range of `10.152.183.0/24`, how could I access these services directly from Windows?

One obvious approach would be to create a network route from Windows all the way to Kubernetes by configuring the IP routing tables. This would enable us to seemlessly access all ClusterIP service types from Windows. Creating an IP route is easy, you specify the next hop in the network to forward network traffic meant for the IP address range you want to reach. The next hop in this case is the WSL virtual ethernet adaper which can be identified by the WSL gateway IP address. By setting up this route, we will be able to forward all the network traffic meant for the cluster IPs to WSL and have further routes setup in WSL in order to forward traffic to the Kubernetes cluster.


Taking cluster IP range of `10.152.183.0/24` as an example, you can add an IP route using the Windows `route` command:

```cmd
route add 10.152.183.0 mask 255.255.255.0 <wsl-gateway-ip>
```

However, adding a route this way has some complications:
 1. WSL gateway IP address and more importantly the WSL network adapter is non-persistent. Routes set on a non-persistent network adapter is lost when the adaper is removed and routes set on a dynamic IP address is useless when WSL stops using it. Both of these happen after rebooting Windows. 
 2. Adding a route requires administrative priviledges. With an dynamic IP address, adding a route cannot be done as a onetime administrative task.
 3. WSL VM and the network is only created when a WSL distro is started. In order to add the route, we need to ensure that this network has already been created.

`route2wsl` solves this problem with a onetime installation under administrative priviledges. Once installed, it will keep the route(s) updated with the WSL dynamic IP address.

![WSL-Windows!](/docs/images/wsl-windows.png)

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

- In Windows, install `route2wsl` for 10.2.0.3/24 (this step requires administrative priviledges)

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

Please refer to [Accessing Kubernetes](docs/Kubernetes.md)

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