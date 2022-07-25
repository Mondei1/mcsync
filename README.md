# mcsync
**Synchronize your local minecraft server data with another server.**

**DISCLAIMER:** This is mostly a concept at this point. That might change in the future but for now, don't use it yet.

`mcsync` is not just a program, it rather consists of a custom client and server and a DNS, SSH & WireGuard server. It has been developed with Docker in mind. All the custom parts were built using **Rust**.

## Features
* Tunnel minecraft server to all participant *(using WireGuard)*
* Sync server files to remote server.
  * This allows all other players to start the server on there own, providing 100 % uptime if needed.
* Create as many servers as you please.
* Fake "minecraft server" that shows an MOTD if no one is hosting.
* Custom DNS resolution
  * Lets say you have an server called `survival1` all members can connect to `survival1.mc` as domain.

This works with all servers and versions *(Vanilla, Spigot, Forge, etc)*.

## Use case
I initially wrote this program to sync my Minecraft server to my less powerful vServer which isn't capable of running a full instance of Minecraft. But when hosting a Minecraft server on your own machine which is not turned on all the time (unlike a server), other people will not be able to play on that server.

This tool not only puts you and your friends into a own private network, it also syncs your server.

**If you rented a cheap VPS, have a decent PC and no access to port-forwarding** or **just want to have a super private Minecraft server** you should use this program.

# Install (not ready, just a concept)
First, you need to install Docker on your system. These steps may be different for your platform. Optionally, but recommended, you may want to install docker-compose.

## Automatic installation
Just execute the following command:
```sh
curl https://raw.githubusercontent.com/Mondei1/mcsync/main/installer.sh | sh
```
This will download and execute a small installer script that will setup everything for you.

## Manual installation
Depending on your hoster you may need to enable Docker and Tunnel support. Please educate yourself prior to the following steps.

Create a `docker-compose.yml` file inside some new folder you create and paste the following content into it:
```yml
# mcsync's Docker compose file.
# DO NOT CHANGE ANY SERVICE NAMES. MCSYNC WILL NOT BE ABLE TO FIND THOSE CONTAINERS IF YOU DO SO.

version: '3.3'
services:

  # We use wireguard-go, which is a userspace implementation of WireGuard, that does not depend on the kernel modules
  # since those are not always available (especially on vServers which are guests of a kernel).
  #
  # If your server does support the WireGuard kernel modules, feel free to use https://github.com/linuxserver/docker-wireguard instead.
  wireguard:
    image: masipcat/wireguard-go:latest
    networks:
      mcsync:
        ipv4_address: 192.168.11.3
    cap_add:
     - NET_ADMIN
    sysctls:
     - net.ipv4.ip_forward=1
    volumes:
     - /dev/net/tun:/dev/net/tun
     - ./config:/etc/wireguard
    environment:
     - PUID=1000
     - PGID=1000
     - WG_COLOR_MODE=always
     - LOG_LEVEL=debug
    ports:
     - 51820:51820/udp
    privileged: true
    restart: always

  # This service is used by rsync to move the minecraft server files around.
  ssh:
    image: lscr.io/linuxserver/openssh-server:latest
    networks:
      mcsync:
        ipv4_address: 192.168.11.4
    hostname: ssh
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Europe/Berlin
      - USER_NAME=mcsync
      - PUBLIC_KEY_DIR=/config/pubkeys
      - SUDO_ACCESS=false
    volumes:
      - ./ssh:/config
      - ./saves:/saves
    restart: unless-stopped

  dns:
    image: mvance/unbound
    networks:
      mcsync:
        ipv4_address: 192.168.11.5
    volumes:
      - ./dns:/opt/unbound/etc/unbound/


networks:
  mcsync:
    external: true
```
Now you'll need to create the `mcsync` network:
```sh
docker network create --subnet 192.168.11.0/24 mcsync
```

*more coming soon*

# Usage
So, you got mcsync up and running on your server and your client works as well.

## Neu user
Your server is working and now you want some players on your server.

### Client
You need to somehow get your WireGuard public key to the server owner. To do this, run the following command:

```sh
mcsync client_info > joe_doe.mcsc     # .MCSync Client = mcsc
```

`joe_doe.mcsc` (example)
```json
{
  "version": 1,
  "wireguard_pub": "VZBslaLy/AXCqk0rXq8Ip/+p7a/RyrG+H/WQ9ZeV8x8=",

  // ED25519 public key
  "ssh_pub": "AAAAC3NzaC1lZDI1NTE5AAAAIAzLg1ogXeY4VBch6uEcgNso26HowdmKpSNWwINSHQJd"
}
```
**Send this to the sever owner and wait for the server's information.**

*[Server ownser see down below]*

Now import the server's file:

```sh
mcsync import [SERVER_NAME] /path/to/server_info.mcss  # MCSync Server = mcss
mcsync import friends ~/Download/friends_server.mcss   # Example
```
`SERVER_NAME` can be any name you want. This name should make it easy to distinguish between multiple servers.

### Server
**Wait for a client to send you their client info.**


