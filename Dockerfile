FROM rust:latest

COPY . /app

WORKDIR /app

RUN cargo build --release

EXPOSE 10000

CMD ["./target/release/url-shortener"]