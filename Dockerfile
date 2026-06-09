# build stage use rust as base image
FROM rust:alpine AS builder
COPY ../ .
RUN apk add --no-cache musl-dev
RUN cargo build --release
# runtime stage use alpine as base image
FROM alpine:latest
# copy compiled file from build stage
COPY --from=builder ./target/release/basjoofan /usr/bin