To import and accept the request, run the following:
```sh
# The name `mcsync-server-1` depends on how you named your container. This is the default.
docker exec -it mcsync-server-1 /bin/mcsync-server accept [CUSTOM_NAME] < /path/to/joe_doe.mcsc > server_info.mcss

# Example
docker exec -it mcsync-server-1 /bin/mcsync-server accept "Joe Doe" < /path/to/joe_doe.mcsc > server_info.mcss
```
`CUSTOM_NAME` can be any name you wish. Its sole purpose is to distinguish between multiple clients.

`server_info.mcss` (example):
```json
{
    "version": 1,
    "endpoint": "example.com:51820",
    "public_key": "VZBslaLy/AXCqk0rXq8Ip/+p7a/RyrG+H/WQ9ZeV8x8=",
    "user_subnet": "192.168.10.0/24",
    "tool_subnet": "192.168.11.0/24",
    "ipv4_address": "192.168.10.3",
    "dns": "192.168.11.5"
}
```

This will add the public key to your WireGuard configuration.

**WARNING: `mcsync` will automatically restart the WireGuard server. This means there is a small interruption of service.**

## Connect to server (client only)
```sh
mcsync connect [SERVER_NAME]
mcsync connect friends        # Example
```
This server is now your current server. All future operations (like adding a new Minecraft server) will affect this server.

## Disconnect from server (client only)
```sh
mcsync disconnect
```

## Status
### Client
```
$ mcsync status

Connected with `friends`
=========================

Available servers:
  * survival1 -> 4 / 20 players online - 1.19 Vanilla
  * creative  -> 0 / 8 players online  - 1.18.2 Vanilla
  - survival2
  - pvp

Clients:
  * Elliot Alderson (online)
  * Mr. Robot (online)
  - Morty Smith (last seen 2 hours ago)
  - Joe Doe (last seen 3 days ago)
  - Deon Wilson (last seen 2 weeks ago)
```

### Server
```
$ docker exec -it mcsync-server-1 /bin/mcsync-server status

Running mcsync v1.0-DEV
=========================

Available servers:
  * survival1 -> 4 / 8 players online - 1.19 Vanilla
  * creative  -> 0 / 8 players online  - 1.18.2 Vanilla
  - survival2
  - pvp

Clients:
  * Elliot Alderson (online)
  * Mr. Robot (online)
  - Morty Smith (last seen 2 hours ago)
  - Joe Doe (last seen 3 days ago)
  - Deon Wilson (last seen 2 weeks ago)
```

## Add new server (client only)
You need to connect to a server before you can use this command.
```sh
mcsync init [NAME]
mcsync init survival1   # Example
```
This will generate a new file called `.sync` inside your minecraft server containing the following information:
```json
{
  "server": "server_name",      // Name of server.
  "first_sync": 1656612770,     // Timestamp of first the sync. (no use but you can see your server getting older)
  "last_sync": 1656643855,      // Timestamp of last sync of your local copy.
  "version": 1              // Version of mcsync that performed the last sync.
}
```
After executing that command, the entirety of this folder will be synced to your remote.

# Backgrounds
## Network structure
This configuration consists of two subnets:
* `192.168.10.0/24` is reserved for all participants.
* `192.168.11.0/24` is reserved for all tools.

If those subnets are colliding with any of your subnets, change them. Just make sure that the second subnet needs to have at least 4 hosts.

Iptables will route traffic from the second to the first subnet.

## Backstory
You may remember the tool [Hamachi](https://vpn.net/). Back in the days, we used to use this program. We didn't knew how it worked, it was some kind of magic, but it allowed us to play Minecraft together. But this was years ago and now I know we achived this with using our own VPN. Yet, there was just one person who had the server. So when we wanted to play we had to wait for the hoster to be ready.

This tool's aim is to elimate this drawback. Unlike Hamachi, it isn't limited to just 5 users. *(technically there is a limit of 254)*

## Why use `wireguard-go`?
WireGuard is directly integrated into the Linux kernel or available as a kernel module, which is generally the way to go. Unfortunately, these modules are not available on most cheap vServers, which is why the official WireGuard server cannot be started. vServers run as a guest on a massive host system and use a shared kernel, like Docker containers. Guest systems are not able to modify the shared kernel. For this reason, you cannot install the required module even if you have root access to the guest.

`wireguard-go` is another official implementation of the WireGuard server which is slower than the kernel modules but it works everywhere. I plan to use the `wireguard-rs` implementation as soon as it's available on Windows.

## Why no OpenVPN?
OpenVPN is a ...
* **mess**: It would require a lot more effort to automatically setup the server and to keep an eye over the configuration.
* **slow**: OpenVPN is way slower than WireGuard. Both in terms of bandwitdh and latency. Take a look [here](https://www.vpnranks.com/blog/wireguard-vs-openvpn/).
* **bloated**: It supports far more ciphers than you actually need, which increases complexity and image size.

I hope that covers it up. Yet, if there is the wish to implement OpenVPN too, then open an issue.

# Future plans
While this project is still growing and at its beginning there are some things I'll like to add in the future:

* Central server which is hosted by me.
  * Easier to join other peoples private networks
  * Cheaper sync of your Minecraft worlds (E2E encrypted, 50 MB free maybe)
  * Dashboard of basic activity
* Client with GUI (using IMGui)

*more coming soon*