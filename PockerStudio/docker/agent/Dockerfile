FROM python:3.11-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl git \
    && rm -rf /var/lib/apt/lists/*

# Clone hermes-agent and checkout latest compatible version
# Set HERMES_AGENT_VERSION env var to pin to a specific version (e.g., 0.18.0)
ARG HERMES_AGENT_VERSION=main
RUN git clone https://github.com/outsourc-e/hermes-agent.git /app

WORKDIR /app

RUN if [ "$HERMES_AGENT_VERSION" != "main" ]; then \
      git fetch origin && git checkout "v${HERMES_AGENT_VERSION}" || git checkout "${HERMES_AGENT_VERSION}"; \
    fi

RUN pip install --no-cache-dir -e .

EXPOSE 8642

CMD ["hermes", "gateway", "run"]
