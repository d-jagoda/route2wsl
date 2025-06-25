# Accessing Kubernetes

There are instances where we want access networks inside of WSL from Windows. Once such scenario is being able to access the POD network, Services, Ingresses or it's successor the Gateway API of a Kubernetes cluster running in WSL.

## Accessing a Microk8s cluster running in WSL

### Prerequisites

* Ensure you have Microk8s installed in WSL. Installing it is easy, just follow the instruction in https://microk8s.io/docs/getting-started to get started.

    ⚠️ **Important:** If your WSL hostname contain capital letters or underscores rename it in `/etc/wsl.conf` and restart WSL for it to take effect  or Microk8s may fail to start.

    ```ini
    [network]
    hostname = <overriden host name>
    ```

* Ensure you have `kubectl` or an alternatively tool that can interact with Kubernetes installed in Windows.

* Download `route2wsl` binary if you have not done so already.

### Accessing the API server from Windows

In order to access Microk8s API server, we need to do the following:
 
* Set a static IP for WSL.

   ```sh
    # Add a network device for our static IP
    ip link add br01 type bridge
    # Set the static IP
    ip addr add 10.2.0.3/16 dev br01
    ip link set br01 up   
   ```
   
   To make this change persistent, add it to a script (for example `/etc/setup_network.sh`) and configure the script to be called when the WSL distro starts by modifying `/etc/wsl.conf`

    ```ini
    [boot]
    command = sh /etc/setup_network.sh
    ```

* Create a persistent route in Windows to access the static IP by installing `route2wsl`. Note that while this step requires administrative priviledges, this route will now be persisted across reboots.

    ```cmd
    route2wsl install -r 10.2.0.3/32
    ```

* Next we have to add the cluster and a user to a kube config file accessible from Windows. The default kube config file is `%USERPROFILE%/.kube/config`

    View kube config by running `microk8s config`

    ```sh
    djagoda@my-wsl:~$ microk8s config
    apiVersion: v1
    clusters:
    - cluster:
        certificate-authority-data: <BASE64 encoded data>
        server: https://172.23.126.80:16443
    name: microk8s-cluster
    contexts:
    - context:
        cluster: microk8s-cluster
        user: admin
    name: microk8s
    current-context: microk8s
    kind: Config
    preferences: {}
    users:
    - name: admin
    user:
        client-certificate-data: <BASE64 encoded data>
        client-key-data: <BASE64 encoded data>
    ```

   ✅ **Required:** since the IP of the `server` is ephemeral, we must replace it with our static ip address to `server:https://10.2.0.3:16443`

   We can now add the cluster information to a kube config file accessible from Windows:

   1. The best option would be to carefully copy the cluster, user and context into `%USERPROFILE%/.kube/config` file.
   2. If you want to use new kube config file you can copy the config to a file accessible from Windows.

        ```sh
        microk8s config | sed -E 's/([0-9]{1,3}\.){3}[0-9]{1,3}/10.2.0.3/g' > file_accessible_from_windows
        ```

        ✅ **Note:** when a separate kube config file is used you must either use the `--kubeconfig` flag in `kubectl` or include the file in the `KUBECONFIG` environment variable along with other kube config files.

* Lets test this out from Windows.

    1. If you added cluster information to `%USERPROFILE%/.kube/config`

        ```sh
        kubectl get all --context microk8s -o wide
        ```

    2. If you added cluster information to a separate file
        ```
        kubectl get all --kubeconfig file_accessible_from_windows -o wide
        ```

### Accessing Pod network and Cluster IP type Services from Windows

* Find the pod CIDR and service CIDR (defaults are 10.1.0.0/16 and 10.152.183.0/24 respectively)

* Create a persistent route in Windows to access the Pod network and cluster IPs using `route2wsl`. This step requires administrative priviledges.

    ```cmd
    route2wsl add-route -r 10.1.0.0/16 -r 10.152.183.0/24
    ```

