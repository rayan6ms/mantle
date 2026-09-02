FROM localhost/mantle-phase15-sanitizer@sha256:fbedb6dffe09dbd160b2ac9e05fc5a7503ee56a81003c18e2f13fbf3e553c274

RUN dnf -y --setopt=install_weak_deps=False install \
      clang-22.1.8-4.fc44 \
      compiler-rt-22.1.8-4.fc44 \
    && dnf clean all
