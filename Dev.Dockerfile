FROM rust:1.66 as builder
USER root
WORKDIR /build
RUN apt update -y && apt install -y protobuf-compiler
CMD ["cargo", "build"]

# docker run -v $(pwd):/build $IMAGE_NAME