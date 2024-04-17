FROM rust:latest

RUN apt update && apt install -y vim

COPY . /app

WORKDIR /app

RUN cargo build --release

EXPOSE 10000

CMD ["./target/release/url-shortener-rust"]