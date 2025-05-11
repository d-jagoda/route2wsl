# route2wsl

route2wsl is a windows service that configures Windows IPv4 routing table to forward specific IP traffic through WSL.

## 🚀 Use cases

### WSL static ip

- Set a static ip on WSL (for example: 10.1.0.3)

```bash
ip link add br01 type bridge
ip addr add 10.1.0.3/16 dev br01
ip link set br01 up
```

If you want this persisted across reboots, then include the commands in /etc/wsl.conf

```ini
[boot]
command = ip link add br01 type bridge & ip addr add 10.1.0.3/16 dev br01 & ip link set br01 up
```

- In Windows, install route2wsl for the 10.1.0.0/16 ip address range

```cmd
D:\temp\route2wsl-x86_64>route2wsl install -r 10.1.0.0/16
Service installed!
Starting service
Service started
```

- Test it out

```cmd
D:\temp\route2wsl-x86_64>ping 10.1.0.3

Pinging 10.1.0.3 with 32 bytes of data:
Reply from 10.1.0.3: bytes=32 time<1ms TTL=64
Reply from 10.1.0.3: bytes=32 time=1ms TTL=64
```

## 🧩 How It Works

route2wsl is a very simple tool that detects the Hyper-V Virtual Ethernet Adapter used by WSL, and creates rules to route network traffic to that network adapter.

It does this with the following steps:

- Scans for the WSL VM every 30 second
- Resolve the network interface used by WSL
- Resolves the IP address of the network interface - does this every 30 seconds
- Adds rules to the routing table if the network interface has changed since last time it was configured.
