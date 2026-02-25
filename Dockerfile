FROM alpine:3.19

ARG TARGET

# Security: Set timezone to UTC and avoid interactive prompts
ENV TZ=UTC
ENV ALPINE_HOME=/home/redlib

RUN apk add --no-cache curl tzdata

# Create non-root user for running redlib
RUN mkdir -p "$ALPINE_HOME" && \
    adduser -D -h "$ALPINE_HOME" redlib && \
    chown -R redlib:redlib "$ALPINE_HOME"

RUN curl -L "https://github.com/redlib-org/redlib/releases/latest/download/redlib-${TARGET}.tar.gz" | \
    tar xz -C /usr/local/bin/

# Switch to non-root user
USER redlib

# Tell Docker to expose port 8080
EXPOSE 8080

# Run a healthcheck every minute to make sure redlib is functional
HEALTHCHECK --interval=1m --timeout=3s --start-period=5s --retries=3 CMD wget --spider -q http://localhost:8080/settings || exit 1

CMD ["redlib"]

