FROM rust:latest AS builder
WORKDIR /usr/src/myapp
COPY . .

RUN cargo update && cargo build -r


FROM ubuntu:latest

COPY --from=builder /usr/src/myapp/target/release/ruast_qqbot /usr/local/bin/ruast_qqbot



RUN apt-get update && apt-get install -y tzdata \
	&& cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime \
	&& echo "Asia/Shanghai" > /etc/timezone

WORKDIR /app

VOLUME /app/config

ENTRYPOINT ["sh", "-c", "ruast_qqbot"]