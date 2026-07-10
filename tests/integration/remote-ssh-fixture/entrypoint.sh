#!/usr/bin/env sh
set -eu

AUTHORIZED_KEYS="/home/kyuubiki-fixture/.ssh/authorized_keys"

if [ ! -s "${AUTHORIZED_KEYS}" ]; then
  echo "missing ${AUTHORIZED_KEYS}; generate runtime/client_key.pub first" >&2
  exit 64
fi

chown kyuubiki-fixture:kyuubiki-fixture /home/kyuubiki-fixture/.ssh
chmod 700 /home/kyuubiki-fixture/.ssh

exec /usr/sbin/sshd -D -e -p 2222
