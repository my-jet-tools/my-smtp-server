# The image carries its own MTA: kumod (KumoMTA, written in Rust) delivers the mail
# straight to the mail server of the recipient. The entrypoint is this service - it
# compiles the kumod configuration out of the settings and supervises the process.
FROM ghcr.io/kumocorp/kumomta:latest

# kumod itself drops privileges to the kumod user, but the entrypoint has to be able to
# write the configuration and to chown the spool.
USER root
WORKDIR /
# The settings model is read from ~/.my-smtp-sender - make sure the home directory of the
# entrypoint is the one the settings file is mounted into.
ENV HOME=/root

COPY ./target/release/my-smtp-sender ./target/release/my-smtp-sender

ENTRYPOINT ["./target/release/my-smtp-sender"]
