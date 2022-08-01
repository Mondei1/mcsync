# Server
This program runs inside your Docker environment.

## Tasks
* Manages custom DNS
* Keeps track of running minecraft instances
* Emulates fake minecraft server to show placeholder MOTD

# Build
The server is meant to be running inside a Docker environment.

To build the server executable you'll need to have Docker installed. Then run:
```sh
# CWD - /path/to/repo/server/
docker build -t YOUR_NAME/mcsync-server .
```

This produce two images. One is about 3 GB large, the other one just ~13 MB. Delete the larger one since it only contains the build environment.

You can now use this image by specifying its image id or name in your `docker-compose.yml`.