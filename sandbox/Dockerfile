# Sandbox image for the Pi coding agent. Built by `sandbox-build` (devenv.nix),
# which passes RUST_VERSION from the host devenv so both toolchains match.
ARG RUST_VERSION=1.97.1
FROM node:24-bookworm-slim AS node

FROM rust:${RUST_VERSION}-slim-bookworm

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
     bash ca-certificates curl fd-find git ripgrep unzip \
  && ln -s /usr/bin/fdfind /usr/local/bin/fd \
  && rm -rf /var/lib/apt/lists/*

# The slim image installs rustup's minimal profile.
RUN rustup component add clippy rustfmt

# Node runtime for Pi, copied from the official image.
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm

ARG PI_VERSION=1.5.0
RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent@${PI_VERSION}

# The bind-mounted repo is owned by the host user, not root.
RUN git config --system --add safe.directory /workspace

COPY models.json settings.json /root/.pi/agent/

WORKDIR /workspace
ENTRYPOINT ["pi"]
