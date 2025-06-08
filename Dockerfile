# shamelessly borrow from https://mitchellh.com/writing/nix-with-dockerfiles

FROM nixos/nix:latest AS builder

COPY . /tmp/build
WORKDIR /tmp/build

RUN nix \
  --extra-experimental-features "nix-command flakes" \
  build

RUN mkdir /tmp/nix-store-closure
RUN cp -R $(nix-store -qR result/) /tmp/nix-store-closure

FROM scratch

WORKDIR /app

COPY --from=builder /tmp/nix-store-closure /nix/store
COPY --from=builder /tmp/build/result/bin/zero2prod zero2prod
COPY --from=builder /tmp/build/configuration configuration
ENV APP_ENVIRONMENT=production
ENTRYPOINT ["./zero2prod"]
