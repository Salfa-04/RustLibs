FROM alpine as builder
COPY . /workspace/source
RUN apk update \
    && apk upgrade \
    && apk add cargo \
    && cargo install --path /workspace/source --root /workspace

FROM alpine as runner
WORKDIR /workspace
RUN apk update --no-cache \
    && apk upgrade --no-cache \
    && apk add libgcc --no-cache
COPY --from=builder /workspace/bin/* .
# EXPOSE 4998/tcp
CMD ["/workspace/dockerfile"] ### [请修改此处]

MAINTAINER Salfa <salfa@foxmail.com>
LABEL name="这是镜像名称"\ ### [请修改此处]
    version="0.1.1"\ ### [请修改此处]
    description="这是镜像描述"\ ### [请修改此处]
    by="Salfa"
