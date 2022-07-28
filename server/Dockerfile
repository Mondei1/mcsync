FROM rust:1.62.1-alpine3.15 as build

WORKDIR /usr/src/app

# Copy over all necessary files to build mcsync-server.
COPY src/ ./src
COPY build.rs ./
COPY Cargo.toml ./
COPY Cargo.lock ./

RUN apk update
RUN apk add --no-cache libpq curl musl-dev openssl-dev
RUN rustup update nightly;
RUN rustup default nightly;

# Compile the project in release mode.
RUN cargo build -r

FROM scratch
EXPOSE 8080
COPY --from=build /usr/src/app/target/release/mcsync-server /bin/mcsync-server
ENTRYPOINT [ "/bin/mcsync-server" ]