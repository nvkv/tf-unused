FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends curl && \
    rm -rf /var/cache/apt/archives /var/lib/apt/lists/*
RUN curl -OL "https://github.com/mijdavis2/tf-unused/releases/download/0.3.0/tf-unused"
RUN chmod +x tf-unused && mv tf-unused /usr/local/bin/tf-unused
