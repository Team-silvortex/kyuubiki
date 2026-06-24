ARG BASE_IMAGE=elixir:1.19
FROM ${BASE_IMAGE}

ARG HTTP_PROXY
ARG HTTPS_PROXY
ARG NO_PROXY
ARG http_proxy
ARG https_proxy
ARG no_proxy
ARG NODE_VERSION=20.19.2
ARG RUST_TOOLCHAIN=1.88.0

ENV DEBIAN_FRONTEND=noninteractive
ENV MIX_ENV=test
ENV CI=1
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV MIX_HOME=/root/.mix
ENV HEX_HOME=/root/.hex
ENV HTTP_PROXY=${HTTP_PROXY}
ENV HTTPS_PROXY=${HTTPS_PROXY}
ENV NO_PROXY=${NO_PROXY}
ENV http_proxy=${http_proxy}
ENV https_proxy=${https_proxy}
ENV no_proxy=${no_proxy}
ENV PATH=/root/.cargo/bin:/usr/local/node/bin:${PATH}

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    libssl-dev \
    make \
    pkg-config \
    python3 \
    python3-pip \
    time \
    xz-utils \
  && rm -rf /var/lib/apt/lists/*

RUN arch="$(dpkg --print-architecture)" \
  && case "$arch" in \
    amd64) node_arch="x64" ;; \
    arm64) node_arch="arm64" ;; \
    *) echo "unsupported architecture: $arch" >&2; exit 1 ;; \
  esac \
  && curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${node_arch}.tar.xz" -o /tmp/node.tar.xz \
  && mkdir -p /usr/local/node \
  && tar -xJf /tmp/node.tar.xz -C /usr/local/node --strip-components=1 \
  && ln -sf /usr/local/node/bin/node /usr/local/bin/node \
  && ln -sf /usr/local/node/bin/npm /usr/local/bin/npm \
  && ln -sf /usr/local/node/bin/npx /usr/local/bin/npx \
  && rm -f /tmp/node.tar.xz \
  && node --version \
  && npm --version

RUN curl -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh \
  && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
  && rm -f /tmp/rustup-init.sh \
  && cargo --version \
  && rustc --version

WORKDIR /workspace

COPY . .

RUN rm -rf \
    /workspace/apps/web/_build \
    /workspace/apps/web/deps \
    /workspace/apps/frontend/node_modules \
    /workspace/workers/rust/target \
  && mkdir -p \
    /workspace/apps/web/_build \
    /workspace/apps/web/deps \
    /root/.mix \
    /root/.hex

RUN mix local.hex --force \
  && mix local.rebar --force

RUN cd apps/web \
  && mix deps.get

RUN cd apps/frontend \
  && npm ci

RUN cd workers/rust \
  && cargo fetch

CMD ["bash"]