* Create a persistent route in WSL to route Cluster IP traffic.

    ℹ️ Microk8s uses Calico CNI for networking which makes the Pod network and services accessible from WSL directly. Calico creates a virtual network device for every pod which makes routing to pods straightforward without requiring additional routing rules. However cluster IPs are not assigned to a physical or virtual network device like a pod IP would be. Instead, they are implemented using iptables/ipvs rules or eBPF-based routing (depending on the CNI plugin and kube-proxy mode). We therefore need to add a routing rule to let WSL knows how to route traffic for the service CIDR. This also poses a challenging question, where exactly do you route this traffic to considering there is no network device associated with the IP addresses? It turns out you can write a routing rule to target any of the non Calico network devices and Calico will scan these routing rules route them correcty. We will therefore pick the same virtual network device that we created for the static IP and add a rule to our boot script.

    It will now look like:

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

* Test out the routes by pinging a pod IP and telnetting or curling a cluster IP.

### Accesing Kubernetes DNS (CoreDNS) from Windows

Being able to access Cluster IPs is great but these IPs are dynamically assigned(static assignment is also possible). You would normally access services in kubernetes using it's DNS name in the format `service-name.namespace.svc.cluster.local`. Ideally we want to do the same from Windows. This can be done by using Kubernetes DNS server from Windows just for the `cluster.local` zone.

* Enable DNS in Microk8s

  ```sh
  microk8s enable dns
  ```

* The DNS service Cluster IP is static and default for Microk8s is `10.152.183.10`. You can verify this by running `microk8s kubectl get service/kube-dns -n kube-system` in WSL. We can now add a rule to Windows to use Kubernetes DNS for `cluster.local` zone. This step requires executing the following powershell command under administrative priviledges.

  ```powershell
  Add-DnsClientNrptRule -Namespace ".cluster.local" -NameServers "10.152.183.10" -DisplayName "Kube DNS Access Rule"
  ```

### Accessing LoadBalancer IPs from Windows

When you want to expose a service externally using an external load balancer, you can create a Kubernetes `LoadBalancer` Service. This how you make your ingress controller accessible externally from your cloud hosted Kubernetes cluster. Apart from an ingress controller, it is common for some of the popular helm charts to provide you with the option to expose services as a `LoadBalancer` as is the case with Bitnami Kafka and MySql helm charts. If you utilize this feature in Docker Desktop kubernetes, you can access a `LoadBalancer` service directly from Windows using the loopback IP 127.0.0.1. This is made possible in Docker Desktop by the VPN kit that comes with it. Despite being able to access cluster IP service from Windows, you may still want access load balancer services in Microk8s from Windows too. We can employ an actually external load balancer with Microk8s to acheive this. We will use `MetalLB` as the external load balancer.

* Enable `MetalLB` in Microk8s. Pick a distinct IP range as your external IP range. I have picked 192.168.2.0/24 in this example.

    ```sh
    djagoda@my-wsl:~$ sudo microk8s enable metallb
    Infer repository core for addon metallb
    Enabling MetalLB
    Enter each IP address range delimited by comma (e.g. '10.64.140.43-10.64.140.49,192.168.0.105-192.168.0.111'): 192.168.2.0/24
    ```
* Create a persistent route in Windows to access the Pod network and cluster IPs using `route2wsl`. This step requires administrative priviledges.

    ```cmd
    route2wsl add-route -r 192.168.2.0/24
    ```
* Create a persistent route in WSL to route load balancer IP traffic.

  We can add a routing rule to our boot script. This script will now look like:

    ```sh
    # Add a network device for our static IP
    ip link add br01 type bridge
    # Set the static IP
    ip addr add 10.2.0.3/16 dev br01
    ip link set br01 up

    # Add a route for the service-CIDR to the network device we added. Calico will pick this up. We don't need to do this for the pod-CIDR because Calico would do it automatically.
    ip route add 10.152.183.0/24 dev br01
    # Add a route for load balalncer IP range
    ip route add 192.168.2.0/24 dev br01
    # Makes the Linux system behave like a router at the Ethernet (Layer 2) level. Without this, WSL (Hyper-V firewall) in Windows will not route to WSL
    echo 1 > /proc/sys/net/ipv4/conf/all/proxy_arp 
    ```

### Accessing Ingresses with External DNS from Windows

So far we are able to access all Kubernetes network resources except for one which is actually meant to be accessed externally - the Ingress resources. Ingress DNS names are registered with an external DNS service and not with Kube DNS.

* Enable Ingress in Microk8s.

  ```sh
  PUBLISH_STATUS_ADDRESS=10.2.0.3 microk8s enable ingress
  ```

