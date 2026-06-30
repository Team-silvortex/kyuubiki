FROM ubuntu:24.04

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      ca-certificates \
      openssh-server \
      sudo \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash kyuubiki-fixture \
    && mkdir -p /run/sshd /tmp/kyuubiki-fixture /home/kyuubiki-fixture/.ssh \
    && chown -R kyuubiki-fixture:kyuubiki-fixture /home/kyuubiki-fixture/.ssh \
    && chown -R kyuubiki-fixture:kyuubiki-fixture /tmp/kyuubiki-fixture

COPY entrypoint.sh /usr/local/bin/kyuubiki-ssh-fixture-entrypoint
RUN chmod +x /usr/local/bin/kyuubiki-ssh-fixture-entrypoint

EXPOSE 2222

ENTRYPOINT ["/usr/local/bin/kyuubiki-ssh-fixture-entrypoint"]
