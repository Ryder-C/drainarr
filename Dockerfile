FROM rust:1.96-slim

WORKDIR /config
COPY Cargo.toml Cargo.lock /config/
COPY src /config/src

RUN cargo install --path .

CMD ["drainarr"]
