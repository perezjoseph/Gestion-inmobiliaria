FROM myoung34/github-runner:latest

USER root
RUN apt-get update -qq && apt-get install -y -qq unzip && rm -rf /var/lib/apt/lists/*
USER runner
