FROM gitpod/workspace-full-vnc

# Install dependencies
RUN apt-get update \
    && apt-get install -y librust-gdk-sys-dev 