podman-claude-build:
    podman build -t podman-proto-dev . -f Dockerfile.claude

podman-claude-init:
    podman volume inspect claude-config >/dev/null 2>&1 || podman volume create claude-config
    podman volume inspect claude-dotconfig >/dev/null 2>&1 || podman volume create claude-dotconfig

podman-claude: podman-claude-init
    podman run -it --rm \
        -e CARGO_REGISTRIES_BUF_TOKEN \
        -e SSH_AUTH_SOCK=/ssh-agent \
        -v $(pwd):/root/project:Z \
        -v claude-config:/root/.claude:z \
        -v claude-dotconfig:/root/.config:z \
        -v $SSH_AUTH_SOCK:/ssh-agent:Z \
        podman-proto-dev
