FROM rust:bullseye
WORKDIR app
COPY . .

RUN cargo build --release --bin schema-engine --target x86_64-unknown-linux-gnu