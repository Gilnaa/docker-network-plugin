# Simple Docker network plugin
Uses Rust+Axum.

# Building from source
This builds the plugin and loads it into the local daemon
```
make all
```

# References

Docker Plugin reference:
 - https://docs.docker.com/engine/extend/config/
 - https://docs.docker.com/engine/extend/plugin_api/

This libnetwork doc is mostly on-point but lack mention of AllocateNetwork/FreeNetwork which
are neceassry to implement for Docker to work.
 - https://github.com/moby/libnetwork/blob/master/docs/remote.md

AllocateNetwork is mentioned here and in the source code.
 - https://pkg.go.dev/github.com/projecteru2/minions/driver#NetworkDriver
 - https://pkg.go.dev/github.com/docker/go-plugins-helpers/network
