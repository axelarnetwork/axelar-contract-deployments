# This docker image expects the binary "relayer" to be placed at its same directory level.

FROM debian:bookworm-slim

RUN apt-get update && apt install -y openssl ca-certificates

RUN useradd -r relayer
COPY ./relayer /app/relayer  
USER relayer
CMD [ "/app/relayer" ]