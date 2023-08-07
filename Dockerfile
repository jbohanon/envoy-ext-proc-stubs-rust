FROM ubuntu:jammy
COPY target/release/server /usr/local/bin/server
EXPOSE 9090
ENTRYPOINT ["/usr/local/bin/server"]
