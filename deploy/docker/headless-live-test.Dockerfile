ARG BASE_IMAGE=elixir:1.19
ARG RUST_IMAGE=rust:1.88.0

FROM ${RUST_IMAGE} AS rust-toolchain

FROM ${BASE_IMAGE}

ARG HTTP_PROXY
ARG HTTPS_PROXY
ARG NO_PROXY
ARG http_proxy
ARG https_proxy
ARG no_proxy

ENV DEBIAN_FRONTEND=noninteractive
ENV MIX_ENV=test
ENV CI=1
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV MIX_HOME=/root/.mix
ENV HEX_HOME=/root/.hex
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV HTTP_PROXY=${HTTP_PROXY}
ENV HTTPS_PROXY=${HTTPS_PROXY}
ENV NO_PROXY=${NO_PROXY}
ENV http_proxy=${http_proxy}
ENV https_proxy=${https_proxy}
ENV no_proxy=${no_proxy}
ENV PATH=/usr/local/cargo/bin:${PATH}

COPY --from=rust-toolchain /usr/local/cargo /usr/local/cargo
COPY --from=rust-toolchain /usr/local/rustup /usr/local/rustup

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    libssl-dev \
    pkg-config \
    python3 \
    sqlite3 \
    xz-utils \
  && rm -rf /var/lib/apt/lists/*

RUN cargo --version \
  && rustc --version

WORKDIR /workspace

COPY apps/web apps/web
COPY workers/rust workers/rust

RUN rm -rf \
    /workspace/apps/web/_build \
    /workspace/apps/web/deps \
    /workspace/workers/rust/target \
  && mkdir -p \
    /workspace/apps/web/_build \
    /workspace/apps/web/deps \
    /workspace/tmp/data \
    /root/.mix \
    /root/.hex

RUN mix local.hex --force \
  && mix local.rebar --force

RUN cd apps/web \
  && mix deps.get

RUN cd workers/rust \
  && cargo fetch

CMD ["bash", "-lc", "cd /workspace/workers/rust && cargo test -p kyuubiki-cli --test headless_live"]
