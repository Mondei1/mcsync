# mcsync
**Synchronize your local minecraft server data with another server.**

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

# Install
First, you need to install Docker on your system. These steps may be different for your platform. Optionally, but recommended, you may want to install docker-compose.

Create a `docker-compose.yml` file inside some new folder you create and paste the following content into it:
```yml
# mcsync's Docker compose file.

version: '3.3'
services:

  # We use wireguard-go, which is a userspace implementation of WireGuard, that does not depend on the kernel modules
  # since those are not always available (especially on vServers which are guests of a kernel).
  #
  # If your server does support the WireGuard kernel modules, feel free to use https://github.com/linuxserver/docker-wireguard instead.
  wireguard:
    image: masipcat/wireguard-go:latest
    networks:
      - mcsync
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

  web:
    image: nginx
    networks:
      mcsync:
        ipv4_address: 192.168.255.2


networks:
  mcsync:
    external: true
```
*more coming soon*

## Backstory
You may remember the tool [Hamachi](https://vpn.net/). Back in the days, we used to use this program. We didn't knew how it worked, it was some kind of magic, but it allowed us to play Minecraft together. But this was years ago and now I know we achived this with using our own VPN. Yet, there was just one person who had the server. So when we wanted to play we had to wait for the hoster to be ready.

This tool's aim is to elimate this drawback. Unlike Hamachi, it isn't limited to just 5 users.

## Why use `wireguard-go`?
WireGuard is directly built into the Linux kernel or available as kernel module which is generally the way to go. Unfortunately, these modules aren't available on most cheap vServers, therefore the official WireGuard server will fail to start. vServers run as guest on a massive host system and they use a shared kernel, like Docker containers. Guest systems are unable to modify the shared kernel. Because of this you cannot install the required module even if you have root access to the guest.

`wireguard-go` is another official implementation of the WireGuard server which is slower than the kernel modules but it works everywhere. I plan to use the `wireguard-rs` implementation as soon as it's available on Windows.