* Install an external DNS server. We will be using `Bind9` as our external DNS server running outside of Kubernetes in this example.

  1. Install Bind9

        ```sh
        sudo apt update -y && apt install bind9 bind9utils dnsutils -y
        ```
  2. Modify configuration to ensure Bind9 listens on an IPv4 addresses.

        ```sh
        sudo cat << EOF > /etc/bind/named.conf.options
        options {
            directory "/var/cache/bind";
            allow-query { any; };
            allow-recursion { none; };
            recursion no;
            dnssec-validation no;
            auth-nxdomain no;
            listen-on { any; };
            listen-on-v6 { none; };
        };
        EOF
        ```
  3. Create a zone file for the zone you want to use in your ingress. For example `local.somedomain.com`.

        ```sh
        cat << EOF > /etc/bind/db.local.somedomain.com
        \$TTL    60
        @       IN      SOA     ns.local.somedomain.com. root.local.somedomain.com. (
                                2         ; Serial
                                604800     ; Refresh
                                86400      ; Retry
                                2419200    ; Expire
                                60 )       ; Negative Cache TTL
                IN      NS      ns.local.somedomain.com.
        ns      IN      A       127.0.0.1
        EOF
        ```
        
  4. Create a TSIG key that can be used by Kubernetes external DNS operator when updating DNS records in Bind9. 

        ```sh
        # Generate TSIG key
        DNS_USER_KEY=$(tsig-keygen -a HMAC-SHA256 k8s-externaldns-key)

        # Add the TSIG key to bind9
        echo "$DNS_USER_KEY" > /etc/bind/named.conf.local

        cat << EOF >> /etc/bind/named.conf.local
        zone "local.somedomain.com" IN {
            type master;
            file "/etc/bind/db.local.somedomain.com";
            allow-update { key "k8s-externaldns-key"; };
        };
        EOF

        # Store TSIG secret in Kubernetes
        DNS_PWD=$(echo "$DNS_USER_KEY" | grep secret | awk '{print $2}' | awk -F \" '{print $2}')
        microk8s kubectl delete secret externaldns-tsi
        microk8s kubectl create secret generic externaldns-tsig --from-file=tsig-secret="$DNS_PWD"

        # Restart Bind9
        chown -R bind:bind /etc/bind
        named-checkconf
        systemctl restart bind9
        ```
  5. Deploy the external DNS operator in Microk8s pointing it to the local Bind9 instance

        ```yaml
        cat << EOF | microk8s kubectl apply -f -
        apiVersion: apps/v1
        kind: Deployment
        metadata:
        name: external-dns
        spec:
        replicas: 1
        selector:
            matchLabels:
            app: external-dns
        template:
            metadata:
            labels:
                app: external-dns
            spec:
            containers:
            - name: external-dns
                image: registry.k8s.io/external-dns/external-dns:v0.13.4
                args:
                - --source=ingress
                - --provider=rfc2136
                - --rfc2136-host=10.2.0.3
                - --rfc2136-port=53
                - --rfc2136-zone=local.somedomain.com
                - --rfc2136-tsig-secret=\$(RFC2136_TSIG_SECRET)
                - --rfc2136-tsig-secret-alg=hmac-sha256
                - --rfc2136-tsig-keyname=k8s-externaldns-key
                - --rfc2136-tsig-axfr
                - --policy=sync
                - --registry=txt
                - --txt-owner-id=microk8s
                env:
                - name: RFC2136_TSIG_SECRET
                valueFrom:
                    secretKeyRef:
                    name: externaldns-tsig
                    key: tsig-secret
        EOF  
        ```

  6. Add a rule to Windows to use bind9 DNS for `local.somedomain.com` zone.

        This step requires executing the following powershell command under administrative priviledges.

        ```powershell
        Add-DnsClientNrptRule -Namespace ".local.somedomain.com" -NameServers "10.2.0.3" -DisplayName "Bind9 DNS Access Rule"
        ```
  
  7. Lets test it out by setting up an ingress against the kubernetes API server.

        ```yaml
        cat << EOF | microk8s kubectl apply -f -
            apiVersion: networking.k8s.io/v1
            kind: Ingress
            metadata:
            name: test-ingress
            spec:
            ingressClassName: nginx
            rules:
            - host: kubernetes.local.somedomain.com
                http:
                paths:
                - path: /
                    pathType: Prefix
                    backend:
                    service:
                        name: kubernetes
                        port:
                        number: 443
            EOF		
        ```